use std::sync::Arc;
use tokio::sync::Notify;

/// Lets `companion_restart_command` interrupt an in-progress supervisor run
/// (kill the current child, skip any pending backoff sleep, restart
/// immediately with a reset backoff) without the command needing direct
/// access to the supervisor's own `tokio::process::Child` handle, which
/// never leaves `core::companion::supervisor`'s local scope.
///
/// A separate managed state (not folded into `CompanionState`) to match this
/// crate's convention of one concern per state struct - see e.g.
/// `SoraTaskQueue` / `GrokImagePromptQueue` living alongside their
/// respective credential managers rather than merged into one god-struct.
#[derive(Clone)]
pub struct CompanionRestartSignal {
  notify: Arc<Notify>,
}

impl CompanionRestartSignal {
  pub fn new() -> Self {
    Self { notify: Arc::new(Notify::new()) }
  }

  /// Called by `companion_restart_command`.
  pub fn request_restart(&self) {
    self.notify.notify_one();
  }

  /// Called by the supervisor loop; resolves when `request_restart()` is
  /// next called.
  pub async fn wait_for_restart_request(&self) {
    self.notify.notified().await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;

  #[tokio::test]
  async fn wait_resolves_after_request_restart_is_called() {
    let signal = CompanionRestartSignal::new();

    let waiter = tokio::spawn({
      let signal = signal.clone();
      async move { signal.wait_for_restart_request().await }
    });

    signal.request_restart();

    tokio::time::timeout(Duration::from_secs(1), waiter)
      .await
      .expect("wait_for_restart_request should resolve promptly after request_restart()")
      .expect("waiter task should not panic");
  }

  #[tokio::test]
  async fn wait_does_not_resolve_without_a_request() {
    let signal = CompanionRestartSignal::new();

    let result = tokio::time::timeout(Duration::from_millis(100), signal.wait_for_restart_request()).await;

    assert!(
      result.is_err(),
      "wait_for_restart_request resolved without request_restart() ever being called"
    );
  }

  #[tokio::test]
  async fn a_permit_recorded_before_waiting_is_still_consumed() {
    // tokio::sync::Notify stores at most one permit if notify_one() is
    // called before anyone is waiting - confirms the supervisor can't miss
    // a restart request that arrives in the brief window before it starts
    // its next wait_for_restart_request() call.
    let signal = CompanionRestartSignal::new();

    signal.request_restart();

    tokio::time::timeout(Duration::from_millis(100), signal.wait_for_restart_request())
      .await
      .expect("a permit recorded before waiting should still be consumed immediately");
  }
}
