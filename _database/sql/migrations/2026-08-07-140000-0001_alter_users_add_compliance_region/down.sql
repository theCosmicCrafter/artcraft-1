-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

ALTER TABLE users
  DROP COLUMN maybe_compliance_regime,
  DROP COLUMN maybe_compliance_country;
