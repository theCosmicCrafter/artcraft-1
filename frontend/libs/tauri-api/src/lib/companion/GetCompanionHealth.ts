import { companionFetch } from "./CompanionFetch";
import { CompanionInfo } from "./GetCompanionStatus";

// Mirrors the JSON shape of companion/backend/main.py's `@app.get("/")`.
export interface CompanionHealth {
  status: string;
  service: string;
}

/**
 * Calls the Companion's own root health endpoint through the authenticated
 * `companionFetch` bridge. This is deliberately the first (and, as of this
 * pass, only) real Companion *data* endpoint the frontend calls - everything
 * else under `companion/` so far only talks to the Tauri supervisor
 * (`GetCompanionStatus`/`RestartCompanion`), which proves the process is
 * alive but not that the frontend can actually reach it over HTTP with the
 * CSP + capability grant + auth header all correctly wired together.
 *
 * Throws `CompanionUnreachableError` (network/no-port) or a plain `Error`
 * if the Companion responded but not with 2xx/valid JSON - callers should
 * treat both as "not verified reachable", the distinction mostly matters
 * for logging.
 */
export const GetCompanionHealth = async (info: CompanionInfo | null): Promise<CompanionHealth> => {
  const response = await companionFetch(info, "/");

  if (!response.ok) {
    throw new Error(`Companion health check returned HTTP ${response.status}`);
  }

  const body = (await response.json()) as Partial<CompanionHealth>;
  if (typeof body.status !== "string" || typeof body.service !== "string") {
    throw new Error("Companion health check returned an unexpected response shape");
  }

  return body as CompanionHealth;
};
