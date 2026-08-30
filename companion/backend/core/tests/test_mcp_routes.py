"""Tests for companion MCP server lifecycle routes."""

import pytest
from fastapi import HTTPException

from routes.mcp import mcp_health, mcp_start


@pytest.mark.asyncio
async def test_mcp_health_not_started():
    """Health endpoint reports not running before the server is started."""
    result = await mcp_health()
    assert result == {"running": False}


@pytest.mark.asyncio
async def test_mcp_start_stop_missing_binary():
    """Start fails gracefully when the binary cannot be found."""
    with pytest.raises(HTTPException) as exc_info:
        await mcp_start(override_path="/nonexistent/artcraft-mcp-server")
    assert exc_info.value.status_code == 503
