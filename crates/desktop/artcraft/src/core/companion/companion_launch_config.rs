use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Env var a developer can set to point at a specific Python interpreter for
/// running the Companion from source. Without it, we fall back to whatever
/// `python3`/`python` resolves to on PATH - which is NOT reliable on Windows
/// (the bare `python`/`python3` names can resolve to the Microsoft Store's
/// "app execution alias" stub instead of a real interpreter; see
/// companion/README.md for the concrete fix). This env var is the escape
/// hatch, not a bug to paper over with a hardcoded machine-specific path.
const PYTHON_OVERRIDE_ENV: &str = "ARTCRAFT_STUDIO_COMPANION_PYTHON";

/// The bundle ID this build reports to the Companion (must match one of the
/// values companion/backend/app_context.py's set_app_context() accepts).
/// Debug builds use the `.debug` channel; see tauri.conf.json's `identifier`
/// for the matching Tauri-side bundle ID (`ai.artcraftstudio.app`).
fn bundle_id() -> &'static str {
  if cfg!(debug_assertions) {
    "ai.artcraftstudio.app.debug"
  } else {
    "ai.artcraftstudio.app"
  }
}

/// Everything needed to spawn the Companion process: the executable, its
/// argv, and the working directory it should run from.
#[derive(Debug, Clone)]
pub struct CompanionLaunchConfig {
  pub program: PathBuf,
  pub args: Vec<String>,
  pub working_dir: PathBuf,
}

/// Resolve how to launch the Companion, trying a packaged build's bundled
/// interpreter first and falling back to a from-source dev build.
///
/// Packaged-build support (see artcraft-studio-plan/DEVOPS.md §2 for the
/// full build-stage design): the release CI workflow vendors an embeddable
/// Python + the Companion's dependencies into `companion/python/` before
/// `tauri build` runs, and `tauri.conf.json`'s `bundle.resources` copies the
/// whole `companion/` tree into the installed app's resource directory -
/// the exact layout `companion/run.cmd`/`run.sh` already expect (matching
/// Stimma's own working shape those scripts were carried over from). This
/// resolves the same `python/python.exe` + `backend/main.py` pair those
/// scripts point at, rather than shelling out to the scripts themselves -
/// spawning a `.cmd`/`.sh` wrapper would add a process-tree hop the
/// supervisor's `kill_on_drop`/restart logic would then have to reach
/// through instead of holding the real interpreter's `Child` handle
/// directly (see supervisor.rs's own doc comment on why that handle
/// matters).
///
/// Not yet exercised end-to-end against a real packaged build in this repo
/// (no `tauri`/`cargo-tauri` CLI available to produce one in this pass) -
/// the resource-presence check below (`main.py`/`python.exe` both existing
/// under the resolved resource dir) is what makes falling through to dev
/// mode safe if that's ever wrong, rather than silently launching a
/// half-resolved path.
pub fn resolve_launch_config(app: &AppHandle) -> Result<CompanionLaunchConfig, String> {
  match resolve_packaged_launch_config(app) {
    Ok(config) => Ok(config),
    Err(packaged_err) => resolve_dev_launch_config().map_err(|dev_err| {
      format!(
        "no packaged Companion found ({packaged_err}); dev-mode fallback also failed ({dev_err})"
      )
    }),
  }
}

fn resolve_packaged_launch_config(app: &AppHandle) -> Result<CompanionLaunchConfig, String> {
  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|e| format!("could not resolve app resource directory: {e}"))?;
  // `resource_dir()` returns a Windows extended-length path (`\\?\C:\...`) -
  // confirmed the hard way: with it, the Companion's Alembic migrations
  // failed with `OSError(22, 'Invalid argument')` partway through (the
  // "add_chat_system" migration, which does more file/SQLite work than the
  // ones before it) on a real packaged build; the identical Companion run
  // via a normal (non-`\\?\`) path completed every migration cleanly. The
  // `\\?\` prefix is a well-known Windows gotcha - various file-handling
  // code (here: somewhere in Alembic/SQLite's own path handling) doesn't
  // expect it. Stripping it here, once, keeps every downstream path (the
  // interpreter's own argv, main.py's path, the working dir) normal.
  let resource_dir = strip_windows_extended_length_prefix(resource_dir);

  let companion_dir = resource_dir.join("companion");
  let backend_dir = companion_dir.join("backend");
  let main_py = backend_dir.join("main.py");
  let python = packaged_python_executable(&companion_dir);

  if !main_py.is_file() {
    return Err(format!("packaged Companion entry point not found at {main_py:?}"));
  }
  if !python.is_file() {
    return Err(format!("packaged Companion Python interpreter not found at {python:?}"));
  }

  Ok(CompanionLaunchConfig {
    program: python,
    args: companion_argv(&main_py),
    working_dir: backend_dir,
  })
}

