use std::marker::PhantomData;
use std::sync::Arc;

use actix_web::web::{Json, Path, Query};
use actix_web::{web, HttpRequest};
use log::warn;
use utoipa::ToSchema;

use mysql_queries::queries::content_policy_violations::list_content_policy_violations_for_user::{list_content_policy_violations_for_user, ListContentPolicyViolationsForUserArgs};
use mysql_queries::queries::users::user::get::get_user_token_by_username::get_user_token_by_username;
use tokens::tokens::users::UserToken;

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::user_lookup::user_session::require_moderator::require_moderator;
use crate::state::server_state::ServerState;

const DEFAULT_PAGE_SIZE: u32 = 50;
const MAX_PAGE_SIZE: u32 = 200;

// ── Request ──

#[derive(Deserialize, ToSchema)]
pub struct ListContentPolicyViolationsPathInfo {
  username_or_token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ListContentPolicyViolationsQuery {
  maybe_cursor_id: Option<u64>,
  maybe_page_size: Option<u32>,
}

// ── Response ──

#[derive(Serialize, ToSchema)]
pub struct ContentPolicyViolationSummary {
  pub token: String,
  pub maybe_media_file_token: Option<String>,
  pub matched_category: String,
  pub maybe_matched_pattern_label: Option<String>,
  pub source: String,
  pub action_taken: String,
  pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ModeratorListContentPolicyViolationsResponse {
  pub success: bool,
  pub user_token: String,
  pub violations: Vec<ContentPolicyViolationSummary>,
  pub maybe_next_cursor_id: Option<u64>,
}

// ── Handler ──

/// Admin visibility into a user's content-policy-violation audit trail
/// (PRD.md F25). Never surfaces raw flagged text - only the category/pattern
/// labels the check itself recorded. See artcraft-studio-plan/SYSTEM_DESIGN.md §5.
#[utoipa::path(
  get,
  tag = "Moderation",
  path = "/v1/moderation/content_policy_violations/user/{username_or_token}",
  responses(
    (status = 200, description = "Success", body = ModeratorListContentPolicyViolationsResponse),
    (status = 401, description = "Unauthorized"),
    (status = 400, description = "Bad input"),
  ),
  params(
    ("username_or_token" = String, Path, description = "Username or user token"),
  )
)]
pub async fn moderator_list_content_policy_violations_handler(
  http_request: HttpRequest,
  path: Path<ListContentPolicyViolationsPathInfo>,
  query: Query<ListContentPolicyViolationsQuery>,
  server_state: web::Data<Arc<ServerState>>,
) -> Result<Json<ModeratorListContentPolicyViolationsResponse>, CommonWebError> {
  let _user_session = require_moderator(&http_request, &server_state.session_checker, &server_state.mysql_pool).await?;

  let username_or_token = path.username_or_token.trim();

  let user_token = if username_or_token.starts_with(UserToken::token_prefix()) || username_or_token.starts_with("U:") {
    UserToken::new_from_str(username_or_token)
  } else {
    get_user_token_by_username(username_or_token, &server_state.mysql_pool)
      .await
      .map_err(|err| {
        warn!("Could not get user token by username: {:?}", err);
        CommonWebError::from_anyhow_error(err)
      })?
      .ok_or_else(|| CommonWebError::NotFound)?
  };

  let page_size = query.maybe_page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE);

  let rows = list_content_policy_violations_for_user(ListContentPolicyViolationsForUserArgs {
    user_token: &user_token,
    maybe_cursor_id: query.maybe_cursor_id,
    limit: page_size,
    mysql_executor: &server_state.mysql_pool,
    phantom: PhantomData,
  }).await.map_err(|err| {
    warn!("list_content_policy_violations_for_user failed: {:?}", err);
    CommonWebError::from_error(err)
  })?;

  let maybe_next_cursor_id = if rows.len() as u32 == page_size {
    rows.last().map(|r| r.id)
  } else {
    None
  };

  let violations = rows.into_iter()
    .map(|r| ContentPolicyViolationSummary {
      token: r.token.as_str().to_string(),
      maybe_media_file_token: r.maybe_media_file_token,
      matched_category: r.matched_category,
      maybe_matched_pattern_label: r.maybe_matched_pattern_label,
      source: r.source.to_str().to_string(),
      action_taken: r.action_taken.to_str().to_string(),
      created_at: r.created_at.to_rfc3339(),
    })
    .collect();

  Ok(Json(ModeratorListContentPolicyViolationsResponse {
    success: true,
    user_token: user_token.as_str().to_string(),
    violations,
    maybe_next_cursor_id,
  }))
}
