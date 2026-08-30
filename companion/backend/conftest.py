"""Top-level pytest config for the Companion test suite.

`pytest_plugins` registrations must live in the rootdir conftest as of
pytest 8+ (non-top-level declarations were deprecated and later removed -
see https://docs.pytest.org/en/stable/deprecations.html#pytest-plugins-in-non-top-level-conftest-files).
Moved here from agent/tests/conftest.py, which is not top-level.
"""

pytest_plugins = ('pytest_asyncio',)
