-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

ALTER TABLE users
  DROP INDEX index_age_verification_status,
  DROP COLUMN age_verified_at,
  DROP COLUMN age_verification_status,
  DROP COLUMN date_of_birth;
