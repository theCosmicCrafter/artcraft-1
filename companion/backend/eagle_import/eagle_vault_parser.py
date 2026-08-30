"""Parser for Eagle.cool `.library` vaults.

Schema verified against a real, live Eagle library (not just Eagle's public
docs, which turned out to describe less than the real on-disk shape) - see
artcraft-studio-plan/SYSTEM_DESIGN.md §6 for the full research and citations.

An Eagle vault is a plain folder, not a proprietary format:

    MyLibrary.library/
    ├── metadata.json          - library-wide folder tree (recursive)
    ├── tags.json               - autocomplete/frecency cache, NOT the
    │                             authoritative tag list (tags live per-item)
    └── images/
        └── <ID>.info/           (ID: 13-char Crockford-style, e.g. "M6G0KBEF35523")
            ├── <uuid>.<ext>              - the original file
            ├── <uuid>_thumbnail.png      - cached thumbnail
            └── metadata.json             - per-item record

This module only reads. It never writes into a `.library` folder - Eagle
compatibility here means one-way import, not shared-vault editing (see
SYSTEM_DESIGN.md §6 for why: CLIP embeddings need a real per-profile
database, not a JSON file per image, so Studio's library can never simply
adopt Eagle's on-disk format as its own).
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from core.logging import get_logger

log = get_logger(__name__)


class EagleVaultError(Exception):
    """Raised when a folder doesn't look like a valid Eagle `.library` vault."""


@dataclass
class EagleFolder:
    """One node in the library-wide folder tree (root `metadata.json`)."""
    id: str
    name: str
    description: str = ""
    children: list["EagleFolder"] = field(default_factory=list)

    def flatten(self) -> list["EagleFolder"]:
        """This folder plus every descendant, depth-first."""
        result = [self]
        for child in self.children:
            result.extend(child.flatten())
        return result


@dataclass
class EaglePalette:
    color: tuple[int, int, int]
    ratio: float


@dataclass
class EagleItem:
    """One item's `images/<ID>.info/metadata.json`, plus the resolved paths
    to its original file and thumbnail on disk."""
    id: str
    name: str
    ext: str
    size: int
    tags: list[str]
    folder_ids: list[str]
    annotation: str
    is_deleted: bool
    width: Optional[int]
    height: Optional[int]
    animated: bool
    palettes: list[EaglePalette]
    ocr_text: Optional[str]
    detected_objects: list[str]
    original_file_path: Path
    thumbnail_path: Optional[Path]


@dataclass
class EagleVaultSummary:
    """Pre-flight scan result - what UI_UX_PLAN.md §4 calls the "Found N
    items across M folders" step, shown before committing to a full import."""
    vault_path: Path
    folder_tree: list[EagleFolder]
    item_count: int
    folder_count: int
    deleted_item_count: int
    distinct_tags: set[str]


def _require_vault_layout(vault_path: Path) -> None:
    if not vault_path.is_dir():
        raise EagleVaultError(f"not a directory: {vault_path}")
    if not (vault_path / "metadata.json").is_file():
        raise EagleVaultError(
            f"missing metadata.json - doesn't look like an Eagle .library vault: {vault_path}"
        )
    if not (vault_path / "images").is_dir():
        raise EagleVaultError(
            f"missing images/ - doesn't look like an Eagle .library vault: {vault_path}"
        )


def _parse_folder_node(raw: dict) -> EagleFolder:
    return EagleFolder(
        id=raw.get("id", ""),
        name=raw.get("name", ""),
        description=raw.get("description", ""),
        children=[_parse_folder_node(child) for child in raw.get("children", [])],
    )


def parse_folder_tree(vault_path: Path) -> list[EagleFolder]:
    """Parse the root `metadata.json`'s recursive folder list."""
    root_metadata_path = vault_path / "metadata.json"
    try:
        raw = json.loads(root_metadata_path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError) as e:
        raise EagleVaultError(f"could not read/parse {root_metadata_path}: {e}") from e

    return [_parse_folder_node(f) for f in raw.get("folders", [])]


def _parse_item_metadata(info_dir: Path) -> Optional[EagleItem]:
    """Parse one `images/<ID>.info/metadata.json`. Returns None (logged, not
    raised) for a single malformed item so one bad item can't abort an
    entire vault scan."""
    metadata_path = info_dir / "metadata.json"
    try:
        raw = json.loads(metadata_path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError) as e:
        log.warning(f"skipping unreadable Eagle item at {info_dir}: {e}")
        return None

    item_id = raw.get("id", "")
    name = raw.get("name", "")
    ext = raw.get("ext", "")
    if not item_id or not name or not ext:
        log.warning(f"skipping Eagle item with missing id/name/ext at {info_dir}")
        return None

    original_file_path = info_dir / f"{name}.{ext}"
    thumbnail_path = info_dir / f"{name}_thumbnail.png"

    palettes = [
        EaglePalette(color=tuple(p.get("color", [0, 0, 0])), ratio=p.get("ratio", 0.0))
        for p in raw.get("palettes", [])
        if isinstance(p.get("color"), list) and len(p["color"]) == 3
    ]

    ocr = raw.get("ocr") or {}
    detection = raw.get("detection") or {}

    return EagleItem(
        id=item_id,
        name=name,
        ext=ext,
        size=raw.get("size", 0),
        tags=list(raw.get("tags", [])),
        folder_ids=list(raw.get("folders", [])),
        annotation=raw.get("annotation", ""),
        is_deleted=bool(raw.get("isDeleted", False)),
        width=raw.get("width"),
        height=raw.get("height"),
        animated=bool(raw.get("animated", False)),
        palettes=palettes,
        ocr_text=(ocr.get("text") or None) if ocr.get("done") else None,
        detected_objects=[o.get("name", "") for o in detection.get("objects", [])] if detection.get("done") else [],
        original_file_path=original_file_path,
        thumbnail_path=thumbnail_path if thumbnail_path.is_file() else None,
    )


def iter_items(vault_path: Path):
    """Yield every parseable item under `images/*.info/`. Generator, not a
    list - vaults can hold tens of thousands of items; callers that only
    need a count/preview (see `scan_vault`) shouldn't pay to materialize
    them all."""
    images_dir = vault_path / "images"
    for info_dir in sorted(images_dir.glob("*.info")):
        if not info_dir.is_dir():
            continue
        item = _parse_item_metadata(info_dir)
        if item is not None:
            yield item


def scan_vault(vault_path: Path) -> EagleVaultSummary:
    """Pre-flight scan: parse the folder tree and count items/tags without
    fully materializing every item. This is what the "Found N items across
    M folders" import-wizard step (UI_UX_PLAN.md §4) calls before the user
    commits to a full import.
    """
    _require_vault_layout(vault_path)

    folder_tree = parse_folder_tree(vault_path)
    all_folders = [f for root in folder_tree for f in root.flatten()]

    item_count = 0
    deleted_item_count = 0
    distinct_tags: set[str] = set()

    for item in iter_items(vault_path):
        item_count += 1
        if item.is_deleted:
            deleted_item_count += 1
        distinct_tags.update(item.tags)

    return EagleVaultSummary(
        vault_path=vault_path,
        folder_tree=folder_tree,
        item_count=item_count,
        folder_count=len(all_folders),
        deleted_item_count=deleted_item_count,
        distinct_tags=distinct_tags,
    )
