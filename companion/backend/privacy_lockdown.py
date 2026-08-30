"""Hard privacy lockdown for ArtCraft's cloud network surfaces.

``ARTCRAFT_STUDIO_PRIVACY_LOCKDOWN=1`` is stronger than the telemetry consent toggle:
the app must not contact ArtCraft's cloud backend, send install identifiers, sign in
to the cloud account system, or buffer volunteerable data for later submission.
"""

from __future__ import annotations

import os
from urllib.parse import urlparse

PRIMARY_ENV = "ARTCRAFT_STUDIO_PRIVACY_LOCKDOWN"

_TRUTHY = {"1", "true", "yes", "on"}


class PrivacyLockdownError(RuntimeError):
    """Raised when code attempts to use ArtCraft's cloud backend in lockdown."""


def _env_truthy(name: str) -> bool:
    return os.environ.get(name, "").strip().lower() in _TRUTHY


def is_privacy_lockdown_enabled() -> bool:
    """True when the local-only privacy mode is active."""
    return _env_truthy(PRIMARY_ENV)


def active_env_name() -> str | None:
    """Return the env var currently enabling lockdown, if any."""
    if _env_truthy(PRIMARY_ENV):
        return PRIMARY_ENV
    return None


def disabled_message(feature: str = "ArtCraft cloud services") -> str:
    return f"{feature} unavailable while Privacy Lockdown is enabled."


def raise_if_enabled(feature: str = "ArtCraft cloud services") -> None:
    if is_privacy_lockdown_enabled():
        raise PrivacyLockdownError(disabled_message(feature))


def is_artcraft_cloud_url(url: str | None) -> bool:
    """Return True for ArtCraft's own cloud backend hosts (storyteller.ai)."""
    if not url:
        return False
    try:
        host = (urlparse(url).hostname or "").lower()
    except Exception:
        return False
    return host == "storyteller.ai" or host.endswith(".storyteller.ai")


def raise_for_artcraft_cloud_url_if_enabled(url: str | None, feature: str = "ArtCraft cloud services") -> None:
    if is_privacy_lockdown_enabled() and is_artcraft_cloud_url(url):
        raise PrivacyLockdownError(disabled_message(feature))


def is_model_cdn_url(url: str | None) -> bool:
    """Return True for the third-party model-weight CDN this app still depends
    on (models.stimma.ai - a Stimma-operated R2 mirror of open model weights,
    unrelated to ArtCraft's own cloud backend; see model_cache.py for why this
    dependency is kept rather than rebranded). Kept as its own check, distinct
    from is_artcraft_cloud_url, so Privacy Lockdown still blocks contacting it."""
    if not url:
        return False
    try:
        host = (urlparse(url).hostname or "").lower()
    except Exception:
        return False
    return host == "stimma.ai" or host.endswith(".stimma.ai")


def raise_for_model_cdn_url_if_enabled(url: str | None, feature: str = "model downloads") -> None:
    if is_privacy_lockdown_enabled() and is_model_cdn_url(url):
        raise PrivacyLockdownError(disabled_message(feature))
