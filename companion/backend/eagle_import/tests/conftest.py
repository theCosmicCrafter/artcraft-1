"""Pytest fixtures for eagle_import tests - same shape as agent/tests/conftest.py
(a fresh in-memory-equivalent SQLite database per test), duplicated rather
than shared because pytest fixtures don't cross sibling test-package
boundaries.
"""
import pytest_asyncio
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from database import Base


@pytest_asyncio.fixture
async def async_engine(tmp_path):
    db_path = tmp_path / "test_eagle_import.db"
    engine = create_async_engine(f"sqlite+aiosqlite:///{db_path}", echo=False)
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    yield engine
    await engine.dispose()


@pytest_asyncio.fixture
async def session_factory(async_engine) -> async_sessionmaker:
    return async_sessionmaker(async_engine, class_=AsyncSession, expire_on_commit=False)


@pytest_asyncio.fixture
async def session(session_factory):
    async with session_factory() as sess:
        yield sess
        await sess.rollback()
