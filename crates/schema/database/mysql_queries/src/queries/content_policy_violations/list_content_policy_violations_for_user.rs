use std::marker::PhantomData;

use sqlx::{Executor, MySql};

use tokens::tokens::users::UserToken;

use crate::queries::content_policy_violations::content_policy_violation_row::ContentPolicyViolationRow;

pub struct ListContentPolicyViolationsForUserArgs<'e, 'c, E>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  pub user_token: &'e UserToken,
  pub maybe_cursor_id: Option<u64>,
  pub limit: u32,
  pub mysql_executor: E,
  pub phantom: PhantomData<&'c E>,
}

/// Paginated list of a user's content-policy-violation audit records, newest
/// first. Backs the moderation admin surface (PRD.md F25) - see
/// artcraft-studio-plan/SYSTEM_DESIGN.md §5.
pub async fn list_content_policy_violations_for_user<'e, 'c: 'e, E>(
  args: ListContentPolicyViolationsForUserArgs<'e, 'c, E>,
) -> Result<Vec<ContentPolicyViolationRow>, sqlx::Error>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  // No-cursor requests use a sentinel of u64::MAX - every id matches, which
  // keeps this to a single static query (mirrors list_tags_for_user).
  let cursor_id = args.maybe_cursor_id.unwrap_or(u64::MAX);
  let limit = args.limit as i64;

  let rows = sqlx::query_as::<_, ContentPolicyViolationRow>(
    r#"
SELECT
  id,
  token,
  user_token,
  maybe_media_file_token,
  matched_category,
  maybe_matched_pattern_label,
  source,
  action_taken,
  created_at
FROM content_policy_violations
WHERE user_token = ?
  AND id < ?
ORDER BY id DESC
LIMIT ?
    "#,
  )
    .bind(args.user_token.as_str())
    .bind(cursor_id)
    .bind(limit)
    .fetch_all(args.mysql_executor)
    .await?;

  Ok(rows)
}
