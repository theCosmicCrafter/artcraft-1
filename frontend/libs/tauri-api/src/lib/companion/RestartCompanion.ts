import { invoke } from "@tauri-apps/api/core";

/**
 * Interrupts the currently-running Companion (if any) and has the supervisor
 * restart it immediately. Fire-and-forget: watch `companion://status_changed`
 * / `companion://ready` for the new state rather than awaiting readiness here.
 */
export const RestartCompanion = async (): Promise<void> => {
  await invoke("companion_restart_command");
};