/// Strips a Windows extended-length path prefix (`\\?\` for local paths,
/// `\\?\UNC\` for network paths) if present, else returns the path
/// unchanged - a no-op on non-Windows or already-normal paths. See
/// `resolve_packaged_launch_config`'s comment for why this matters: real,
/// confirmed migration failures with the prefix present, none without it.
fn strip_windows_extended_length_prefix(path: PathBuf) -> PathBuf {
  let raw = path.to_string_lossy();
  if let Some(unc_rest) = raw.strip_prefix(r"\\?\UNC\") {
    PathBuf::from(format!(r"\\{unc_rest}"))
  } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
    PathBuf::from(rest)
  } else {
    path
  }
}

#[cfg(target_os = "windows")]
fn packaged_python_executable(companion_dir: &Path) -> PathBuf {
  // Only one Windows build target exists in this repo's CI today
  // (`windows-latest`, i.e. x86_64) - no per-arch split needed here, unlike
  // the macOS case below.
  companion_dir.join("python").join("python.exe")
}

/// python-build-standalone's (and CPython's own embeddable-on-Unix
/// convention's) layout: `python/bin/python3`, not `python/python.exe` -
/// unlike `companion/run.sh`, which references `python/python.exe`
/// unconditionally even though it's a bash script. That's a real,
/// pre-existing bug in `run.sh` (presumably copy-pasted from the Windows
/// original without adjustment - there is no verified working macOS/Linux
/// packaged distribution anywhere in this repo's history to confirm
/// against), fixed alongside this to match the standard convention rather
/// than propagated into new code.
#[cfg(target_os = "linux")]
fn packaged_python_executable(companion_dir: &Path) -> PathBuf {
  companion_dir.join("python").join("bin").join("python3")
}

/// macOS is built as `universal-apple-darwin` (see
/// .github/workflows/artcraft-macos-publish.yml) - one fat Mach-O binary
/// covering both arm64 and x86_64, chosen by the OS loader at launch based
/// on the *running* CPU. A bundled Python resource is just files, not a
/// Mach-O the loader unifies the same way - an x86_64-only interpreter
/// would still run on Apple Silicon under Rosetta 2, but silently lose
/// native performance for exactly the CPU-bound work (ONNX/CLIP inference)
/// this Companion does. So both architectures are vendored side by side
/// (`python-aarch64/`, `python-x86_64/`) and this picks the one matching
/// the *actual running* architecture via `std::env::consts::ARCH` - which
/// correctly reflects the process's real architecture even inside a
/// universal binary, unlike a compile-time `cfg!(target_arch)` here (that
/// would bake in whichever arch the Rust *host* compiler used, not
/// necessarily the arch the user's Mac is actually running as).
#[cfg(target_os = "macos")]
fn packaged_python_executable(companion_dir: &Path) -> PathBuf {
  let arch_dir = match std::env::consts::ARCH {
    "aarch64" => "python-aarch64",
    _ => "python-x86_64",
  };
  companion_dir.join(arch_dir).join("bin").join("python3")
}

/// argv shared by both the packaged and dev launch paths - `python <main.py>
/// --bundle-id ... --sandbox default --port 0` - so the two resolvers can't
/// silently drift apart on flags.
fn companion_argv(main_py: &Path) -> Vec<String> {
  vec![
    main_py.to_string_lossy().to_string(),
    "--bundle-id".to_string(),
    bundle_id().to_string(),
    "--sandbox".to_string(),
    "default".to_string(),
    "--port".to_string(),
    "0".to_string(),
  ]
}

