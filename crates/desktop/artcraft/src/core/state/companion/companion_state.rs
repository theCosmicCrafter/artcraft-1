use crate::core::artcraft_error::ArtcraftError;
use log::error;
use serde_derive::Serialize;
use std::sync::{Arc, RwLock};

/// Lifecycle status of the local Companion Service child process.
///
/// The Companion is a Python/FastAPI service bundled with the desktop app - see
/// artcraft-studio-plan/SYSTEM_DESIGN.md §1. This enum mirrors the states the
/// supervisor (core::companion::supervisor) walks through.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum CompanionStatus {
  /// Process spawned; waiting for the port-announce line on stdout and a
  /// passing health check.
  Starting,
  /// Health check passed; ready to accept requests on `CompanionInfo::port`.
  Ready,
  /// Process exited on its own. `restarting` is true if the supervisor will
  /// retry (crash or the main.py exit-code-3 "please restart me" convention);
  /// false once the retry budget is exhausted.
  Exited { code: Option<i32>, restarting: bool },
  /// The supervisor could not even spawn the process (binary/script not
  /// found, etc.) - distinct from `Exited` because there was never a PID.
  FailedToStart { reason: String },
}

/// Everything the frontend needs to talk to the Companion directly over
/// localhost HTTP/WS, plus enough status to render a "starting up" UI state.
///
/// `port` is `None` until the port-announce line has been read from the
/// child's stdout - the frontend should treat a `None` port as "not ready
/// yet" regardless of `status` (defensive: don't race a stale port).
///
/// `auth_token` is generated fresh per spawn attempt and must be sent as the
/// `X-ArtCraftStudio-Companion-Auth` header on every request to the
/// Companion (SYSTEM_DESIGN.md §1's shared-secret handshake) - the Companion
/// binds to 127.0.0.1 only, but loopback-reachable is not the same as
/// authenticated, so this closes the "any local process can call it" gap
/// that CORS alone (Stimma's original design) does not.
#[derive(Clone, Debug, Serialize)]
pub struct CompanionInfo {
  pub port: Option<u16>,
  pub auth_token: String,
  pub status: CompanionStatus,
}

/// Shared, thread-safe handle to the Companion's current `CompanionInfo`.
///
/// Follows the same `Arc<RwLock<Option<T>>>` shape as
/// `WorldlabsBearerBridge` - a single source of truth commands read from and
/// the supervisor task writes to. The supervisor owns the actual
/// `tokio::process::Child` handle itself (not stored here - `Child` isn't
/// `Clone` and nothing outside the supervisor needs to touch it directly);
/// this struct only ever holds the small, `Clone`-able public info.
#[derive(Clone)]
pub struct CompanionState {
  inner: Arc<RwLock<Option<CompanionInfo>>>,
}

impl CompanionState {
  pub fn empty() -> Self {
    Self {
      inner: Arc::new(RwLock::new(None)),
    }
  }

  pub fn get(&self) -> Result<Option<CompanionInfo>, ArtcraftError> {
    match self.inner.read() {
      Ok(state) => Ok(state.clone()),
      Err(err) => {
        error!("Lock poisoned: {:?}", err);
        Err(ArtcraftError::RwLockReadError)
      }
    }
  }

  pub fn set(&self, new_state: CompanionInfo) -> Result<(), ArtcraftError> {
    match self.inner.write() {
      Ok(mut state) => {
        *state = Some(new_state);
        Ok(())
      }
      Err(err) => {
        error!("Lock poisoned: {:?}", err);
        Err(ArtcraftError::RwLockWriteError)
      }
    }
  }

  /// Update just the status of the current info, if any is set. No-ops if
  /// nothing has been set yet (the supervisor always calls `set()` first).
  pub fn set_status(&self, status: CompanionStatus) -> Result<(), ArtcraftError> {
    match self.inner.write() {
      Ok(mut state) => {
        if let Some(info) = state.as_mut() {
          info.status = status;
        }
        Ok(())
      }
      Err(err) => {
        error!("Lock poisoned: {:?}", err);
        Err(ArtcraftError::RwLockWriteError)
      }
    }
  }

