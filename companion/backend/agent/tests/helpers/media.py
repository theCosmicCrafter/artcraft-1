"""
Test helpers for creating `MediaItem` rows and backing image files.

Missing from the initial Companion port (5 test files under agent/tests/
import from here but the module itself never got carried over - see
.github/workflows/companion-ci.yml's header comment and
artcraft-studio-plan/SPRINT_PLAN.md). Two distinct, independently-usable
helpers, matching how the test files that import them actually use them:

- `create_media_item` is a pure DB-row factory. It never touches disk and
  never requires `file_path` to point at a real file - several call sites
  (agent/tests/test_show_artifact_revisions.py, agent/tests/test_v2_library_tool.py)
  create MediaItem rows purely for their id/relationships and never read the
  file back.
- `generate_test_image` actually writes a real PNG to disk (for tests that
  exercise code paths which read the file, e.g. agent/tests/test_run_code_v2.py's
  run_code tool) and returns its real SHA256 hash via the same
  `media_scanner.compute_file_hash` production code uses, so a test's
  file_hash means the same thing it would for a real ingested file.
"""
from pathlib import Path
from typing import Optional
from uuid import uuid4

from PIL import Image

from database import MediaItem
from media_scanner import compute_file_hash

DEFAULT_WIDTH = 16
DEFAULT_HEIGHT = 16
DEFAULT_COLOR = (128, 128, 128)


def generate_test_image(
    path: Path,
    width: int = DEFAULT_WIDTH,
    height: int = DEFAULT_HEIGHT,
    color: tuple = DEFAULT_COLOR,
) -> str:
    """Write a real PNG to `path` (creating parent dirs as needed) and return
    its SHA256 hash, computed the same way production ingestion does."""
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.new("RGB", (width, height), color).save(path, format="PNG")
    return compute_file_hash(path)


async def create_media_item(
    session,
    *,
    file_path: Optional[Path] = None,
    file_hash: Optional[str] = None,
    width: Optional[int] = None,
    height: Optional[int] = None,
    tool_id: Optional[str] = None,
    vlm_caption: Optional[str] = None,
    extracted_prompt: Optional[str] = None,
    generation_metadata: Optional[str] = None,
    **overrides,
) -> MediaItem:
    """Insert and return a `MediaItem` row with sane defaults for every
    NOT NULL column the caller doesn't care about for a given test.

    Does not require `file_path` to exist on disk - if it does (e.g. written
    by `generate_test_image` first), its real size is used; otherwise a
    placeholder is used, since most callers only need the row to exist, not
    a byte-accurate file_size.
    """
    if file_path is None:
        file_path = Path(f"/tmp/stimma-test/synthetic-{uuid4().hex}.png")
    if file_hash is None:
        file_hash = uuid4().hex
    if width is None:
        width = DEFAULT_WIDTH
    if height is None:
        height = DEFAULT_HEIGHT

    file_size = file_path.stat().st_size if file_path.exists() else 1024
    file_format = file_path.suffix.lstrip(".").lower() or "png"

    item = MediaItem(
        file_path=str(file_path),
        file_hash=file_hash,
        file_size=file_size,
        file_format=file_format,
        width=width,
        height=height,
        megapixels=(width * height) / 1_000_000,
        tool_id=tool_id,
        vlm_caption=vlm_caption,
        extracted_prompt=extracted_prompt,
        generation_metadata=generation_metadata,
        **overrides,
    )
    session.add(item)
    await session.commit()
    await session.refresh(item)
    return item
