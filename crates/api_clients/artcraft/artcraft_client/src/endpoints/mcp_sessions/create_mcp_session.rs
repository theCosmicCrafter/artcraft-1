use crate::credentials::storyteller_credential_set::StorytellerCredentialSet;
use crate::error::storyteller_error::StorytellerError;
use crate::utils::api_host::ApiHost;
use crate::utils::basic_json_post_request::basic_json_post_request;
use artcraft_api_defs::mcp_sessions::create_mcp_session::{
  CreateMcpSessionRequest, CreateMcpSessionSuccessResponse,
};

const CREATE_MCP_SESSION_PATH: &str = "/v1/mcp/session/create";

pub async fn create_mcp_session(
  api_host: &ApiHost,
  maybe_creds: Option<&StorytellerCredentialSet>,
  request: CreateMcpSessionRequest,
) -> Result<CreateMcpSessionSuccessResponse, StorytellerError> {
  basic_json_post_request(
    api_host,
    CREATE_MCP_SESSION_PATH,
    maybe_creds,
    request,
  ).await
}
