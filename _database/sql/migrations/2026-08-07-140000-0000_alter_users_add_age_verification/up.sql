-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- Age verification, added for ArtCraft Studio's mature-content generation support.
-- See artcraft-studio-plan/SYSTEM_DESIGN.md §2/§5 and PRD.md §8 for the full design
-- and the resolved product decisions this schema serves.
--
-- NB: `users` previously had an unrelated `nsfw_status` column on media_files (not
-- users), dropped 2026-07-09 (migration alter_media_files_drop_nsfw_status). These
-- new columns are a distinct concept (account-level age verification, not a
-- per-media-item moderation status) and are not a resurrection of that column.
ALTER TABLE users
  ADD COLUMN date_of_birth DATE DEFAULT NULL AFTER maybe_mod_user_token,

  -- Rust enum: AgeVerificationStatus (crates/schema/public/enums/src/by_table/users/age_verification_status.rs)
  ADD COLUMN age_verification_status VARCHAR(32) NOT NULL DEFAULT 'unverified' AFTER date_of_birth,

  ADD COLUMN age_verified_at TIMESTAMP NULL DEFAULT NULL AFTER age_verification_status;

ALTER TABLE users
  ADD INDEX index_age_verification_status (age_verification_status);
