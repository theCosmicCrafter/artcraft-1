import pytest

from distribution import DISTRIBUTION_DEV, DISTRIBUTION_OFFICIAL, get_distribution


@pytest.mark.parametrize("value, expected", [("official", DISTRIBUTION_OFFICIAL), ("dev", DISTRIBUTION_DEV)])
def test_artcraft_studio_distribution_env_controls_build_mode(monkeypatch, value, expected):
    monkeypatch.setenv("ARTCRAFT_STUDIO_DISTRIBUTION", value)
    monkeypatch.delenv("STIMMA_DISTRIBUTION", raising=False)

    assert get_distribution() == expected


def test_artcraft_studio_distribution_env_takes_precedence_over_legacy_env(monkeypatch):
    monkeypatch.setenv("ARTCRAFT_STUDIO_DISTRIBUTION", "dev")
    monkeypatch.setenv("STIMMA_DISTRIBUTION", "official")

    assert get_distribution() == DISTRIBUTION_DEV


def test_legacy_distribution_env_remains_compatible(monkeypatch):
    monkeypatch.delenv("ARTCRAFT_STUDIO_DISTRIBUTION", raising=False)
    monkeypatch.setenv("STIMMA_DISTRIBUTION", "official")

    assert get_distribution() == DISTRIBUTION_OFFICIAL
