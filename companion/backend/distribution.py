"""
Build distribution flag.

``ARTCRAFT_STUDIO_DISTRIBUTION`` is a compile-time constant threaded through all
three layers of the app (Vue via Vite define, Rust/Tauri via build-time
env, and the packaged Python backend via env baked into the launcher at
packaging time). The default everywhere is ``dev``; only release CI sets
``official``.

It gates the telemetry client, the consent UI, and the update check.
It does NOT gate the feature-flags fetch or cloud sign-in/API.

There is no secret involved — the flag is honest configuration, visible
in the open-source tree; what makes a build "official" is simply that
release CI built it.
"""
import os

DISTRIBUTION_DEV = "dev"
DISTRIBUTION_OFFICIAL = "official"


def get_distribution() -> str:
    """Return the build distribution: ``dev`` (default) or ``official``.

    Resolution order:
    1. ``ARTCRAFT_STUDIO_DISTRIBUTION`` environment variable (set by the Tauri shell
       from its own compile-time constant, or baked into the packaged
       backend's launcher script at build time).
    2. Legacy ``STIMMA_DISTRIBUTION`` when the new variable is absent.
    3. ``dev``.
    """
    value = os.environ.get("ARTCRAFT_STUDIO_DISTRIBUTION")
    if value is None:
        value = os.environ.get("STIMMA_DISTRIBUTION", "")
    value = value.strip().lower()
    if value == DISTRIBUTION_OFFICIAL:
        return DISTRIBUTION_OFFICIAL
    return DISTRIBUTION_DEV


def is_official() -> bool:
    """True only for builds produced by release CI."""
    return get_distribution() == DISTRIBUTION_OFFICIAL


def is_privacy_lockdown() -> bool:
    """True when ``ARTCRAFT_STUDIO_PRIVACY_LOCKDOWN=1`` is set."""
    from privacy_lockdown import is_privacy_lockdown_enabled

    return is_privacy_lockdown_enabled()
