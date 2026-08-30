"""Enforces the Tauri-shell <-> Companion shared-secret handshake.

CORS origin-checking alone (Stimma's original design) is not authentication -
it only restricts what a *browser* is allowed to read from a cross-origin
response, and does nothing for non-browser local clients or DNS-rebinding
attacks. See artcraft-studio-plan/SYSTEM_DESIGN.md §1 for the full threat
model this closes.

The Rust supervisor (crates/desktop/artcraft/src/core/companion/supervisor.rs)
generates a random token per spawn and passes it via the
ARTCRAFT_STUDIO_COMPANION_AUTH_TOKEN environment variable - never over the
network, so nothing local can sniff it in transit. Every request must then
carry that same token in the X-ArtCraftStudio-Companion-Auth header (see
supervisor.rs's AUTH_HEADER_NAME constant - keep both in sync).
"""
import os

from starlette.datastructures import Headers
from starlette.responses import JSONResponse
from starlette.types import ASGIApp, Receive, Scope, Send

from core.logging import get_logger

log = get_logger(__name__)

AUTH_HEADER = "X-ArtCraftStudio-Companion-Auth"
TOKEN_ENV_VAR = "ARTCRAFT_STUDIO_COMPANION_AUTH_TOKEN"

_warned_no_token_configured = False


def _warn_auth_disabled() -> None:
    global _warned_no_token_configured
    if _warned_no_token_configured:
        return
    log.warning(
        f"{TOKEN_ENV_VAR} is not set - Companion auth is DISABLED. "
        "Expected when running from source without the Tauri shell; "
        "should never happen in a real spawned/packaged Companion."
    )
    _warned_no_token_configured = True


class CompanionAuthMiddleware:
    """Require the supervisor-issued token for HTTP and WebSocket scopes."""

    def __init__(self, app: ASGIApp) -> None:
        self.app = app

    async def __call__(self, scope: Scope, receive: Receive, send: Send) -> None:
        if scope["type"] not in {"http", "websocket"}:
            await self.app(scope, receive, send)
            return

        expected_token = os.environ.get(TOKEN_ENV_VAR)
        if not expected_token:
            _warn_auth_disabled()
            await self.app(scope, receive, send)
            return

        provided_token = Headers(scope=scope).get(AUTH_HEADER)
        if provided_token != expected_token:
            path = scope.get("path", "")
            log.warning(f"rejected request to {path}: missing/invalid {AUTH_HEADER}")
            if scope["type"] == "websocket":
                await send({
                    "type": "websocket.close",
                    "code": 1008,
                    "reason": "unauthorized",
                })
            else:
                response = JSONResponse(status_code=401, content={"detail": "unauthorized"})
                await response(scope, receive, send)
            return

        await self.app(scope, receive, send)
