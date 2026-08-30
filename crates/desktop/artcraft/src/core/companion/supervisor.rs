use crate::core::companion::companion_launch_config::{resolve_launch_config, CompanionLaunchConfig};
use crate::core::state::companion::companion_restart_signal::CompanionRestartSignal;
use crate::core::state::companion::companion_state::{CompanionInfo, CompanionState, CompanionStatus};
use log::{error, info, warn};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;

/// Prefix `companion/backend/main.py` prints to stdout, before starting
/// uvicorn, so the parent can discover the ephemeral port it bound. Must
/// match the string literal in `companion/backend/main.py` exactly.
const PORT_MARKER: &str = "ARTCRAFT_STUDIO_COMPANION_PORT=";

/// Header the Companion requires on every request (SYSTEM_DESIGN.md §1's
/// shared-secret handshake). This constant must stay in sync with
/// `AUTH_HEADER` in `companion/backend/core/companion_auth_middleware.py` -
/// enforced Companion-side by `CompanionAuthMiddleware`. The token itself is
/// generated here and forwarded to the child process via the
/// `ARTCRAFT_STUDIO_COMPANION_AUTH_TOKEN` env var (never over the network).
pub const AUTH_HEADER_NAME: &str = "X-ArtCraftStudio-Companion-Auth";

const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const HEALTH_CHECK_MAX_ATTEMPTS: u32 = 30; // ~30s at 1s apart, generous for cold-start ONNX/model imports
const HEALTH_CHECK_RETRY_DELAY: Duration = Duration::from_secs(1);
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// After this many consecutive crashes (not counting the expected
/// exit-code-3 "please restart me" convention, which always resets the
/// counter), stop retrying and surface `FailedToStart` instead of looping
/// forever on a Companion that can't stay up.
const MAX_CONSECUTIVE_CRASHES: u32 = 5;

/// The exit code `companion/backend/main.py` uses to signal "I exited on
/// purpose because config changed, please restart me" - distinct from an
/// actual crash. Treated as a free restart (no backoff, no crash-count
/// increment).
const EXPECTED_RESTART_EXIT_CODE: i32 = 3;

const READY_EVENT: &str = "companion://ready";
const EXITED_EVENT: &str = "companion://exited";
const STATUS_CHANGED_EVENT: &str = "companion://status_changed";

/// Long-running supervisor task: spawn the Companion, discover its port,
/// health-check it, publish `CompanionInfo` + emit events, and restart on
/// crash with capped exponential backoff. Intended to be run via
/// `tauri::async_runtime::spawn(run_companion_supervisor(...))` once, at
/// startup (see `core::lifecycle::startup::tasks::spawn_companion_supervisor`).
///
/// Process-tree cleanup: this function relies on TWO layers, not one -
/// `tokio::process::Command::kill_on_drop(true)` for best-effort explicit
/// cleanup, AND `companion/backend/main.py`'s own `_watch_parent()` thread
/// (ported near-verbatim from Stimma) as the authoritative backstop, since
/// it detects its own parent's death via `getppid()` and self-terminates
/// regardless of how the Rust side exits - including a hard crash that never
/// runs any Drop impl. The Python multiprocessing ingestion worker does the
/// same one level down, watching the main Companion process. This two-layer
/// design means a full Windows Job Object implementation is NOT required
/// for correctness (SYSTEM_DESIGN.md originally called for one; simplified
/// here once the existing Python-side parent-watch chain was confirmed to
/// already cover the hard-crash case portably - see companion/README.md).
pub async fn run_companion_supervisor(app: AppHandle, state: CompanionState, restart_signal: CompanionRestartSignal) {
  let mut consecutive_crashes: u32 = 0;
  let mut backoff = INITIAL_BACKOFF;

  loop {
    let launch_config = match resolve_launch_config(&app) {
      Ok(cfg) => cfg,
      Err(reason) => {
        error!("Companion supervisor: cannot resolve launch config: {}", reason);
        publish_status(&app, &state, CompanionStatus::FailedToStart { reason });
        return; // Nothing to retry - this is a static config problem, not a transient crash.
      }
    };

    let auth_token = uuid_utils::uuid::generate_random_uuid();

    if let Err(err) = state.set(CompanionInfo {
      port: None,
      auth_token: auth_token.clone(),
      status: CompanionStatus::Starting,
    }) {
      error!("Companion supervisor: failed to publish Starting state: {:?}", err);
    }
    emit_status_changed(&app, &state);

    match spawn_and_supervise_once(&app, &state, &launch_config, &auth_token).await {
      RunOutcome::Ready(mut child) => {
        // spawn_and_supervise_once() already published Ready. Race the
        // child's natural exit against an explicit restart request so
        // companion_restart_command can interrupt a healthy, long-running
        // Companion without waiting for it to crash on its own.
        let (exit_code, was_requested_restart) = tokio::select! {
          code = wait_for_exit(&mut child) => (code, false),
          _ = restart_signal.wait_for_restart_request() => {
            info!("Restart requested; killing current Companion process");
            let _ = child.kill().await;
            let _ = child.wait().await;
            (None, true)
          }
        };
        if is_expected_restart(was_requested_restart, exit_code) {
          info!("Companion restarting (requested={}, exit_code={:?}); no backoff", was_requested_restart, exit_code);
          consecutive_crashes = 0;
          backoff = INITIAL_BACKOFF;
          publish_exited(&app, &state, exit_code, true);
          continue;
        }

        consecutive_crashes += 1;
        warn!("Companion exited unexpectedly (code={:?}), crash {}/{}", exit_code, consecutive_crashes, MAX_CONSECUTIVE_CRASHES);

        if consecutive_crashes >= MAX_CONSECUTIVE_CRASHES {
          error!("Companion crashed {} times in a row; giving up", consecutive_crashes);
          publish_status(&app, &state, CompanionStatus::FailedToStart {
            reason: format!("crashed {} times in a row (last exit code: {:?})", consecutive_crashes, exit_code),
          });
          emit_status_changed(&app, &state);
          return;
        }

        publish_exited(&app, &state, exit_code, true);
        tokio::time::sleep(backoff).await;
        backoff = next_backoff(backoff);
      }
      RunOutcome::NeverBecameReady => {
        consecutive_crashes += 1;
        error!("Companion never became healthy; crash {}/{}", consecutive_crashes, MAX_CONSECUTIVE_CRASHES);

        if consecutive_crashes >= MAX_CONSECUTIVE_CRASHES {
          publish_status(&app, &state, CompanionStatus::FailedToStart {
            reason: format!("failed health check {} times in a row", consecutive_crashes),
          });
          emit_status_changed(&app, &state);
          return;
        }

        tokio::time::sleep(backoff).await;
        backoff = next_backoff(backoff);
      }
      RunOutcome::FailedToSpawn(reason) => {
        error!("Companion supervisor: failed to spawn process: {}", reason);
        publish_status(&app, &state, CompanionStatus::FailedToStart { reason });
        emit_status_changed(&app, &state);
        return; // Spawn failures are almost always static (bad path, no interpreter) - don't loop.
      }
    }
  }
}

