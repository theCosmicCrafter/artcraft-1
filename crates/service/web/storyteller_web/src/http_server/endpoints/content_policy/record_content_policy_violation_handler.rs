use std::marker::PhantomData;
use std::sync::Arc;

use actix_web::web::Json;
use actix_web::{web, HttpRequest};
use log::warn;
use utoipa::ToSchema;

use enums::by_table::content_policy_violations::content_policy_violation_action::ContentPolicyViolationAction;
use enums::by_table::content_policy_violations::content_policy_violation_source::ContentPolicyViolationSource;
use mysql_queries::queries::content_policy_violations::insert_content_policy_violation::{insert_content_policy_violation, InsertContentPolicyViolationArgs};

use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::user_lookup::user_session::require_user_session::require_user_session;
use crate::state::server_state::ServerState;

const MAX_CATEGORY_LEN: usize = 64;
const MAX_PATTERN_LABEL_LEN: usize = 128;

// ── Request ──

/// Durable audit-trail write for a content-policy check that already happened
/// client-side (Companion's local keyword-blocklist pre-check) or is happening
/// server-side right now, alongside this call (e.g. at the point of Omni API
/// spend). This endpoint does NOT itself decide policy - the caller has
/// already made that determination; this just records it. See
/// artcraft-studio-plan/SYSTEM_DESIGN.md §5.
///
/// `matched_category` and `maybe_matched_pattern_label` must be LABELS only,
/// never the raw flagged text - matching the local blocklist's own
/// "never leaves the machine" discipline for the actual matched content.
#[derive(Deserialize, ToSchema)]
pub struct RecordContentPolicyViolationRequest {
  pub maybe_media_file_token: Option<String>,
  pub matched_category: String,
  pub maybe_matched_pattern_label: Option<String>,
  pub source: ContentPolicyViolationSource,
  pub action_taken: ContentPolicyViolationAction,
}

// ── Response ──

#[derive(Serialize, ToSchema)]
pub struct RecordContentPolicyViolationResponse {
  pub success: bool,
  pub content_policy_violation_token: String,
}

// ── Handler ──

#[utoipa::path(
  post,
  tag = "Content Policy",
  path = "/v1/content_policy/record_violation",
  request_body = RecordContentPolicyViolationRequest,
  responses(
    (status = 200, description = "Success", body = RecordContentPolicyViolationResponse),
    (status = 400, description = "Bad input"),
    (status = 401, description = "Unauthorized"),
  ),
)]
pub async fn record_content_policy_violation_handler(
  http_request: HttpRequest,
  request: Json<RecordContentPolicyViolationRequest>,
  server_state: web::Data<Arc<ServerState>>,
) -> Result<Json<RecordContentPolicyViolationResponse>, CommonWebError> {
  let mut conn = server_state.mysql_pool.acquire().await.map_err(|err| {
    warn!("MySQL pool error: {:?}", err);
    CommonWebError::from_error(err)
  })?;

  let user_session = require_user_session(&http_request, &server_state.session_checker, &mut *conn).await?;

  let matched_category = request.matched_category.trim();
  if matched_category.is_empty() {
    return Err(CommonWebError::BadInputWithSimpleMessage("matched_category is empty".to_string()));
  }
  if matched_category.len() > MAX_CATEGORY_LEN {
    return Err(CommonWebError::BadInputWithSimpleMessage(
      format!("matched_category too long (max {} chars)", MAX_CATEGORY_LEN),
    ));
  }

  let maybe_matched_pattern_label = request.maybe_matched_pattern_label.as_deref().map(|s| s.trim());
  if let Some(label) = maybe_matched_pattern_label {
    if label.len() > MAX_PATTERN_LABEL_LEN {
      return Err(CommonWebError::BadInputWithSimpleMessage(
        format!("maybe_matched_pattern_label too long (max {} chars)", MAX_PATTERN_LABEL_LEN),
      ));
    }
  }

  let token = insert_content_policy_violation(InsertContentPolicyViolationArgs {
    user_token: &user_session.user_token,
    maybe_media_file_token: request.maybe_media_file_token.as_deref(),
    matched_category,
    maybe_matched_pattern_label,
    source: request.source,
    action_taken: request.action_taken,
    mysql_executor: &mut *conn,
    phantom: PhantomData,
  }).await.map_err(|err| {
    warn!("insert_content_policy_violation failed: {:?}", err);
    CommonWebError::from_error(err)
  })?;

  Ok(Json(RecordContentPolicyViolationResponse {
    success: true,
    content_policy_violation_token: token.as_str().to_string(),
  }))
}
