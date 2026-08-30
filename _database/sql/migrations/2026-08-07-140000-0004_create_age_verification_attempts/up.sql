-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- Audit trail of individual age-verification attempts, separate from the single
-- current-status column on `users` (age_verification_status) so history/appeals are
-- preserved across repeated attempts. See artcraft-studio-plan/SYSTEM_DESIGN.md §2/§5.
CREATE TABLE age_verification_attempts (
  -- Not used for anything except replication.
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,

  -- Effective "primary key".
  token VARCHAR(32) NOT NULL,

  -- The account attempting verification.
  user_token VARCHAR(32) NOT NULL,

  -- Rust enum: AgeVerificationMethod (crates/schema/public/enums/src/by_table/age_verification_attempts/age_verification_method.rs)
  method VARCHAR(32) NOT NULL,

  -- Rust enum: AgeVerificationResult (crates/schema/public/enums/src/by_table/age_verification_attempts/age_verification_result.rs)
  result VARCHAR(32) NOT NULL,

  -- OPTIONAL. Opaque reference into a third-party KYC/ID-verification provider's own
  -- system, for method = 'third_party_kyc'. Never store raw ID documents here.
  maybe_provider_reference VARCHAR(255) DEFAULT NULL,

  -- ========== TIMESTAMPS ==========

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  -- INDICES --
  PRIMARY KEY (id),
  UNIQUE KEY (token),

  KEY index_user_token (user_token),
  KEY index_created_at (created_at)

) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