enum RunOutcome {
  /// Became healthy at some point; the `Child` is returned so the caller can
  /// await its actual exit (kept here rather than inside this function so
  /// `Ready` status stays published for the whole time the process runs,
  /// not just until this function returns).
  Ready(Child),
  NeverBecameReady,
  FailedToSpawn(String),
}

async fn spawn_and_supervise_once(
  app: &AppHandle,
  state: &CompanionState,
  launch_config: &CompanionLaunchConfig,
  auth_token: &str,
) -> RunOutcome {
  info!("Spawning Companion: {:?} {:?}", launch_config.program, launch_config.args);

  let mut command = Command::new(&launch_config.program);
  command
    .args(&launch_config.args)
    .current_dir(&launch_config.working_dir)
    .env("ARTCRAFT_STUDIO_COMPANION_AUTH_TOKEN", auth_token)
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .kill_on_drop(true); // Best-effort cleanup layer - see run_companion_supervisor's doc comment.

  let mut child = match command.spawn() {
    Ok(child) => child,
    Err(err) => {
      return RunOutcome::FailedToSpawn(format!(
        "could not spawn {:?}: {} (is a Python interpreter on PATH, or is ARTCRAFT_STUDIO_COMPANION_PYTHON set correctly?)",
        launch_config.program, err
      ));
    }
  };

  // Forward stderr to our own logs as it streams, so a Companion crash's
  // traceback shows up in the desktop app's log file without extra steps.
  if let Some(stderr) = child.stderr.take() {
    tauri::async_runtime::spawn(forward_companion_stderr(stderr));
  }

  let Some(stdout) = child.stdout.take() else {
    let _ = child.kill().await;
    return RunOutcome::FailedToSpawn("child stdout handle unavailable".to_string());
  };

  let port = match read_port_from_stdout(stdout).await {
    Some(port) => port,
    None => {
      warn!("Companion stdout closed before printing a port - process likely exited immediately");
      let _ = child.kill().await;
      return RunOutcome::NeverBecameReady;
    }
  };

  info!("Companion announced port {port}; waiting for health check");

  if !wait_for_health_check(port, auth_token).await {
    warn!("Companion did not pass health check within {} attempts", HEALTH_CHECK_MAX_ATTEMPTS);
    let _ = child.kill().await;
    return RunOutcome::NeverBecameReady;
  }

  info!("Companion healthy on port {port}");

  if let Err(err) = state.set(CompanionInfo {
    port: Some(port),
    auth_token: auth_token.to_string(),
    status: CompanionStatus::Ready,
  }) {
    error!("Companion supervisor: failed to publish Ready state: {:?}", err);
  }

  let payload = state.get().ok().flatten();
  if let Err(err) = app.emit(READY_EVENT, &payload) {
    error!("Failed to emit {}: {:?}", READY_EVENT, err);
  }
  emit_status_changed(app, state);

  RunOutcome::Ready(child)
}

