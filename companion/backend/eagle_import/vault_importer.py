"""One-way import: create real Companion library rows (MediaItem/Asset/Tag/
Board) from a parsed Eagle vault.

Builds on eagle_vault_parser.py's read-only scan/parse step - this module is
the read+write half that module's own docstring flagged as "deliberately NOT
implemented here yet ... needs to be built against the real schema rather
than guessed at." See artcraft-studio-plan/SYSTEM_DESIGN.md §6 for the full
design and the real-vault research it's grounded in.

Deliberately reuses this codebase's own existing primitives rather than
hand-rolling equivalents: `asset_service.create_asset_from_media` (the same
Media->Asset promotion path uploads and every other producer uses) and
`storage_service.stage_managed_media` (with `remove_source=False` - an
Eagle vault is the user's own permanent library, not transient upload
staging, so the original file is copied, never moved or deleted).
"""
from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from asset_service import create_asset_from_media
from core.logging import get_logger
from database import Asset, Board, BoardItem, BoardSection, MediaItem, MediaTag, Tag
from eagle_import.eagle_vault_parser import _require_vault_layout, iter_items, parse_folder_tree
from media_scanner import compute_file_hash
from storage_service import stage_managed_media

# Tracked on Asset.origin_type so a re-run of the same vault skips items it
# already imported (see _already_imported) - the one row type in this import
# path that has an origin_type/origin_id column at all (see the Board
# section below for why folder/board mapping can't use the same trick).
ORIGIN_TYPE = "eagle_import"

log = get_logger(__name__)


@dataclass
class ImportResult:
    imported: int = 0
    already_imported: int = 0
    skipped_deleted: int = 0
    skipped_missing_file: int = 0
    skipped_error: int = 0
    tags_touched: int = 0
    boards_touched: int = 0


def _eagle_origin_id(vault_path: Path, item_id: str) -> str:
    """Stable identity for the Asset.origin_id idempotency check. The vault
    path has to be part of the key - two different vaults could reuse the
    same Eagle item id (it's a per-vault Crockford ID, not globally unique)."""
    return f"{vault_path}:{item_id}"


async def _already_imported(session: AsyncSession, vault_path: Path, item_id: str) -> bool:
    origin_id = _eagle_origin_id(vault_path, item_id)
    existing = await session.scalar(
        select(Asset.id).where(Asset.origin_type == ORIGIN_TYPE, Asset.origin_id == origin_id)
    )
    return existing is not None


async def _get_or_create_tag(session: AsyncSession, cache: dict[str, Tag], tag_text: str) -> Tag:
    if tag_text in cache:
        return cache[tag_text]
    existing = await session.scalar(select(Tag).where(Tag.tag_text == tag_text))
    tag = existing or Tag(tag_text=tag_text)
    if existing is None:
        session.add(tag)
        await session.flush()
    cache[tag_text] = tag
    return tag


async def _get_or_create_board_section(session: AsyncSession, cache: dict[str, int], folder_name: str) -> int:
    """Returns the default BoardSection id for the Board an Eagle folder maps
    to, creating both if this is the first item in this run that needs it.

    Idempotency tradeoff, stated plainly: `Board` has no origin_type/
    origin_id column the way `Asset` does, so re-run safety here is
    name-based (`Board.name == "Eagle: <folder name>"`), not id-based. Safe
    against re-running this importer unchanged against the same vault; not
    safe against a user renaming the board in between runs (a second run
    would create a duplicate rather than recognizing the renamed one). A
    `Board.origin_id` column would close that gap - deferred rather than
    added speculatively for a case not yet confirmed to matter in practice.
    """
    board_name = f"Eagle: {folder_name}" if folder_name else "Eagle: (unnamed folder)"
    if board_name in cache:
        return cache[board_name]

    board = await session.scalar(select(Board).where(Board.name == board_name, Board.deleted_at.is_(None)))
    if board is None:
        board = Board(name=board_name)
        session.add(board)
        await session.flush()

    section = await session.scalar(
        select(BoardSection).where(BoardSection.board_id == board.id, BoardSection.is_default.is_(True))
    )
    if section is None:
        section = BoardSection(board_id=board.id, is_default=True, display_order=0)
        session.add(section)
        await session.flush()

    cache[board_name] = section.id
    return section.id


