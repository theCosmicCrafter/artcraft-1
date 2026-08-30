"""
Omni API Provider — wraps the ArtCraft cloud Omni API as a first-class STP
tool source.

Each Omni-supported model (image, video, audio, mesh, splat) becomes a
`ToolDescriptor` with a JSON-Schema parameter set derived from the Omni
API's own `/v1/omni_gen/models/*` endpoints.  The agent (Phase 2) and Flows
(Phase 2) can call *any* generation backend — local ComfyUI, an in-process
lightweight tool, or a paid Omni API model — through the same `execute()`
interface, with cost/wallet checks happening naturally at the Omni API call
boundary.

This provider is additive Python-only work — it does not touch
`artcraft_router` or the Omni API's Rust code at all.

See SYSTEM_DESIGN.md §3 ("Omni API as STP tool source") for the design
rationale.
"""

from __future__ import annotations

import asyncio
import logging
import os
import uuid
from typing import Any, AsyncIterator, Callable, Dict, List, Optional

import httpx

from .base import (
    ExecutionProgress,
    ExecutionResult,
    ProviderStatus,
    ToolDescriptor,
    ToolProvider,
)
from cloud_runtime import effective_cloud_base_url, with_cloud_access_headers

logger = logging.getLogger(__name__)

PROVIDER_ID = "omni-api"
PROVIDER_NAME = "ArtCraft Omni API"
PROVIDER_TYPE = "cloud"

# Omni API model categories → generate endpoint suffix.
MODEL_CATEGORIES = ("image", "video", "audio", "mesh", "splat")

# Polling interval for async job status.
JOB_POLL_INTERVAL_SECONDS = 2.0
JOB_POLL_TIMEOUT_SECONDS = 600.0

# A token provider is an async callable that returns a valid Firebase ID token
# (or None if the user is not authenticated).  When supplied, the Omni API
# provider uses it instead of a static API key — this is the Phase 2
# Companion↔cloud session-token auth path (SYSTEM_DESIGN.md §1).
TokenProvider = Callable[[], "asyncio.Future[Optional[str]]"]


