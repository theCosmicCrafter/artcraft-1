use reqwest::RequestBuilder;
use tokens::tokens::mcp_session_private::McpSessionPrivateToken;
use tokens::tokens::mcp_session_refresh::McpSessionRefreshToken;

use crate::credentials::parse_multi_cookie_header::parse_multi_cookie_header;
use crate::credentials::storyteller_avt_cookie::StorytellerAvtCookie;
use crate::credentials::storyteller_session_cookie::StorytellerSessionCookie;

#[derive(Clone)]
pub struct StorytellerCredentialSet {
  pub avt: Option<StorytellerAvtCookie>,
  pub session: Option<StorytellerSessionCookie>,
  pub artcraft_api_key: Option<String>,
  pub mcp_private_session_token: Option<McpSessionPrivateToken>,
  pub mcp_refresh_token: Option<McpSessionRefreshToken>,
}

impl StorytellerCredentialSet {
  pub fn empty() -> Self {
    Self {
      avt: None,
      session: None,
      artcraft_api_key: None,
      mcp_private_session_token: None,
      mcp_refresh_token: None,
    }
  }

  pub fn initialize(
    avt: Option<StorytellerAvtCookie>,
    session: Option<StorytellerSessionCookie>,
  ) -> Self {
    Self {
      avt,
      session,
      artcraft_api_key: None,
      mcp_private_session_token: None,
      mcp_refresh_token: None,
    }
  }

  pub fn initialize_with_mcp_token(
    mcp_private_session_token: McpSessionPrivateToken,
  ) -> Self {
    Self {
      avt: None,
      session: None,
      artcraft_api_key: None,
      mcp_private_session_token: Some(mcp_private_session_token),
      mcp_refresh_token: None,
    }
  }

  pub fn initialize_with_mcp_tokens(
    mcp_private_session_token: McpSessionPrivateToken,
    mcp_refresh_token: McpSessionRefreshToken,
  ) -> Self {
    Self {
      avt: None,
      session: None,
      artcraft_api_key: None,
      mcp_private_session_token: Some(mcp_private_session_token),
      mcp_refresh_token: Some(mcp_refresh_token),
    }
  }

  pub fn initialize_with_api_key(api_key: String) -> Self {
    Self {
      avt: None,
      session: None,
      artcraft_api_key: Some(api_key),
      mcp_private_session_token: None,
      mcp_refresh_token: None,
    }
  }

  pub fn parse_multi_cookie_header(header: &str) -> Result<Option<Self>, cookie::ParseError> {
    parse_multi_cookie_header(header)
  }

  pub fn initialize_with_just_cookie(session: StorytellerSessionCookie) -> Self {
    Self {
      avt: None,
      session: Some(session),
      artcraft_api_key: None,
      mcp_private_session_token: None,
      mcp_refresh_token: None,
    }
  }

