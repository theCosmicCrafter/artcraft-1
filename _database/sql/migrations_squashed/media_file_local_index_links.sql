-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- Thin reconciliation join between a cloud `media_files` row and the local Companion
-- library entry (SQLite, on-device) it was promoted from - e.g. when a user uploads a
-- locally-ingested file. Deliberately thin (a link, not a copy): the local SQLite row
-- stays authoritative for anything not yet uploaded. See
-- artcraft-studio-plan/SYSTEM_DESIGN.md §2 for the full rationale, including why local
-- CLIP embeddings/face-cluster data never appear in this or any MySQL table.
CREATE TABLE media_file_local_index_links (
  -- Not used for anything except replication.
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,

  -- Effective "primary key".
  token VARCHAR(32) NOT NULL,

  -- The cloud media_files row this local file was promoted to/linked with.
  media_file_token VARCHAR(32) NOT NULL,

  -- The account that owns both the cloud media_files row and the local library entry.
  owner_user_token VARCHAR(32) NOT NULL,

  -- Companion profile UUID (per-device/per-install local library), matching Stimma's
  -- existing profile_id concept (database_registry.py) - not a token from this table's
  -- own family, since it identifies a local install, not a cloud entity.
  local_profile_id VARCHAR(64) NOT NULL,

  -- Content hash of the local file, used by the Companion to answer "does this local
  -- file already have a cloud token?" without re-uploading.
  local_file_hash VARCHAR(64) NOT NULL,

  -- ========== TIMESTAMPS ==========

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  -- INDICES --
  PRIMARY KEY (id),
  UNIQUE KEY (token),
  UNIQUE KEY index_media_file_token (media_file_token),

  KEY index_owner_user_token (owner_user_token),
  KEY index_local_profile_and_hash (local_profile_id, local_file_hash)

) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
