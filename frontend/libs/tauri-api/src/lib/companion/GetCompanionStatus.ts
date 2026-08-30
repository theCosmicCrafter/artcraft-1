import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";

// Mirrors crates/desktop/artcraft/src/core/state/companion/companion_state.rs.
// Keep in sync with that file - this is a hand-maintained mirror, not codegen.
export type CompanionStatus =
  | { state: "starting" }
  | { state: "ready" }
  | { state: "exited"; code?: number; restarting: boolean }
  | { state: "failed_to_start"; reason: string };

export interface CompanionInfo {
  port?: number;
  auth_token: string;
  status: CompanionStatus;
}

interface GetCompanionStatusSuccess extends CommandResult {
  payload?: CompanionInfo | null;
}

/**
 * Current Companion port/auth-token/status, or `null`/`undefined` if the
 * supervisor hasn't published anything yet (e.g. called very early during
 * app startup, before `spawn_companion_supervisor` has run). Poll this or
 * (preferably) subscribe to the `companion://status_changed` /
 * `companion://ready` events instead of polling in a loop.
 */
export const GetCompanionStatus = async (): Promise<CompanionInfo | null> => {
  const result = (await invoke(
    "companion_status_command"
  )) as GetCompanionStatusSuccess;

  return result?.payload ?? null;
};