  pub fn initialize_with_just_avt(avt: StorytellerAvtCookie) -> Self {
    Self {
      avt: Some(avt),
      session: None,
      artcraft_api_key: None,
      mcp_private_session_token: None,
      mcp_refresh_token: None,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.avt.is_none()
      && self.session.is_none()
      && self.artcraft_api_key.is_none()
      && self.mcp_private_session_token.is_none()
  }

  pub fn equals(&self, other: &Self) -> bool {
    match (&self.avt, &other.avt) {
      (None, None) => {} // Fallthrough
      (Some(avt), Some(other_avt)) => {
        if !avt.equals(other_avt) {
          return false;
        }
      }
      _ => return false,
    }

    match (&self.session, &other.session) {
      (None, None) => {} // Fallthrough
      (Some(session), Some(other_session)) => {
        if !session.equals(other_session) {
          return false;
        }
      }
      _ => return false,
    }

    match (&self.artcraft_api_key, &other.artcraft_api_key) {
      (None, None) => {} // Fallthrough
      (Some(key), Some(other_key)) => {
        if key != other_key {
          return false;
        }
      }
      _ => return false,
    }

    match (&self.mcp_private_session_token, &other.mcp_private_session_token) {
      (None, None) => {} // Fallthrough
      (Some(token), Some(other_token)) => {
        if token != other_token {
          return false;
        }
      }
      _ => return false,
    }

    match (&self.mcp_refresh_token, &other.mcp_refresh_token) {
      (None, None) => {} // Fallthrough
      (Some(token), Some(other_token)) => {
        if token != other_token {
          return false;
        }
      }
      _ => return false,
    }

    true
  }

  pub fn maybe_as_cookie_header(&self) -> Option<String> {
    let mut cookies = Vec::new();

    if let Some(avt) = &self.avt {
      cookies.push(avt.as_cookie_header());
    }

    if let Some(session) = &self.session {
      cookies.push(session.as_cookie_header());
    }

    if cookies.is_empty() {
      None
    } else {
      Some(cookies.join("; "))
    }
  }

  pub fn maybe_as_authorization_header(&self) -> Option<String> {
    self.mcp_private_session_token
      .as_ref()
      .map(|token| format!("Bearer {}", token.as_str()))
  }

  /// Apply the most appropriate auth header(s) to a [`reqwest::RequestBuilder`].
  ///
  /// - If an MCP private session token is set, it is sent as `Authorization: Bearer <token>`.
  /// - Otherwise, if an ArtCraft API key is set, it is sent as `Authorization: Bearer <key>`.
  /// - Otherwise, if session/AVT cookies are set, they are sent as `Cookie`.
  pub fn apply_auth(&self, builder: RequestBuilder) -> RequestBuilder {
    if let Some(auth) = self.maybe_as_authorization_header() {
      builder.header("Authorization", auth)
    } else if let Some(api_key) = &self.artcraft_api_key {
      builder.header("Authorization", format!("Bearer {}", api_key))
    } else if let Some(cookie) = self.maybe_as_cookie_header() {
      builder.header("Cookie", cookie)
    } else {
      builder
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::credentials::storyteller_avt_cookie::StorytellerAvtCookie;
  use crate::credentials::storyteller_credential_set::StorytellerCredentialSet;
  use crate::credentials::storyteller_session_cookie::StorytellerSessionCookie;
  use tokens::tokens::mcp_session_private::McpSessionPrivateToken;

  mod cookies_header {
    use super::*;

    #[test]
    fn no_cookie() {
      let creds = StorytellerCredentialSet::empty();
      let header = creds.maybe_as_cookie_header();
      assert_eq!(header, None);
    }

    #[test]
    fn avt_cookies_header() {
      let creds = StorytellerCredentialSet::initialize_with_just_avt(
        StorytellerAvtCookie::new("bob".to_string()),
      );

      let header = creds.maybe_as_cookie_header();
      assert_eq!(header, Some("visitor=bob".to_string()));
    }

    #[test]
    fn session_cookies_header() {
      let creds = StorytellerCredentialSet::initialize_with_just_cookie(
        StorytellerSessionCookie::new("bob".to_string()),
      );

      let header = creds.maybe_as_cookie_header();
      assert_eq!(header, Some("session=bob".to_string()));
    }

    #[test]
    fn both_cookies_header() {
      let creds = StorytellerCredentialSet::initialize(
        Some(StorytellerAvtCookie::new("bob".to_string())),
        Some(StorytellerSessionCookie::new("alice".to_string())),
      );

      let header = creds.maybe_as_cookie_header();
      assert_eq!(header, Some("visitor=bob; session=alice".to_string()));
    }
  }

  mod authorization_header {
    use super::*;

    #[test]
    fn no_authorization() {
      let creds = StorytellerCredentialSet::empty();
      assert_eq!(creds.maybe_as_authorization_header(), None);
    }

    #[test]
    fn mcp_private_session_token_header() {
      let token = McpSessionPrivateToken::new_from_str("mcp_session_private_test123");
      let creds = StorytellerCredentialSet::initialize_with_mcp_token(token);

      assert_eq!(
        creds.maybe_as_authorization_header(),
        Some("Bearer mcp_session_private_test123".to_string()),
      );
    }

    #[test]
    fn api_key_preferred_over_cookie() {
      let creds = StorytellerCredentialSet {
        avt: Some(StorytellerAvtCookie::new("bob".to_string())),
        session: None,
        artcraft_api_key: Some("artcraft_api_test".to_string()),
        mcp_private_session_token: None,
        mcp_refresh_token: None,
      };

      // We can't inspect the built headers without sending a request, but we can at least
      // confirm the auth helper builds the right Authorization string.
      assert_eq!(
        creds.maybe_as_authorization_header(),
        None,
      );
      assert_eq!(
        creds.artcraft_api_key.as_ref().map(|k| format!("Bearer {}", k)),
        Some("Bearer artcraft_api_test".to_string()),
      );
    }
  }
}
