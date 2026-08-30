"""Tests for the Omni API STP provider.

These tests use a mock httpx.AsyncClient to exercise the discovery, execute,
and asset-upload paths without hitting a real Omni API endpoint.
"""

from __future__ import annotations

import asyncio
from typing import Any, Dict, Optional
from unittest.mock import AsyncMock, MagicMock

import pytest

from providers.omni_api_provider import OmniApiProvider


class MockResponse:
    def __init__(
        self,
        json_data: Optional[Dict[str, Any]] = None,
        content: bytes = b"",
        status_code: int = 200,
    ) -> None:
        self._json = json_data or {}
        self.content = content
        self.status_code = status_code

    def raise_for_status(self) -> None:
        if self.status_code >= 400:
            raise RuntimeError(f"HTTP {self.status_code}")

    def json(self) -> Dict[str, Any]:
        return self._json


class MockHttpClient:
    """A minimal mock that records calls and returns scripted responses."""

    def __init__(self, responses: Dict[str, list[MockResponse]]) -> None:
        # responses maps URL → list of queued responses (FIFO)
        self._responses = {k: list(v) for k, v in responses.items()}
        self.calls: list[tuple[str, str, dict[str, Any]]] = []

    def _next(self, url: str) -> MockResponse:
        queue = self._responses.get(url)
        if not queue:
            return MockResponse({"error": f"no queued response for {url}"}, status_code=404)
        return queue.pop(0)

    async def get(self, url: str, headers: Optional[Dict[str, str]] = None, **kw: Any) -> MockResponse:
        self.calls.append(("GET", url, headers or {}))
        return self._next(url)

    async def post(self, url: str, headers: Optional[Dict[str, str]] = None, **kw: Any) -> MockResponse:
        self.calls.append(("POST", url, headers or {}))
        return self._next(url)

    async def aclose(self) -> None:
        pass


# ---------------------------------------------------------------------------
# Test fixtures
# ---------------------------------------------------------------------------

IMAGE_MODELS_RESPONSE = {
    "success": True,
    "models": [
        {
            "model": "flux_1_dev",
            "full_name": "FLUX.1 [dev]",
            "model_creator": "BlackForestLabs",
            "text_prompt_supported": True,
            "aspect_ratio_options": ["square", "wide_sixteen_by_nine"],
            "aspect_ratio_default": "square",
            "batch_size_min": 1,
            "batch_size_max": 4,
            "batch_size_default": 4,
        },
        {
            "model": "midjourney_8",
            "full_name": "Midjourney v8",
            "model_creator": "Midjourney",
            "text_prompt_supported": True,
            "image_refs_supported": True,
            "image_refs_max": 1,
            "aspect_ratio_options": ["auto", "square"],
            "aspect_ratio_default": "square",
            "batch_size_min": 1,
            "batch_size_max": 2,
            "batch_size_default": 1,
        },
    ],
    "providers": [],
}

VIDEO_MODELS_RESPONSE = {
    "success": True,
    "models": [
        {
            "model": "seedance_2p0_fast_g",
            "full_name": "Seedance 2.0 Fast Global",
            "model_creator": "Bytedance",
            "text_prompt_supported": True,
            "aspect_ratio_options": ["wide_sixteen_by_nine"],
            "aspect_ratio_default": "wide_sixteen_by_nine",
        },
    ],
    "providers": [],
}

GENERATE_RESPONSE = {
    "success": True,
    "inference_job_token": "jinf_test123",
}

JOB_COMPLETED = {
    "status": "completed",
    "result_url": "https://cdn.example.com/output.png",
}


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
async def test_connect_discovers_image_and_video_tools() -> None:
    base = "https://api.test.local"
    client = MockHttpClient({
        f"{base}/v1/omni_gen/models/image": [MockResponse(IMAGE_MODELS_RESPONSE)],
        f"{base}/v1/omni_gen/models/video": [MockResponse(VIDEO_MODELS_RESPONSE)],
        f"{base}/v1/omni_gen/models/audio": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/mesh": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/splat": [MockResponse({"models": []})],
    })

    provider = OmniApiProvider(api_key="key_test", base_url=base, http_client=client)
    await provider.connect()

    tools = await provider.list_tools()
    tool_ids = {t.id for t in tools}
    assert "image:flux_1_dev" in tool_ids
    assert "image:midjourney_8" in tool_ids
    assert "video:seedance_2p0_fast_g" in tool_ids
    assert provider.status.value == "connected"
    await provider.disconnect()