  pub fn clear(&self) -> Result<(), ArtcraftError> {
    match self.inner.write() {
      Ok(mut state) => {
        *state = None;
        Ok(())
      }
      Err(err) => {
        error!("Lock poisoned: {:?}", err);
        Err(ArtcraftError::RwLockWriteError)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const TEST_AUTH_TOKEN: &str = "test-auth-token";

  mod get_set_tests {
    use super::*;

    #[test]
    fn empty_state_returns_none() {
      let state = CompanionState::empty();
      assert!(state.get().unwrap().is_none());
    }

    #[test]
    fn set_then_get_round_trips() {
      let state = CompanionState::empty();
      state.set(starting_info()).unwrap();

      let info = state.get().unwrap().unwrap();
      assert_eq!(info.port, None);
      assert_eq!(info.auth_token, TEST_AUTH_TOKEN);
      assert!(matches!(info.status, CompanionStatus::Starting));
    }

    #[test]
    fn set_overwrites_previous_value() {
      let state = CompanionState::empty();
      state.set(starting_info()).unwrap();
      state.set(ready_info(9999)).unwrap();

      let info = state.get().unwrap().unwrap();
      assert_eq!(info.port, Some(9999));
      assert!(matches!(info.status, CompanionStatus::Ready));
    }

    #[test]
    fn clear_resets_to_none() {
      let state = CompanionState::empty();
      state.set(starting_info()).unwrap();
      state.clear().unwrap();

      assert!(state.get().unwrap().is_none());
    }

    #[test]
    fn clear_on_already_empty_state_is_a_no_op() {
      let state = CompanionState::empty();
      state.clear().unwrap();

      assert!(state.get().unwrap().is_none());
    }
  }

  mod set_status_tests {
    use super::*;

    #[test]
    fn updates_status_on_existing_info_without_touching_other_fields() {
      let state = CompanionState::empty();
      state.set(ready_info(1234)).unwrap();

      state
        .set_status(CompanionStatus::Exited { code: Some(1), restarting: true })
        .unwrap();

      let info = state.get().unwrap().unwrap();
      // port/auth_token must survive a status-only update.
      assert_eq!(info.port, Some(1234));
      assert_eq!(info.auth_token, TEST_AUTH_TOKEN);
      assert!(matches!(
        info.status,
        CompanionStatus::Exited { code: Some(1), restarting: true }
      ));
    }

    #[test]
    fn no_ops_when_nothing_has_been_set_yet() {
      let state = CompanionState::empty();

      // Must not panic or surface an error - the supervisor always calls
      // set() before set_status(), but this should still degrade safely.
      state.set_status(CompanionStatus::Ready).unwrap();

      assert!(state.get().unwrap().is_none());
    }
  }

  mod poisoned_lock_tests {
    use super::*;

    #[test]
    fn get_and_set_and_set_status_all_surface_the_error_variant_after_poisoning() {
      let state = CompanionState::empty();
      state.set(starting_info()).unwrap();

      // Poison the lock the same way production code could: a panic while
      // holding the write guard. std::panic::catch_unwind suppresses the
      // panic's default abort-the-test behavior so we can assert on what
      // happens afterward, which is the actual thing under test.
      let poisoning_state = state.clone();
      let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = poisoning_state.inner.write().unwrap();
        panic!("intentional panic to poison the lock");
      }));

      assert!(matches!(state.get(), Err(ArtcraftError::RwLockReadError)));
      assert!(matches!(
        state.set(starting_info()),
        Err(ArtcraftError::RwLockWriteError)
      ));
      assert!(matches!(
        state.set_status(CompanionStatus::Ready),
        Err(ArtcraftError::RwLockWriteError)
      ));
      assert!(matches!(state.clear(), Err(ArtcraftError::RwLockWriteError)));
    }
  }

  fn starting_info() -> CompanionInfo {
    CompanionInfo {
      port: None,
      auth_token: TEST_AUTH_TOKEN.to_string(),
      status: CompanionStatus::Starting,
    }
  }

  fn ready_info(port: u16) -> CompanionInfo {
    CompanionInfo {
      port: Some(port),
      auth_token: TEST_AUTH_TOKEN.to_string(),
      status: CompanionStatus::Ready,
    }
  }
}