/// Resolve how to launch the Companion for a from-source dev build.
///
/// Locates `companion/` via `CARGO_MANIFEST_DIR` (a compile-time constant
/// pointing at `crates/desktop/artcraft/`), which only makes sense for
/// from-source builds - this is a known, deliberate limitation, not an
/// oversight. Packaged builds use `resolve_launch_config`'s packaged path
/// above instead; this function remains the fallback (and the whole story
/// for a plain `cargo tauri dev` run, where no bundled resource dir exists).
pub fn resolve_dev_launch_config() -> Result<CompanionLaunchConfig, String> {
  let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let repo_root = manifest_dir
    .parent() // crates/desktop
    .and_then(|p| p.parent()) // crates
    .and_then(|p| p.parent()) // repo root
    .ok_or_else(|| format!("could not walk up from CARGO_MANIFEST_DIR: {:?}", manifest_dir))?
    .to_path_buf();

  let companion_dir = repo_root.join("companion");
  let backend_dir = companion_dir.join("backend");
  let main_py = backend_dir.join("main.py");

  if !main_py.is_file() {
    return Err(format!(
      "companion entry point not found at {:?} - is this a from-source checkout with companion/ present?",
      main_py
    ));
  }

  let python = std::env::var(PYTHON_OVERRIDE_ENV)
    .ok()
    .filter(|s| !s.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(default_python_command);

  Ok(CompanionLaunchConfig {
    program: python,
    args: companion_argv(&main_py),
    working_dir: backend_dir,
  })
}

#[cfg(target_os = "windows")]
fn default_python_command() -> PathBuf {
  // `python3` is rarely registered on Windows; `python` is more common but
  // risks resolving to the WindowsApps alias stub if no real interpreter is
  // on PATH ahead of it. Either way this is a best-effort default - see
  // PYTHON_OVERRIDE_ENV above for the reliable path.
  PathBuf::from("python")
}

#[cfg(not(target_os = "windows"))]
fn default_python_command() -> PathBuf {
  PathBuf::from("python3")
}

#[cfg(test)]
mod tests {
  use super::*;

  mod strip_windows_extended_length_prefix_tests {
    use super::*;

    #[test]
    fn strips_the_local_path_prefix() {
      let stripped = strip_windows_extended_length_prefix(PathBuf::from(r"\\?\C:\Users\test\companion"));
      assert_eq!(stripped, PathBuf::from(r"C:\Users\test\companion"));
    }

    #[test]
    fn strips_the_unc_path_prefix_down_to_a_normal_unc_path() {
      let stripped = strip_windows_extended_length_prefix(PathBuf::from(r"\\?\UNC\server\share\companion"));
      assert_eq!(stripped, PathBuf::from(r"\\server\share\companion"));
    }

    #[test]
    fn leaves_an_already_normal_path_unchanged() {
      let stripped = strip_windows_extended_length_prefix(PathBuf::from(r"C:\Users\test\companion"));
      assert_eq!(stripped, PathBuf::from(r"C:\Users\test\companion"));
    }

    #[test]
    fn leaves_a_relative_path_unchanged() {
      let stripped = strip_windows_extended_length_prefix(PathBuf::from("companion"));
      assert_eq!(stripped, PathBuf::from("companion"));
    }
  }

  mod packaged_python_executable_tests {
    use super::*;

    #[test]
    fn resolves_somewhere_under_the_companion_dir() {
      let companion_dir = Path::new("/resources/companion");
      let python = packaged_python_executable(companion_dir);
      assert!(python.starts_with(companion_dir));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn is_python_exe_at_the_python_subdirectory_root_on_windows() {
      let python = packaged_python_executable(Path::new("/resources/companion"));
      assert_eq!(python, Path::new("/resources/companion/python/python.exe"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn is_bin_python3_on_linux() {
      let python = packaged_python_executable(Path::new("/resources/companion"));
      assert_eq!(python, Path::new("/resources/companion/python/bin/python3"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn picks_the_arch_specific_directory_matching_the_running_process_on_macos() {
      let python = packaged_python_executable(Path::new("/resources/companion"));
      let expected_dir = if std::env::consts::ARCH == "aarch64" { "python-aarch64" } else { "python-x86_64" };
      assert_eq!(python, Path::new("/resources/companion").join(expected_dir).join("bin").join("python3"));
    }
  }

  mod companion_argv_tests {
    use super::*;

    #[test]
    fn main_py_is_the_first_argument() {
      let main_py = Path::new("/resources/companion/backend/main.py");
      let argv = companion_argv(main_py);
      assert_eq!(argv[0], main_py.to_string_lossy());
    }

    #[test]
    fn always_includes_bundle_id_sandbox_and_ephemeral_port() {
      let argv = companion_argv(Path::new("main.py"));
      assert!(argv.contains(&"--bundle-id".to_string()));
      assert!(argv.contains(&bundle_id().to_string()));
      assert!(argv.contains(&"--sandbox".to_string()));
      assert!(argv.contains(&"default".to_string()));
      assert!(argv.contains(&"--port".to_string()));
      // "0" - let the OS assign an ephemeral port; the supervisor reads the
      // real one back off the port-announce line on stdout (see
      // supervisor.rs's PORT_MARKER), never assumes a fixed port.
      assert!(argv.contains(&"0".to_string()));
    }
  }
}