async def _import_one_item(session, profile_id, vault_path, item, folder_names_by_id, tag_cache, board_cache) -> str:
    """Returns "imported", "already_imported", or "missing_file" - errors
    raise instead, handled by the caller so one bad item can't lose the
    whole run's progress."""
    if await _already_imported(session, vault_path, item.id):
        return "already_imported"

    if not item.original_file_path.is_file():
        log.warning(f"Eagle item {item.id} ({item.name}): source file missing at {item.original_file_path}, skipping")
        return "missing_file"

    file_hash = compute_file_hash(item.original_file_path)
    stat_info = item.original_file_path.stat()
    width = item.width or 0
    height = item.height or 0

    media_item = MediaItem(
        file_path=str(item.original_file_path),
        file_hash=file_hash,
        file_size=stat_info.st_size,
        file_format=item.ext.lower(),
        original_filename=f"{item.name}.{item.ext}",
        created_date=datetime.utcfromtimestamp(stat_info.st_ctime),
        modified_date=datetime.utcfromtimestamp(stat_info.st_mtime),
        indexed_date=datetime.utcnow(),
        metadata_status="completed",
        metadata_processed_at=datetime.utcnow(),
        width=width,
        height=height,
        megapixels=(width * height) / 1_000_000,
        extracted_prompt=item.annotation or None,
    )
    session.add(media_item)
    await session.flush()

    await stage_managed_media(session, media=media_item, profile_id=profile_id, remove_source=False)

    await create_asset_from_media(
        session,
        media_id=media_item.id,
        origin_type=ORIGIN_TYPE,
        origin_id=_eagle_origin_id(vault_path, item.id),
    )

    for raw_tag in item.tags:
        tag_text = raw_tag.strip()
        if not tag_text:
            continue
        tag = await _get_or_create_tag(session, tag_cache, tag_text)
        link_exists = await session.scalar(
            select(MediaTag.tag_id).where(MediaTag.media_id == media_item.id, MediaTag.tag_id == tag.id)
        )
        if link_exists is None:
            session.add(MediaTag(media_id=media_item.id, tag_id=tag.id))

    for folder_id in item.folder_ids:
        folder_name = folder_names_by_id.get(folder_id, folder_id)
        section_id = await _get_or_create_board_section(session, board_cache, folder_name)
        item_exists = await session.scalar(
            select(BoardItem.media_id).where(
                BoardItem.board_section_id == section_id, BoardItem.media_id == media_item.id
            )
        )
        if item_exists is None:
            session.add(BoardItem(board_section_id=section_id, media_id=media_item.id))

    await session.commit()
    return "imported"


async def import_vault(
    session: AsyncSession,
    *,
    profile_id: str,
    vault_path: Path,
    include_deleted: bool = False,
) -> ImportResult:
    """Import every (non-deleted, by default) item in an Eagle vault into
    this profile's library. Safe to re-run against the same vault - already-
    imported items (tracked via `Asset.origin_type`/`origin_id`) are skipped,
    not re-created.

    Never writes into the vault itself - read-only there, matching
    eagle_vault_parser's "one-way import, not shared-vault editing" design.
    CLIP embeddings/captions are intentionally NOT computed here; imported
    items land with the normal `clip_status='pending'` default so the
    existing background ingestion worker picks them up the same way it
    would any other newly-indexed media (see SYSTEM_DESIGN.md §6 on why
    Eagle's own vector store isn't reusable).
    """
    _require_vault_layout(vault_path)
    folder_tree = parse_folder_tree(vault_path)
    folder_names_by_id = {f.id: f.name for root in folder_tree for f in root.flatten()}

    result = ImportResult()
    tag_cache: dict[str, Tag] = {}
    board_cache: dict[str, int] = {}

    for item in iter_items(vault_path):
        if item.is_deleted and not include_deleted:
            result.skipped_deleted += 1
            continue

        try:
            outcome = await _import_one_item(
                session, profile_id, vault_path, item, folder_names_by_id, tag_cache, board_cache
            )
        except Exception as exc:
            await session.rollback()
            log.error(f"Failed to import Eagle item {item.id} ({item.name}): {exc}")
            result.skipped_error += 1
            continue

        if outcome == "imported":
            result.imported += 1
        elif outcome == "already_imported":
            result.already_imported += 1
        else:
            result.skipped_missing_file += 1

    result.tags_touched = len(tag_cache)
    result.boards_touched = len(board_cache)
    return result
