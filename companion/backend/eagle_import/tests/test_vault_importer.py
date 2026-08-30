"""Tests for eagle_import.vault_importer - the read+write half of Eagle
vault support (eagle_vault_parser is the read-only scan step, already
tested indirectly via routes/eagle_import.py's /scan endpoint)."""
import json

import pytest
from PIL import Image
from sqlalchemy import select

from database import Asset, Board, BoardItem, BoardSection, MediaItem, MediaTag, Tag
from eagle_import.eagle_vault_parser import EagleVaultError
from eagle_import.vault_importer import import_vault

PROFILE_ID = "test-profile"


@pytest.fixture(autouse=True)
def _isolated_managed_storage(tmp_path, monkeypatch):
    # stage_managed_media writes into app_dirs.get_profile_dir(profile_id),
    # which defaults to a real OS app-data path - redirect it into tmp_path
    # so tests never touch (or depend on) the real user data directory.
    monkeypatch.setenv("ARTCRAFT_STUDIO_DATA_DIR", str(tmp_path / "app_data"))


def _write_eagle_item(images_dir, *, item_id, name="photo", ext="png", tags=None,
                       folders=None, annotation="", is_deleted=False, width=8, height=6):
    info_dir = images_dir / f"{item_id}.info"
    info_dir.mkdir()
    Image.new("RGB", (width, height), (10, 20, 30)).save(info_dir / f"{name}.{ext}", format="PNG")
    metadata = {
        "id": item_id,
        "name": name,
        "ext": ext,
        "size": 1234,
        "tags": tags or [],
        "folders": folders or [],
        "annotation": annotation,
        "isDeleted": is_deleted,
        "width": width,
        "height": height,
        "animated": False,
        "palettes": [],
    }
    (info_dir / "metadata.json").write_text(json.dumps(metadata), encoding="utf-8")


def _write_vault(vault_path, *, folders=None, items=()):
    vault_path.mkdir(parents=True, exist_ok=True)
    (vault_path / "metadata.json").write_text(json.dumps({"folders": folders or []}), encoding="utf-8")
    images_dir = vault_path / "images"
    images_dir.mkdir()
    for item in items:
        _write_eagle_item(images_dir, **item)


@pytest.mark.asyncio
async def test_imports_a_basic_item(session, tmp_path):
    vault_path = tmp_path / "MyLibrary.library"
    _write_vault(vault_path, items=[{"item_id": "ITEM00000001", "tags": ["cat", "portrait"]}])

    result = await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)

    assert result.imported == 1
    assert result.already_imported == 0
    assert result.skipped_error == 0

    media = (await session.scalars(select(MediaItem))).all()
    assert len(media) == 1
    assert media[0].width == 8
    assert media[0].height == 6
    assert media[0].metadata_status == "completed"

    asset = (await session.scalars(select(Asset))).one()
    assert asset.origin_type == "eagle_import"
    assert asset.current_revision_id is not None

    tags = {t.tag_text for t in (await session.scalars(select(Tag))).all()}
    assert tags == {"cat", "portrait"}
    media_tags = (await session.scalars(select(MediaTag))).all()
    assert len(media_tags) == 2


@pytest.mark.asyncio
async def test_annotation_becomes_extracted_prompt(session, tmp_path):
    vault_path = tmp_path / "MyLibrary.library"
    _write_vault(vault_path, items=[{"item_id": "ITEM00000001", "annotation": "a cat in the sun"}])

    await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)

    media = (await session.scalars(select(MediaItem))).one()
    assert media.extracted_prompt == "a cat in the sun"


@pytest.mark.asyncio
async def test_deleted_items_are_skipped_by_default(session, tmp_path):
    vault_path = tmp_path / "MyLibrary.library"
    _write_vault(vault_path, items=[
        {"item_id": "ITEM00000001", "is_deleted": True},
        {"item_id": "ITEM00000002", "is_deleted": False},
    ])

    result = await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)

    assert result.imported == 1
    assert result.skipped_deleted == 1
    assert len((await session.scalars(select(MediaItem))).all()) == 1


@pytest.mark.asyncio
async def test_missing_source_file_is_skipped_not_fatal(session, tmp_path):
    vault_path = tmp_path / "MyLibrary.library"
    _write_vault(vault_path, items=[
        {"item_id": "ITEM00000001"},
        {"item_id": "ITEM00000002"},
    ])
    # Delete one item's original file after writing its metadata, simulating
    # a vault with a missing/moved source - the parser already tolerates
    # this (see eagle_vault_parser docstring); the importer must too.
    (vault_path / "images" / "ITEM00000002.info" / "photo.png").unlink()

    result = await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)

    assert result.imported == 1
    assert result.skipped_missing_file == 1
    assert result.skipped_error == 0


@pytest.mark.asyncio
async def test_rerunning_the_same_vault_does_not_duplicate(session, tmp_path):
    vault_path = tmp_path / "MyLibrary.library"
    _write_vault(vault_path, items=[{"item_id": "ITEM00000001", "tags": ["cat"]}])

    first = await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)
    second = await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)

    assert first.imported == 1
    assert second.imported == 0
    assert second.already_imported == 1
    assert len((await session.scalars(select(MediaItem))).all()) == 1
    assert len((await session.scalars(select(Asset))).all()) == 1
    # The tag itself must be reused too, not duplicated (Tag.tag_text is
    # unique) - re-running should not error trying to insert it again.
    assert len((await session.scalars(select(Tag))).all()) == 1


@pytest.mark.asyncio
async def test_multi_folder_membership_creates_one_board_per_folder(session, tmp_path):
    vault_path = tmp_path / "MyLibrary.library"
    _write_vault(
        vault_path,
        folders=[
            {"id": "F1", "name": "Portraits", "children": []},
            {"id": "F2", "name": "Favorites", "children": []},
        ],
        items=[{"item_id": "ITEM00000001", "folders": ["F1", "F2"]}],
    )

    result = await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)

    assert result.boards_touched == 2
    boards = {b.name for b in (await session.scalars(select(Board))).all()}
    assert boards == {"Eagle: Portraits", "Eagle: Favorites"}

    sections = (await session.scalars(select(BoardSection))).all()
    assert len(sections) == 2
    board_items = (await session.scalars(select(BoardItem))).all()
    assert len(board_items) == 2  # same media item, one row per board


@pytest.mark.asyncio
async def test_rerun_reuses_existing_board_by_name(session, tmp_path):
    vault_path = tmp_path / "MyLibrary.library"
    _write_vault(
        vault_path,
        folders=[{"id": "F1", "name": "Portraits", "children": []}],
        items=[
            {"item_id": "ITEM00000001", "folders": ["F1"]},
            {"item_id": "ITEM00000002", "folders": ["F1"]},
        ],
    )

    await import_vault(session, profile_id=PROFILE_ID, vault_path=vault_path)

    boards = (await session.scalars(select(Board))).all()
    assert len(boards) == 1  # not one board per item sharing the folder


@pytest.mark.asyncio
async def test_rejects_a_folder_that_is_not_a_real_vault(session, tmp_path):
    not_a_vault = tmp_path / "just_a_folder"
    not_a_vault.mkdir()

    with pytest.raises(EagleVaultError):
        await import_vault(session, profile_id=PROFILE_ID, vault_path=not_a_vault)
