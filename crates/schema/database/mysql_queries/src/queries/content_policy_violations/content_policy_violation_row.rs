use chrono::{DateTime, Utc};
use sqlx::FromRow;

use enums::by_table::content_policy_violations::content_policy_violation_action::ContentPolicyViolationAction;
use enums::by_table::content_policy_violations::content_policy_violation_source::ContentPolicyViolationSource;
use tokens::tokens::content_policy_violations::ContentPolicyViolationToken;
use tokens::tokens::users::UserToken;

/// One `content_policy_violations` row. Uses runtime `sqlx::query_as` (not the
/// compile-time `query_as!` macro) since this table has no `.sqlx` offline-cache
/// entry yet — no live database was available to run `cargo sqlx prepare`
/// against in this environment. See mysql_queries/CLAUDE.md's documented
/// escape hatch for "SELECT queries for new tables where column type metadata
/// isn't cached yet."
#[derive(Debug, Clone, FromRow)]
pub struct ContentPolicyViolationRow {
  pub id: u64,
  pub token: ContentPolicyViolationToken,
  pub user_token: UserToken,
  pub maybe_media_file_token: Option<String>,
  pub matched_category: String,
  pub maybe_matched_pattern_label: Option<String>,
  pub source: ContentPolicyViolationSource,
  pub action_taken: ContentPolicyViolationAction,
  pub created_at: DateTime<Utc>,
}
