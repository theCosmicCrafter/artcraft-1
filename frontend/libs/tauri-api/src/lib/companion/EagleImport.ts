import { companionFetch } from "./CompanionFetch";
import { CompanionInfo } from "./GetCompanionStatus";

// Mirrors companion/backend/routes/eagle_import.py's response models.
export interface EagleFolderSummary {
  id: string;
  name: string;
  child_count: number;
}

export interface ScanEagleVaultResult {
  vault_path: string;
  item_count: number;
  folder_count: number;
  deleted_item_count: number;
  tag_count: number;
  top_level_folders: EagleFolderSummary[];
}

export interface ImportEagleVaultResult {
  imported: number;
  already_imported: number;
  skipped_deleted: number;
  skipped_missing_file: number;
  skipped_error: number;
  tags_touched: number;
  boards_touched: number;
}

async function postJson<T>(info: CompanionInfo | null, path: string, body: unknown): Promise<T> {
  const response = await companionFetch(info, path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const detail = await response.json().catch(() => null);
    throw new Error(detail?.detail ?? `Eagle import request to ${path} returned HTTP ${response.status}`);
  }
  return (await response.json()) as T;
}

/**
 * Pre-flight step of the Eagle `.library` import wizard (UI_UX_PLAN.md §4) -
 * read-only, reports what's in a vault ("Found N items across M folders")
 * before the user commits to importing it.
 */
export const ScanEagleVault = (info: CompanionInfo | null, vaultPath: string): Promise<ScanEagleVaultResult> =>
  postJson(info, "/api/eagle_import/scan", { vault_path: vaultPath });

/**
 * Actually imports a scanned vault into the local library. Safe to call
 * again for the same vault - the Companion skips items it already
 * imported rather than duplicating them (see vault_importer.py).
 */
export const ImportEagleVault = (info: CompanionInfo | null, vaultPath: string): Promise<ImportEagleVaultResult> =>
  postJson(info, "/api/eagle_import/import", { vault_path: vaultPath });