class OmniApiProvider(ToolProvider):
    """A `ToolProvider` that wraps the ArtCraft Omni API.

    On `connect()`, fetches the model lists from `/v1/omni_gen/models/*`
    and builds one `ToolDescriptor` per model.  On `execute()`, calls the
    appropriate `/v1/omni_api/generate/*` endpoint (API-key authenticated)
    and polls `/v1/omni_api/job_status/{token}` until the job completes.
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        http_client: Optional[httpx.AsyncClient] = None,
        token_provider: Optional[TokenProvider] = None,
    ) -> None:
        self._api_key = api_key or os.environ.get("ARTCRAFT_STUDIO_OMNI_API_KEY", "")
        self._base_url = (base_url or effective_cloud_base_url()).rstrip("/")
        self._http = http_client
        self._owns_http = http_client is None
        self._token_provider = token_provider
        self._status = ProviderStatus.DISCONNECTED
        self._tools: Dict[str, ToolDescriptor] = {}

    # --- ToolProvider interface ---

    @property
    def provider_id(self) -> str:
        return PROVIDER_ID

    @property
    def provider_name(self) -> str:
        return PROVIDER_NAME

    @property
    def provider_type(self) -> str:
        return PROVIDER_TYPE

    @property
    def status(self) -> ProviderStatus:
        return self._status

    @property
    def max_concurrent(self) -> int:
        return 4

    async def connect(self) -> None:
        self._status = ProviderStatus.CONNECTING
        if self._http is None:
            self._http = httpx.AsyncClient(timeout=60.0)
            self._owns_http = True

        try:
            self._tools = await self._discover_tools()
            if not self._tools:
                # Every category endpoint failed — treat as a connection failure
                # rather than silently connecting with an empty tool set.
                raise RuntimeError(
                    f"No Omni API models discovered from {self._base_url} "
                    "(all category endpoints failed)"
                )
            self._status = ProviderStatus.CONNECTED
            logger.info(
                "OmniApiProvider connected: %d tools from %s",
                len(self._tools),
                self._base_url,
            )
        except Exception as exc:
            self._status = ProviderStatus.ERROR
            logger.error("OmniApiProvider connect failed: %s", exc)
            raise ConnectionError(f"Omni API connect failed: {exc}") from exc

    async def disconnect(self) -> None:
        self._tools.clear()
        self._status = ProviderStatus.DISCONNECTED
        if self._owns_http and self._http is not None:
            await self._http.aclose()
            self._http = None

    async def list_tools(self) -> List[ToolDescriptor]:
        return list(self._tools.values())

    async def execute(
        self,
        tool_id: str,
        parameters: Dict[str, Any],
        output_path: Optional[str] = None,
        progress_callback: Optional[Callable[[ExecutionProgress], None]] = None,
        request_id: Optional[str] = None,
    ) -> AsyncIterator[ExecutionProgress | ExecutionResult]:
        tool = self._tools.get(tool_id)
        if tool is None:
            yield ExecutionResult(success=False, error=f"Unknown tool: {tool_id}")
            return
        if self._status != ProviderStatus.CONNECTED or self._http is None:
            yield ExecutionResult(success=False, error="Provider not connected")
            return

        category = tool.metadata.get("omni_category", "image")
        generate_path = f"/v1/omni_api/generate/{category}"
        body = self._build_generate_body(tool, parameters)

        try:
            result_token = await self._post_generate(generate_path, body)
        except Exception as exc:
            yield ExecutionResult(success=False, error=str(exc))
            return

        if progress_callback:
            progress_callback(ExecutionProgress(progress=0.1, stage="submitted"))

        try:
            job_result = await self._poll_job(result_token, progress_callback)
        except Exception as exc:
            yield ExecutionResult(success=False, error=str(exc))
            return

        output_bytes = await self._download_result(job_result)

        if output_path and output_bytes:
            with open(output_path, "wb") as fh:
                fh.write(output_bytes)

        yield ExecutionResult(
            success=True,
            output_data=output_bytes,
            metadata={
                "job_token": result_token,
                "model": tool.model,
                "category": category,
            },
        )

    async def upload_asset(self, data: bytes, mime_type: str) -> str:
        if self._http is None or self._status != ProviderStatus.CONNECTED:
            raise RuntimeError("Provider not connected")

        category = _mime_to_category(mime_type)
        upload_path = f"/v1/omni_api/upload/{category}"
        headers = await self._auth_headers_async()
        files = {"file": (f"asset.{_mime_to_ext(mime_type)}", data, mime_type)}
        resp = await self._http.post(
            f"{self._base_url}{upload_path}",
            headers=headers,
            files=files,
        )
        resp.raise_for_status()
        payload = resp.json()
        asset_id = payload.get("media_token") or payload.get("asset_id") or ""
        if not asset_id:
            raise RuntimeError("Upload succeeded but no asset id returned")
        return asset_id

    async def download_asset(self, asset_id: str) -> bytes:
        if self._http is None or self._status != ProviderStatus.CONNECTED:
            raise RuntimeError("Provider not connected")
        headers = await self._auth_headers_async()
        resp = await self._http.get(
            f"{self._base_url}/v1/omni_api/asset/{asset_id}",
            headers=headers,
        )
        resp.raise_for_status()
        return resp.content

    # --- Internal helpers ---

    def _auth_headers(self) -> Dict[str, str]:
        headers: Dict[str, str] = {}
        if self._api_key:
            headers["Authorization"] = f"Bearer {self._api_key}"
        return with_cloud_access_headers(headers)

    async def _auth_headers_async(self) -> Dict[str, str]:
        """Build auth headers, resolving the token provider if one is set.

        When a `token_provider` is configured (Phase 2 session-token auth),
        it takes precedence over the static API key — the user's live Firebase
        ID token is used, with automatic refresh handled by `firebase_auth`.
        """
        headers: Dict[str, str] = {}
        if self._token_provider is not None:
            try:
                token = await self._token_provider()
                if token:
                    headers["Authorization"] = f"Bearer {token}"
                    return with_cloud_access_headers(headers)
            except Exception as exc:
                logger.warning("token_provider failed: %s — falling back to API key", exc)
        if self._api_key:
            headers["Authorization"] = f"Bearer {self._api_key}"
        return with_cloud_access_headers(headers)

    async def _discover_tools(self) -> Dict[str, ToolDescriptor]:
        tools: Dict[str, ToolDescriptor] = {}
        for category in MODEL_CATEGORIES:
            try:
                category_tools = await self._fetch_category_tools(category)
                tools.update(category_tools)
            except Exception as exc:
                logger.warning("Failed to fetch %s models: %s", category, exc)
        return tools

    async def _fetch_category_tools(self, category: str) -> Dict[str, ToolDescriptor]:
        if self._http is None:
            return {}
        url = f"{self._base_url}/v1/omni_gen/models/{category}"
        resp = await self._http.get(url, headers=with_cloud_access_headers({}))
        resp.raise_for_status()
        payload = resp.json()
        models = payload.get("models", [])
        task_type = _category_to_task_type(category)
        tools: Dict[str, ToolDescriptor] = {}
        for model_entry in models:
            model_name = model_entry.get("model", "")
            if not model_name:
                continue
            tool_id = f"{category}:{model_name}"
            tools[tool_id] = _build_descriptor(category, model_name, model_entry, task_type)
        return tools

    def _build_generate_body(self, tool: ToolDescriptor, parameters: Dict[str, Any]) -> Dict[str, Any]:
        body: Dict[str, Any] = {
            "model": tool.model,
            "idempotency_token": parameters.get("idempotency_token") or str(uuid.uuid4()),
        }
        for key in ("prompt", "negative_prompt", "aspect_ratio", "resolution", "quality",
                     "duration_seconds", "batch_size", "seed", "image_refs", "image_token",
                     "audio_token", "video_token"):
            if key in parameters and parameters[key] is not None:
                body[key] = parameters[key]
        return body

    async def _post_generate(self, path: str, body: Dict[str, Any]) -> str:
        assert self._http is not None
        headers = await self._auth_headers_async()
        headers["Content-Type"] = "application/json"
        resp = await self._http.post(
            f"{self._base_url}{path}",
            headers=headers,
            json=body,
        )
        resp.raise_for_status()
        payload = resp.json()
        token = payload.get("inference_job_token") or payload.get("job_token")
        if not token:
            raise RuntimeError("Generate succeeded but no job token returned")
        return token

    async def _poll_job(
        self,
        job_token: str,
        progress_callback: Optional[Callable[[ExecutionProgress], None]],
    ) -> Dict[str, Any]:
        assert self._http is not None
        headers = await self._auth_headers_async()
        elapsed = 0.0
        while elapsed < JOB_POLL_TIMEOUT_SECONDS:
            resp = await self._http.get(
                f"{self._base_url}/v1/omni_api/job_status/{job_token}",
                headers=headers,
            )
            resp.raise_for_status()
            status_payload = resp.json()
            state = status_payload.get("status", "").lower()
            if state in ("completed", "succeeded", "success"):
                if progress_callback:
                    progress_callback(ExecutionProgress(progress=1.0, stage="completed"))
                return status_payload
            if state in ("failed", "error", "cancelled"):
                raise RuntimeError(f"Job {job_token} {state}: {status_payload.get('error', '')}")
            if progress_callback:
                progress_callback(ExecutionProgress(
                    progress=min(0.9, elapsed / JOB_POLL_TIMEOUT_SECONDS),
                    stage=state or "processing",
                ))
            await asyncio.sleep(JOB_POLL_INTERVAL_SECONDS)
            elapsed += JOB_POLL_INTERVAL_SECONDS
        raise TimeoutError(f"Job {job_token} timed out after {JOB_POLL_TIMEOUT_SECONDS}s")

    async def _download_result(self, job_result: Dict[str, Any]) -> Optional[bytes]:
        download_url = (
            job_result.get("result_url")
            or job_result.get("download_url")
            or job_result.get("output_url")
        )
        if not download_url or self._http is None:
            return None
        resp = await self._http.get(download_url, headers=await self._auth_headers_async())
        resp.raise_for_status()
        return resp.content


# --- Module-level helpers ---

def _category_to_task_type(category: str) -> str:
    return {
        "image": "text-to-image",
        "video": "text-to-video",
        "audio": "text-to-audio",
        "mesh": "text-to-mesh",
        "splat": "text-to-splat",
    }.get(category, category)


def _mime_to_category(mime_type: str) -> str:
    if mime_type.startswith("image/"):
        return "image"
    if mime_type.startswith("video/"):
        return "video"
    if mime_type.startswith("audio/"):
        return "audio"
    return "image"


def _mime_to_ext(mime_type: str) -> str:
    return mime_type.split("/")[-1].split(";")[0] or "bin"


def _build_descriptor(
    category: str,
    model_name: str,
    model_entry: Dict[str, Any],
    task_type: str,
) -> ToolDescriptor:
    full_name = model_entry.get("full_name") or model_name
    tool_id = f"{category}:{model_name}"
    aspect_options = model_entry.get("aspect_ratio_options") or []
    resolution_options = model_entry.get("resolution_options") or []
    quality_options = model_entry.get("quality_options") or []
    batch_min = model_entry.get("batch_size_min", 1)
    batch_max = model_entry.get("batch_size_max", 1)
    batch_default = model_entry.get("batch_size_default", 1)

    properties: Dict[str, Any] = {
        "prompt": {
            "type": "string",
            "description": "Text prompt for the generation.",
        },
        "idempotency_token": {
            "type": "string",
            "description": "UUID for deduplication. Auto-generated if omitted.",
        },
    }
    required: List[str] = ["prompt"]

    if aspect_options:
        properties["aspect_ratio"] = {
            "type": "string",
            "enum": aspect_options,
            "default": model_entry.get("aspect_ratio_default"),
            "description": "Output aspect ratio.",
        }
    if resolution_options:
        properties["resolution"] = {
            "type": "string",
            "enum": resolution_options,
            "default": model_entry.get("resolution_default"),
            "description": "Output resolution.",
        }
    if quality_options:
        properties["quality"] = {
            "type": "string",
            "enum": quality_options,
            "default": model_entry.get("default_quality"),
            "description": "Output quality tier.",
        }
    if batch_max and batch_max > 1:
        properties["batch_size"] = {
            "type": "integer",
            "minimum": batch_min,
            "maximum": batch_max,
            "default": batch_default,
            "description": "Number of outputs to generate.",
        }
    if model_entry.get("image_refs_supported"):
        properties["image_refs"] = {
            "type": "array",
            "items": {"type": "string"},
            "maxItems": model_entry.get("image_refs_max", 1),
            "description": "Reference image asset IDs.",
        }
    if category == "video":
        properties["duration_seconds"] = {
            "type": "number",
            "minimum": 1,
            "maximum": 30,
            "default": 5,
            "description": "Video duration in seconds.",
        }
    properties["seed"] = {
        "type": "integer",
        "description": "Random seed. Omit for random.",
    }
    properties["negative_prompt"] = {
        "type": "string",
        "description": "Negative prompt (if supported).",
    }

    return ToolDescriptor(
        id=tool_id,
        name=full_name,
        parameter_schema={
            "type": "object",
            "properties": properties,
            "required": required,
        },
        output_schema={
            "type": "object",
            "properties": {
                "output_data": {"type": "string", "format": "binary"},
                "job_token": {"type": "string"},
            },
        },
        task_type=task_type,
        task_types=[task_type],
        model_vendor=model_entry.get("model_creator"),
        model=model_name,
        subtitle=f"Omni API · {category}",
        description=f"Generate {category} via the ArtCraft Omni API ({full_name}).",
        metadata={
            "omni_category": category,
            "omni_model": model_name,
            "provider_type": PROVIDER_TYPE,
        },
    )
