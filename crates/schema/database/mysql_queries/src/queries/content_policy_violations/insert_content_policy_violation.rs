use std::marker::PhantomData;

use sqlx::{Executor, MySql};

use enums::by_table::content_policy_violations::content_policy_violation_action::ContentPolicyViolationAction;
use enums::by_table::content_policy_violations::content_policy_violation_source::ContentPolicyViolationSource;
use tokens::tokens::content_policy_violations::ContentPolicyViolationToken;
use tokens::tokens::users::UserToken;

pub struct InsertContentPolicyViolationArgs<'e, 'c, E>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  pub user_token: &'e UserToken,
  pub maybe_media_file_token: Option<&'e str>,
  pub matched_category: &'e str,
  pub maybe_matched_pattern_label: Option<&'e str>,
  pub source: ContentPolicyViolationSource,
  pub action_taken: ContentPolicyViolationAction,
  pub mysql_executor: E,
  pub phantom: PhantomData<&'c E>,
}

/// Durable audit-trail write for a content-policy check that already happened
/// client-side (Companion pre-check) or server-side (the check endpoint this
/// pairs with). Never pass raw flagged text as `maybe_matched_pattern_label` -
/// category/pattern LABELS only, matching the local blocklist's own
/// "never leaves the machine" discipline for the actual matched content. See
/// artcraft-studio-plan/SYSTEM_DESIGN.md §5.
pub async fn insert_content_policy_violation<'e, 'c: 'e, E>(
  args: InsertContentPolicyViolationArgs<'e, 'c, E>,
) -> Result<ContentPolicyViolationToken, sqlx::Error>
where
  E: 'e + Executor<'c, Database = MySql>,
{
  let token = ContentPolicyViolationToken::generate();

  sqlx::query(
    r#"
INSERT INTO content_policy_violations
  (token, user_token, maybe_media_file_token, matched_category, maybe_matched_pattern_label, source, action_taken)
VALUES
  (?, ?, ?, ?, ?, ?, ?)
    "#,
  )
    .bind(token.as_str())
    .bind(args.user_token.as_str())
    .bind(args.maybe_media_file_token)
    .bind(args.matched_category)
    .bind(args.maybe_matched_pattern_label)
    .bind(args.source.to_str())
    .bind(args.action_taken.to_str())
    .execute(args.mysql_executor)
    .await?;

  Ok(token)
}
