from cloud_runtime import cloud_access_headers, env_cloud_base_url


def test_artcraft_studio_cloud_base_url_takes_precedence(monkeypatch):
    monkeypatch.setenv("ARTCRAFT_STUDIO_CLOUD_BASE_URL", "https://studio.example/ ")
    monkeypatch.setenv("STIMMA_CLOUD_BASE_URL", "https://legacy.example")

    assert env_cloud_base_url() == "https://studio.example"


def test_legacy_cloud_base_url_remains_compatible(monkeypatch):
    monkeypatch.delenv("ARTCRAFT_STUDIO_CLOUD_BASE_URL", raising=False)
    monkeypatch.delenv("ARTCRAFT_STUDIO_CLOUD_URL", raising=False)
    monkeypatch.setenv("STIMMA_CLOUD_URL", "https://legacy.example/")

    assert env_cloud_base_url() == "https://legacy.example"


def test_artcraft_studio_cloudflare_credentials_are_supported(monkeypatch):
    monkeypatch.setenv("ARTCRAFT_STUDIO_CLOUD_ACCESS_CLIENT_ID", "studio-client")
    monkeypatch.setenv("ARTCRAFT_STUDIO_CLOUD_ACCESS_CLIENT_SECRET", "studio-secret")

    assert cloud_access_headers() == {
        "CF-Access-Client-Id": "studio-client",
        "CF-Access-Client-Secret": "studio-secret",
    }
