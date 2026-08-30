import { useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { CompanionInfo } from "@storyteller/tauri-api";

// Companion events are plain string-keyed `app.emit()` calls from
// core/companion/supervisor.rs - NOT routed through the shared
// `TauriEventName` enum (crates/schema/public/enums), which is
// database-persisted with strict "never remove a variant" rules meant for
// backend-driven events. These are purely local/ephemeral, so a dedicated,
// lightweight hook here is the better fit. See
// artcraft-studio-plan/SYSTEM_DESIGN.md §4 for the full reasoning.
const COMPANION_STATUS_CHANGED_EVENT = "companion://status_changed";

/**
 * Fires on every Companion state transition (starting/ready/exited/
 * failed_to_start) - the general-purpose hook for rendering "Companion is
 * starting up..." / "ready" / "crashed" UI. For edge-triggered behavior
 * (e.g. "do X exactly once when it becomes ready"), listen to
 * `companion://ready` / `companion://exited` directly instead.
 */
export const useCompanionStatusChangedEvent = (
  onChange: (info: CompanionInfo | null) => void
) => {
  useEffect(() => {
    let isUnmounted = false;
    let unlisten: Promise<UnlistenFn>;

    const setup = async () => {
      unlisten = listen<CompanionInfo | null>(
        COMPANION_STATUS_CHANGED_EVENT,
        (event) => {
          onChange(event.payload ?? null);
        }
      );

      if (isUnmounted) {
        unlisten.then((f) => f()); // Unsubscribe if unmounted early.
      }
    };

    setup();

    return () => {
      isUnmounted = true;
      unlisten.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
};
