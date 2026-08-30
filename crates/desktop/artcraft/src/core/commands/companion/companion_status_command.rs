use crate::core::commands::response::shorthand::InfallibleResponse;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use crate::core::state::companion::companion_state::{CompanionInfo, CompanionState};
use log::warn;
use tauri::State;

impl SerializeMarker for Option<CompanionInfo> {}

/// Returns the Companion's current port/auth-token/status, or `None` if the
/// supervisor hasn't published anything yet (e.g. called before startup has
/// reached `spawn_companion_supervisor`). The frontend should treat `None`
/// the same as `Starting` - there is no meaningful difference to the user.
///
/// This doubles as what SYSTEM_DESIGN.md §1 calls `companion_get_base_url` -
/// the frontend derives `http://127.0.0.1:{port}` itself from `port` rather
/// than getting a pre-built URL, since it also needs `auth_token` for the
/// `X-ArtCraftStudio-Companion-Auth` header on every request.
#[tauri::command]
pub fn companion_status_command(
  companion_state: State<'_, CompanionState>,
) -> InfallibleResponse<Option<CompanionInfo>> {
  match companion_state.get() {
    Ok(info) => info.into(),
    Err(err) => {
      warn!("companion_status_command: failed to read companion state: {:?}", err);
      None.into()
    }
  }
}
