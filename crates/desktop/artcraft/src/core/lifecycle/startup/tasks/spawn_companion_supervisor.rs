use crate::core::companion::supervisor::run_companion_supervisor;
use crate::core::state::companion::companion_restart_signal::CompanionRestartSignal;
use crate::core::state::companion::companion_state::CompanionState;
use errors::AnyhowResult;
use tauri::AppHandle;

/// Starts the long-running Companion supervisor task (spawn/monitor/restart
/// the local Python service - see core::companion::supervisor). Follows the
/// same fire-and-forget `tauri::async_runtime::spawn` pattern as
/// `spawn_discord_presence_thread`, just with state to hand off first.
pub fn spawn_companion_supervisor(
  app: &AppHandle,
  companion_state: &CompanionState,
  restart_signal: &CompanionRestartSignal,
) -> AnyhowResult<()> {
  tauri::async_runtime::spawn(run_companion_supervisor(
    app.clone(),
    companion_state.clone(),
    restart_signal.clone(),
  ));

  Ok(())
}
