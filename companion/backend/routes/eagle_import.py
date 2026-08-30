"""Eagle.cool `.library` vault import routes.

Phase 1 scope (artcraft-studio-plan/SYSTEM_DESIGN.md §6, UI_UX_PLAN.md §4):
the pre-flight scan only - parse a vault and report what's in it, so the
import wizard can show "Found N items across M folders" before the user
commits. The actual import-into-local-library step (mapping EagleItem rows
into this Companion's Asset/AssetRevision/MediaItem tables - see
database.py, which turned out to have a fuller revisioning model than the
plan's original simplified "MediaItem" description assumed) is deliberately
NOT implemented here yet - it needs to be built against that real schema
rather than guessed at, and is tracked as its own follow-up rather than
rushed alongside the parser.
"""
from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy.ext.asyncio import AsyncSession

from core.app import get_process_pending_event
from core.dependencies import get_db_session
from core.logging import get_logger
from core.profile_context import get_current_profile
from eagle_import.eagle_vault_parser import EagleVaultError, scan_vault
from eagle_import.vault_importer import import_vault

router = APIRouter(prefix="/api/eagle_import", tags=["eagle_import"])
log = get_logger(__name__)


class ScanEagleVaultRequest(BaseModel):
    vault_path: str


class EagleFolderSummary(BaseModel):
    id: str
    name: str
    child_count: int


class ScanEagleVaultResponse(BaseModel):
    vault_path: str
    item_count: int
    folder_count: int
    deleted_item_count: int
    tag_count: int
    top_level_folders: list[EagleFolderSummary]


@router.post("/scan", response_model=ScanEagleVaultResponse)
async def scan_eagle_vault(request: ScanEagleVaultRequest) -> ScanEagleVaultResponse:
    """Pre-flight scan of an Eagle `.library` folder - the first step of the
    import wizard (UI_UX_PLAN.md §4). Read-only; never writes into the vault
    or the local library. Fails fast with a clear 400 if the folder doesn't
    look like a real Eagle vault, rather than a silent partial result.
    """
    vault_path = Path(request.vault_path).expanduser()

    try:
        summary = scan_vault(vault_path)
    except EagleVaultError as e:
        log.info(f"Eagle vault scan rejected: {e}")
        raise HTTPException(status_code=400, detail=str(e)) from e

    return ScanEagleVaultResponse(
        vault_path=str(summary.vault_path),
        item_count=summary.item_count,
        folder_count=summary.folder_count,
        deleted_item_count=summary.deleted_item_count,
        tag_count=len(summary.distinct_tags),
        top_level_folders=[
            EagleFolderSummary(id=f.id, name=f.name, child_count=len(f.children))
            for f in summary.folder_tree
        ],
    )


class ImportEagleVaultRequest(BaseModel):
    vault_path: str


class ImportEagleVaultResponse(BaseModel):
    imported: int
    already_imported: int
    skipped_deleted: int
    skipped_missing_file: int
    skipped_error: int
    tags_touched: int
    boards_touched: int


@router.post("/import", response_model=ImportEagleVaultResponse)
async def import_eagle_vault(
    request: ImportEagleVaultRequest,
    session: AsyncSession = Depends(get_db_session),
) -> ImportEagleVaultResponse:
    """Second step of the import wizard (UI_UX_PLAN.md §4), after `/scan` has
    shown the user what's there. Creates real library rows - unlike `/scan`,
    this writes. Safe to call again for the same vault: already-imported
    items are skipped, not duplicated (see vault_importer.import_vault).
    """
    vault_path = Path(request.vault_path).expanduser()
    profile_id = get_current_profile()

    try:
        result = await import_vault(session, profile_id=profile_id, vault_path=vault_path)
    except EagleVaultError as e:
        log.info(f"Eagle vault import rejected: {e}")
        raise HTTPException(status_code=400, detail=str(e)) from e

    if result.imported:
        # Newly-imported items land with clip_status/vlm_caption_status
        # still 'pending' (see import_vault's docstring) - wake the
        # background ingestion worker the same way any other new media
        # would, rather than leaving them unindexed until its next poll.
        process_pending_event = get_process_pending_event()
        if process_pending_event:
            process_pending_event.set()

    return ImportEagleVaultResponse(
        imported=result.imported,
        already_imported=result.already_imported,
        skipped_deleted=result.skipped_deleted,
        skipped_missing_file=result.skipped_missing_file,
        skipped_error=result.skipped_error,
        tags_touched=result.tags_touched,
        boards_touched=result.boards_touched,
    )
