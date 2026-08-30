-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- Durable audit trail for content-policy checks, both the fast client-side pre-check
-- (Companion, ported from Stimma's keyword_blocklist.py) and the server-side authority
-- endpoint (new - see artcraft-studio-plan/SYSTEM_DESIGN.md §5). Never stores raw
-- matched text, only a category/pattern label - matching the local blocklist's own
-- "never leaves the machine" discipline for the actual flagged content.
CREATE TABLE content_policy_violations (
  -- Not used for anything except replication.
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,

  -- Effective "primary key".
  token VARCHAR(32) NOT NULL,

  -- The account the check ran against.
  user_token VARCHAR(32) NOT NULL,

  -- OPTIONAL. The media file this check concerned, if any (e.g. a generation request
  -- for a specific asset, as opposed to a prompt-only check).
  maybe_media_file_token VARCHAR(32) DEFAULT NULL,

  -- Which policy category matched (e.g. "illegal_content", "csam_adjacent") - a label,
  -- not the raw matched text.
  matched_category VARCHAR(64) NOT NULL,

  -- OPTIONAL. Which specific named pattern within the category matched, for triage -
  -- still a label, never the raw flagged text itself.
  maybe_matched_pattern_label VARCHAR(128) DEFAULT NULL,

  -- Rust enum: ContentPolicyViolationSource (crates/schema/public/enums/src/by_table/content_policy_violations/content_policy_violation_source.rs)
  source VARCHAR(32) NOT NULL,

  -- Rust enum: ContentPolicyViolationAction (crates/schema/public/enums/src/by_table/content_policy_violations/content_policy_violation_action.rs)
  action_taken VARCHAR(32) NOT NULL,

  -- ========== TIMESTAMPS ==========

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  -- INDICES --
  PRIMARY KEY (id),
  UNIQUE KEY (token),

  KEY index_user_token (user_token),
  KEY index_maybe_media_file_token (maybe_media_file_token),
  KEY index_matched_category (matched_category),
  KEY index_created_at (created_at)

) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
