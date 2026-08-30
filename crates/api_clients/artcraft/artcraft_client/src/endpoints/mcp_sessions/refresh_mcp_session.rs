use crate::credentials::storyteller_credential_set::StorytellerCredentialSet;
use crate::error::storyteller_error::StorytellerError;
use crate::utils::api_host::ApiHost;
use crate::utils::basic_json_post_request::basic_json_post_request;
use artcraft_api_defs::mcp_sessions::refresh_mcp_session::{
  RefreshMcpSessionRequest, RefreshMcpSessionSuccessResponse,
};

const REFRESH_MCP_SESSION_PATH: &str = "/v1/mcp/session/refresh";

pub async fn refresh_mcp_session(
  api_host: &ApiHost,
  creds: &StorytellerCredentialSet,
  request: RefreshMcpSessionRequest,
) -> Result<RefreshMcpSessionSuccessResponse, StorytellerError> {
  basic_json_post_request(
    api_host,
    REFRESH_MCP_SESSION_PATH,
    Some(creds),
    request,
  ).await
}
