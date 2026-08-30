import httpx
import pytest
from starlette.applications import Starlette
from starlette.responses import JSONResponse
from starlette.routing import Route

import core.companion_auth_middleware as companion_auth_middleware
from core.companion_auth_middleware import AUTH_HEADER, TOKEN_ENV_VAR, CompanionAuthMiddleware

TEST_TOKEN = "test-shared-secret"


async def _ok_handler(request):
    return JSONResponse({"ok": True})


def _make_client():
    # Starlette 0.27.0's TestClient (starlette.testclient.TestClient) wraps
    # `httpx.Client` with an `app=` kwarg that httpx removed in 0.28 - and
    # this repo pins httpx==0.28.1 alongside starlette==0.27.0, so that
    # TestClient is broken outright against this project's actual pinned
    # versions (raises `TypeError: Client.__init__() got an unexpected
    # keyword argument 'app'`), independent of anything under test here.
    # httpx.AsyncClient + ASGITransport is the httpx>=0.28-native way to
    # drive an ASGI app directly and isn't affected.
    app = Starlette(routes=[Route("/", _ok_handler)])
    app.add_middleware(CompanionAuthMiddleware)
    return httpx.AsyncClient(transport=httpx.ASGITransport(app=app), base_url="http://testserver")


async def _run_websocket_request(headers):
    events = []

    async def app(scope, receive, send):
        events.append({"type": "app_called"})

    scope = {
        "type": "websocket",
        "asgi": {"version": "3.0"},
        "path": "/ws",
        "raw_path": b"/ws",
        "query_string": b"",
        "headers": [(key.lower().encode(), value.encode()) for key, value in headers.items()],
        "client": ("127.0.0.1", 1234),
        "server": ("127.0.0.1", 8000),
        "subprotocols": [],
    }

    async def receive():
        return {"type": "websocket.connect"}

    async def send(message):
        events.append(message)

    middleware = CompanionAuthMiddleware(app)
    await middleware(scope, receive, send)
    return events


@pytest.fixture(autouse=True)
def _reset_warned_once_flag(monkeypatch):
    # `_warned_no_token_configured` is module-level state that would
    # otherwise leak across tests within the same pytest process and make
    # the "warns exactly once" test order-dependent.
    monkeypatch.setattr(companion_auth_middleware, "_warned_no_token_configured", False)


@pytest.mark.asyncio
async def test_request_with_matching_token_is_allowed(monkeypatch):
    monkeypatch.setenv(TOKEN_ENV_VAR, TEST_TOKEN)

    async with _make_client() as client:
        response = await client.get("/", headers={AUTH_HEADER: TEST_TOKEN})

    assert response.status_code == 200
    assert response.json() == {"ok": True}


@pytest.mark.asyncio
async def test_request_with_missing_header_is_rejected(monkeypatch):
    monkeypatch.setenv(TOKEN_ENV_VAR, TEST_TOKEN)

    async with _make_client() as client:
        response = await client.get("/")

    assert response.status_code == 401
    assert response.json() == {"detail": "unauthorized"}


@pytest.mark.asyncio
async def test_request_with_wrong_token_is_rejected(monkeypatch):
    monkeypatch.setenv(TOKEN_ENV_VAR, TEST_TOKEN)

    async with _make_client() as client:
        response = await client.get("/", headers={AUTH_HEADER: "wrong-token"})

    assert response.status_code == 401


@pytest.mark.asyncio
async def test_empty_header_is_rejected_even_though_expected_token_is_falsy_checked(monkeypatch):
    # Regression guard: `provided_token != expected_token` must reject an
    # empty string, not accidentally treat "" as "not provided" and fall
    # through to some other allow path.
    monkeypatch.setenv(TOKEN_ENV_VAR, TEST_TOKEN)

    async with _make_client() as client:
        response = await client.get("/", headers={AUTH_HEADER: ""})

    assert response.status_code == 401


@pytest.mark.asyncio
async def test_fails_open_when_token_env_var_is_unset(monkeypatch):
    # Dev-from-source mode (no Tauri supervisor spawned this process) - see
    # module docstring. No header at all should still be allowed through.
    monkeypatch.delenv(TOKEN_ENV_VAR, raising=False)

    async with _make_client() as client:
        response = await client.get("/")

    assert response.status_code == 200


@pytest.mark.asyncio
async def test_warns_only_once_when_token_env_var_is_unset(monkeypatch, caplog):
    monkeypatch.delenv(TOKEN_ENV_VAR, raising=False)

    async with _make_client() as client:
        with caplog.at_level("WARNING"):
            await client.get("/")
            await client.get("/")
            await client.get("/")

    warnings = [r for r in caplog.records if "Companion auth is DISABLED" in r.message]
    assert len(warnings) == 1


@pytest.mark.asyncio
async def test_websocket_without_token_is_rejected(monkeypatch):
    monkeypatch.setenv(TOKEN_ENV_VAR, TEST_TOKEN)

    events = await _run_websocket_request({})

    assert events == [{"type": "websocket.close", "code": 1008, "reason": "unauthorized"}]


@pytest.mark.asyncio
async def test_websocket_with_matching_token_reaches_the_app(monkeypatch):
    monkeypatch.setenv(TOKEN_ENV_VAR, TEST_TOKEN)

    events = await _run_websocket_request({AUTH_HEADER: TEST_TOKEN})

    assert events == [{"type": "app_called"}]
