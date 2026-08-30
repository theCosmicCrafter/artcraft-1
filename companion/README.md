# ArtCraft Studio — Companion Service

A local Python/FastAPI service, bundled with the ArtCraft Studio desktop app, responsible for
local media library indexing/semantic search, the agent copilot, Flows, Darkroom, and
ComfyUI/STP orchestration. It does not handle accounts, billing, or generation-provider
routing — those stay with ArtCraft's existing Rust cloud backend (`storyteller-web`), unchanged.

Full architecture: [`../../artcraft-studio-plan/SYSTEM_DESIGN.md`](../../artcraft-studio-plan/SYSTEM_DESIGN.md).

## Origin

This source tree was copied directly from Stimma's `stimma-backend/backend/` (a separate,
unrelated product being merged into ArtCraft Studio — see the planning package for the full
rationale). The original Stimma installation was **not modified** by this copy.

**Not copied**: `stimma-backend/python/` (Stimma's embeddable CPython runtime — a ~13,600-file
binary distribution, not source code; the build pipeline provisions its own Python runtime
instead of vendoring one, see [`../../artcraft-studio-plan/DEVOPS.md`](../../artcraft-studio-plan/DEVOPS.md)).

## Adaptation status

Per the file-by-file migration strategy in SYSTEM_DESIGN.md §7:

- **Carried over as-is**: most of the ML/orchestration core (`clip_service.py`,
  `face_detection_service.py`, `providers/`, `flow_dsl/`, `flow_runtime/`, `database.py`,
  `database_registry.py`, etc.) — no changes needed structurally, though nothing has been
  test-run end-to-end in this new location yet (no MySQL/live-service integration test done).
- **Done**: `main.py` (port-announce marker renamed to `ARTCRAFT_STUDIO_COMPANION_PORT=`,
  bundle-id CLI default, service name in the health-check response), `core/app.py` (CORS
  origin updated to this app's actual Vite dev port; `CompanionAuthMiddleware` wired in as the
  shared-secret handshake enforcement — see `core/companion_auth_middleware.py`),
  `app_context.py`/`app_dirs.py` (bundle-ID constants and `ARTCRAFT_STUDIO_DATA_DIR`/
  `ARTCRAFT_STUDIO_CACHE_DIR` env vars renamed; `stimma_v1.db` → `artcraft_studio_v1.db`;
  `get_all_stimma_owned_roots()` → `get_all_companion_owned_roots()`), `user_agent.py` (UA
  string + `ARTCRAFT_STUDIO_APP_VERSION` env var), `cloud_runtime.py`/`config.py` (cloud base
  URL repointed from `stimma.ai` to `api.storyteller.ai`), `privacy_lockdown.py` (domain checks
  split into `is_artcraft_cloud_url` for ArtCraft's backend and a separate `is_model_cdn_url`
  for the still-external `models.stimma.ai` dependency — see `model_cache.py`'s docstring for
  why that one's deliberately kept), `llm.py`'s `_inject_local_system()` (the resolved
  no-product-censorship-on-local-LLM decision — content-policy injection removed entirely, it
  was already exclusively wired to the local-endpoint path).
- **Needs adaptation** (not started): `generation_queue.py` (replace the
  Stimma-Cloud-as-provider path with the new `omni_api_provider.py`, not yet written — Phase 2).
- **To be replaced, not adapted** (not started): `firebase_auth.py`, `auth_storage.py`,
  `routes/auth.py` (drop entirely — the Companion will trust a session token handed down by
  the Tauri shell instead); `account_sync.py`, `routes/cloud.py`, the billing/402-handling in
  `cloud_api.py`/`cloud_events.py` (drop — no independent billing concept survives the merge,
  see SYSTEM_DESIGN.md §7); `stimpack_marketplace.py` (Phase 3, deferred).
- **Not yet done**: the cosmetic sweep of ~1200 remaining "Stimma" references in comments/
  docstrings/log messages across the tree (functionally inert, tracked as a separate low-priority
  backlog item — every functionally load-bearing identifier is already renamed and verified).

## Eagle.cool vault import

`eagle_import/eagle_vault_parser.py` (parser) + `routes/eagle_import.py`
(`POST /api/eagle_import/scan`) implement the Phase 1 pre-flight scan step
(UI_UX_PLAN.md §4). Verified against the real reference library at
`F:\AIPrograms\Ai images.library`, not just synthetic data:

- 35,489 items, 235 folders, 25,086 distinct tags, 41 deleted items, 42 top-level folders —
  all correctly parsed, including multi-folder membership and Unicode folder names (`Niš
  Fortress` round-trips correctly; an earlier console `print()` mangling it to `Ni? Fortress`
  was confirmed to be a Windows-terminal display artifact, not a parsing bug, by writing the
  raw string to a UTF-8 file and checking its `\uXXXX` escape).
- A handful of items in this real library have empty/corrupt `metadata.json` files in
  practice — the parser logs and skips them per-item rather than aborting the whole scan, which
  is exactly the resilience this was designed for, now confirmed against real messy data rather
  than just the happy path.

**Implemented:** the read+write import path in `eagle_import/vault_importer.py` maps parsed
`EagleItem` rows into the Companion's real `Asset`/`AssetRevision`/`MediaItem` model, copies
source bytes into managed storage without deleting the Eagle originals, preserves tags and
multi-folder membership, and skips already-imported items on re-run. The `/api/eagle_import/import`
route wakes the existing ingestion worker so local CLIP/caption processing continues after the
copy. The remaining gap is the user-facing import wizard and mapping/progress UI.

## Running locally

`run.cmd`/`run.sh` launch the bundled interpreter layout used by release builds and set the
`ARTCRAFT_STUDIO_DISTRIBUTION` flag. For local dev/testing, run
`python backend/main.py --bundle-id ai.artcraftstudio.app.debug --sandbox default --port 0`
directly with a real Python 3.11+ interpreter after installing the pinned requirements, or use
the Rust supervisor (`crates/desktop/artcraft/src/core/companion/companion_launch_config.rs`),
which resolves the packaged resource first and falls back to source mode. Its
`ARTCRAFT_STUDIO_COMPANION_PYTHON` override remains available when `python`/`python3` does not
resolve to a real interpreter on PATH.

## Testing

From `companion/backend/`, the complete suite now collects and passes locally: **332 passed**
on Python 3.12 with the pinned runtime and development dependencies. The test harness fixes that
made the full suite runnable are:

- `conftest.py` (new, top-level) / `agent/tests/conftest.py` (edited) — pytest 8+ requires
  `pytest_plugins` registrations to live in the rootdir conftest, not a nested one.
- `pytest.ini` (new) — `pythonpath = agent`, since `agent/tests/*.py` import test helpers as
  `from tests.helpers...` (not `agent.tests.helpers...`), which only resolves with `agent/`
  itself on `sys.path`.
- `agent/tests/helpers/media.py` (new) — supplies the media fixtures used by the previously
  uncollectible tests.
- Windows-specific workspace-jail and path-separator fixes were verified through the full run;
  relative sandbox glob results now use POSIX separators for agent portability.

CI: `.github/workflows/companion-ci.yml` runs the complete pytest suite as a blocking check and
keeps the pinned Ruff check informational while the inherited judgment-heavy lint backlog is
burned down. This machine verified Python 3.12; the workflow's exact Python 3.11 run remains an
open cross-environment verification item.
