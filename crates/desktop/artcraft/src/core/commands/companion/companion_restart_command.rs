use crate::core::commands::response::shorthand::InfallibleResponse;
use crate::core::state::companion::companion_restart_signal::CompanionRestartSignal;
use log::info;
use tauri::State;

/// Interrupts the currently-running Companion (if any) and has the
/// supervisor restart it immediately with a reset backoff - see
/// `CompanionRestartSignal` and `core::companion::supervisor`'s
/// `tokio::select!` between "child exited naturally" and "restart
/// requested". Fire-and-forget: the frontend should watch
/// `companion://status_changed` / poll `companion_status_command` for the
/// new state rather than expect this command itself to block until ready.
#[tauri::command]
pub fn companion_restart_command(
  restart_signal: State<'_, CompanionRestartSignal>,
) -> InfallibleResponse<()> {
  info!("companion_restart_command called");
  restart_signal.request_restart();
  ().into()
}