/// Read stdout line-by-line until the port-announce marker appears, or the
/// stream ends (process exited before printing it). Lines that aren't the
/// marker are forwarded to our own logs, same as stderr, so early startup
/// diagnostics aren't silently dropped.
async fn read_port_from_stdout(stdout: tokio::process::ChildStdout) -> Option<u16> {
  let mut lines = BufReader::new(stdout).lines();
  while let Ok(Some(line)) = lines.next_line().await {
    if let Some(port_str) = line.strip_prefix(PORT_MARKER) {
      return port_str.trim().parse::<u16>().ok();
    }
    info!("[companion] {line}");
  }
  None
}

async fn forward_companion_stderr(stderr: tokio::process::ChildStderr) {
  let mut lines = BufReader::new(stderr).lines();
  while let Ok(Some(line)) = lines.next_line().await {
    warn!("[companion:stderr] {line}");
  }
}

async fn wait_for_health_check(port: u16, auth_token: &str) -> bool {
  let client = reqwest::Client::builder()
    .timeout(HEALTH_CHECK_TIMEOUT)
    .build();
  let Ok(client) = client else {
    return false;
  };

  let url = format!("http://127.0.0.1:{port}/");
  for attempt in 1..=HEALTH_CHECK_MAX_ATTEMPTS {
    // Must carry the shared-secret header - CompanionAuthMiddleware (Python
    // side) rejects everything, including this health check, without it.
    let request = client.get(&url).header(AUTH_HEADER_NAME, auth_token);
    match timeout(HEALTH_CHECK_TIMEOUT, request.send()).await {
      Ok(Ok(resp)) if resp.status().is_success() => return true,
      _ => {
        if attempt < HEALTH_CHECK_MAX_ATTEMPTS {
          tokio::time::sleep(HEALTH_CHECK_RETRY_DELAY).await;
        }
      }
    }
  }
  false
}

async fn wait_for_exit(child: &mut Child) -> Option<i32> {
  match child.wait().await {
    Ok(status) => status.code(),
    Err(err) => {
      error!("Error waiting on Companion process: {:?}", err);
      None
    }
  }
}

fn publish_status(app: &AppHandle, state: &CompanionState, status: CompanionStatus) {
  if let Err(err) = state.set_status(status.clone()) {
    error!("Companion supervisor: failed to publish status {:?}: {:?}", status, err);
  }
  emit_status_changed(app, state);
}

fn publish_exited(app: &AppHandle, state: &CompanionState, code: Option<i32>, restarting: bool) {
  publish_status(app, state, CompanionStatus::Exited { code, restarting });
  let payload = state.get().ok().flatten();
  if let Err(err) = app.emit(EXITED_EVENT, &payload) {
    error!("Failed to emit {}: {:?}", EXITED_EVENT, err);
  }
}

fn emit_status_changed(app: &AppHandle, state: &CompanionState) {
  let payload = state.get().ok().flatten();
  if let Err(err) = app.emit(STATUS_CHANGED_EVENT, &payload) {
    error!("Failed to emit {}: {:?}", STATUS_CHANGED_EVENT, err);
  }
}

/// True if a Companion exit should be treated as a deliberate, free restart
/// (no backoff, no crash-count increment) rather than a crash: either the
/// user explicitly requested it via `companion_restart_command`, or the
/// process used the exit-code-3 "please restart me" convention.
fn is_expected_restart(was_requested_restart: bool, exit_code: Option<i32>) -> bool {
  was_requested_restart || exit_code == Some(EXPECTED_RESTART_EXIT_CODE)
}

/// Doubles the current backoff, capped at `MAX_BACKOFF`.
fn next_backoff(current: Duration) -> Duration {
  (current * 2).min(MAX_BACKOFF)
}

#[cfg(test)]
mod tests {
  use super::*;

  mod restart_classification_tests {
    use super::*;

    #[test]
    fn requested_restart_is_expected_regardless_of_exit_code() {
      assert!(is_expected_restart(true, None));
      assert!(is_expected_restart(true, Some(1)));
      assert!(is_expected_restart(true, Some(EXPECTED_RESTART_EXIT_CODE)));
    }

    #[test]
    fn exit_code_three_is_expected_even_without_a_request() {
      assert!(is_expected_restart(false, Some(EXPECTED_RESTART_EXIT_CODE)));
    }

    #[test]
    fn any_other_unrequested_exit_is_a_crash() {
      assert!(!is_expected_restart(false, None));
      assert!(!is_expected_restart(false, Some(0)));
      assert!(!is_expected_restart(false, Some(1)));
    }
  }

  mod backoff_tests {
    use super::*;

    #[test]
    fn doubles_from_initial() {
      assert_eq!(next_backoff(INITIAL_BACKOFF), Duration::from_secs(2));
    }

    #[test]
    fn keeps_doubling_up_to_the_cap() {
      let mut backoff = INITIAL_BACKOFF;
      for _ in 0..10 {
        backoff = next_backoff(backoff);
      }
      assert_eq!(backoff, MAX_BACKOFF);
    }

    #[test]
    fn never_exceeds_the_cap_once_reached() {
      assert_eq!(next_backoff(MAX_BACKOFF), MAX_BACKOFF);
    }
  }
}