@pytest.mark.asyncio
async def test_tool_descriptor_has_correct_schema() -> None:
    base = "https://api.test.local"
    client = MockHttpClient({
        f"{base}/v1/omni_gen/models/image": [MockResponse(IMAGE_MODELS_RESPONSE)],
        f"{base}/v1/omni_gen/models/video": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/audio": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/mesh": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/splat": [MockResponse({"models": []})],
    })

    provider = OmniApiProvider(api_key="key_test", base_url=base, http_client=client)
    await provider.connect()

    flux = next(t for t in await provider.list_tools() if t.id == "image:flux_1_dev")
    assert flux.task_type == "text-to-image"
    assert flux.model == "flux_1_dev"
    assert flux.model_vendor == "BlackForestLabs"
    props = flux.parameter_schema["properties"]
    assert "prompt" in props
    assert "aspect_ratio" in props
    assert props["aspect_ratio"]["enum"] == ["square", "wide_sixteen_by_nine"]
    assert "batch_size" in props  # batch_size_max > 1
    assert props["batch_size"]["maximum"] == 4

    mj = next(t for t in await provider.list_tools() if t.id == "image:midjourney_8")
    assert "image_refs" in mj.parameter_schema["properties"]
    assert mj.parameter_schema["properties"]["image_refs"]["maxItems"] == 1

    await provider.disconnect()


@pytest.mark.asyncio
async def test_execute_calls_generate_and_polls_job() -> None:
    base = "https://api.test.local"
    client = MockHttpClient({
        f"{base}/v1/omni_gen/models/image": [MockResponse(IMAGE_MODELS_RESPONSE)],
        f"{base}/v1/omni_gen/models/video": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/audio": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/mesh": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/splat": [MockResponse({"models": []})],
        f"{base}/v1/omni_api/generate/image": [MockResponse(GENERATE_RESPONSE)],
        f"{base}/v1/omni_api/job_status/jinf_test123": [MockResponse(JOB_COMPLETED)],
        "https://cdn.example.com/output.png": [MockResponse(content=b"\x89PNG\r\n\x1a\n")],
    })

    provider = OmniApiProvider(api_key="key_test", base_url=base, http_client=client)
    await provider.connect()

    results = []
    async for item in provider.execute(
        "image:flux_1_dev",
        {"prompt": "a corgi", "aspect_ratio": "square"},
    ):
        results.append(item)

    assert len(results) == 1
    result = results[0]
    assert result.success is True
    assert result.output_data == b"\x89PNG\r\n\x1a\n"
    assert result.metadata["job_token"] == "jinf_test123"
    assert result.metadata["model"] == "flux_1_dev"

    # Verify the generate call carried the auth header
    post_calls = [c for c in client.calls if c[0] == "POST"]
    assert any("Authorization" in c[2] and c[2]["Authorization"] == "Bearer key_test" for c in post_calls)

    await provider.disconnect()


@pytest.mark.asyncio
async def test_execute_unknown_tool_returns_error() -> None:
    base = "https://api.test.local"
    client = MockHttpClient({
        f"{base}/v1/omni_gen/models/image": [MockResponse(IMAGE_MODELS_RESPONSE)],
        f"{base}/v1/omni_gen/models/video": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/audio": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/mesh": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/splat": [MockResponse({"models": []})],
    })

    provider = OmniApiProvider(api_key="key_test", base_url=base, http_client=client)
    await provider.connect()

    results = []
    async for item in provider.execute("image:nonexistent", {"prompt": "test"}):
        results.append(item)

    assert len(results) == 1
    assert results[0].success is False
    assert "Unknown tool" in (results[0].error or "")

    await provider.disconnect()


@pytest.mark.asyncio
async def test_connect_tolerates_individual_category_failure() -> None:
    """A 500 on one category endpoint is logged but doesn't block connect."""
    base = "https://api.test.local"
    client = MockHttpClient({
        f"{base}/v1/omni_gen/models/image": [MockResponse(status_code=500)],
        f"{base}/v1/omni_gen/models/video": [MockResponse(VIDEO_MODELS_RESPONSE)],
        f"{base}/v1/omni_gen/models/audio": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/mesh": [MockResponse({"models": []})],
        f"{base}/v1/omni_gen/models/splat": [MockResponse({"models": []})],
    })

    provider = OmniApiProvider(api_key="key_test", base_url=base, http_client=client)
    await provider.connect()
    # Image failed, but video succeeded — provider is still connected
    assert provider.status.value == "connected"
    tools = await provider.list_tools()
    assert any(t.id == "video:seedance_2p0_fast_g" for t in tools)
    assert not any(t.id.startswith("image:") for t in tools)
    await provider.disconnect()


@pytest.mark.asyncio
async def test_connect_fails_when_all_categories_error() -> None:
    """If every category endpoint returns 500, connect raises ConnectionError."""
    base = "https://api.test.local"
    client = MockHttpClient({
        f"{base}/v1/omni_gen/models/image": [MockResponse(status_code=500)],
        f"{base}/v1/omni_gen/models/video": [MockResponse(status_code=500)],
        f"{base}/v1/omni_gen/models/audio": [MockResponse(status_code=500)],
        f"{base}/v1/omni_gen/models/mesh": [MockResponse(status_code=500)],
        f"{base}/v1/omni_gen/models/splat": [MockResponse(status_code=500)],
    })

    provider = OmniApiProvider(api_key="key_test", base_url=base, http_client=client)
    with pytest.raises(ConnectionError):
        await provider.connect()
    assert provider.status.value == "error"
