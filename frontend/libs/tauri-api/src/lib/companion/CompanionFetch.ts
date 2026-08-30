import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import { CompanionInfo } from "./GetCompanionStatus";

// Mirrors supervisor.rs's AUTH_HEADER_NAME - keep both in sync.
const AUTH_HEADER = "X-ArtCraftStudio-Companion-Auth";

/**
 * Thrown when `companionFetch` is called before the supervisor has
 * published a port (still starting, or never became healthy), or when the
 * request itself fails at the network layer (Companion process died, CSP/
 * capability misconfiguration, etc.) - callers should treat this as "not
 * reachable right now", not surface it as an application error.
 */
export class CompanionUnreachableError extends Error {
  constructor(message: string, cause?: unknown) {
    super(message);
    this.name = "CompanionUnreachableError";
    this.cause = cause;
  }
}

/**
 * Authenticated fetch against the local Companion Service, using the port
 * and shared-secret token the Rust supervisor publishes via
 * `GetCompanionStatus`/`companion://status_changed` (SYSTEM_DESIGN.md §1).
 *
 * Uses `@tauri-apps/plugin-http`'s fetch (not the browser's native fetch) so
 * the request goes through the scoped `http:default` capability grant
 * (`http://127.0.0.1:*` in capabilities/default.json) rather than the
 * webview's own network stack - this is a desktop-only concept (there is no
 * local Companion to reach from a plain web deployment), so unlike
 * `@storyteller/tauri-utils`'s `FetchProxy` there is intentionally no
 * browser-fetch fallback branch here.
 */
export const companionFetch = async (
  info: CompanionInfo | null,
  path: string,
  init?: RequestInit,
): Promise<Response> => {
  if (!info?.port) {
    throw new CompanionUnreachableError(
      "Companion has no published port yet (still starting, or never became healthy)",
    );
  }

  const url = `http://127.0.0.1:${info.port}${path}`;
  const headers = new Headers(init?.headers);
  headers.set(AUTH_HEADER, info.auth_token);

  try {
    return await tauriFetch(url, { ...init, headers });
  } catch (err) {
    throw new CompanionUnreachableError(`Request to Companion (${path}) failed`, err);
  }
};
