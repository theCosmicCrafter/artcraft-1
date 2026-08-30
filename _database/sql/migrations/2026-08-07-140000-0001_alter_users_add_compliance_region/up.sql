-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- Server-authoritative compliance region/consent regime, replacing the per-install
-- client-side cache pattern the Companion inherited from Stimma's compliance_region.py
-- (SYSTEM_DESIGN.md §5) - fixes a real gap where regime detection was inconsistent
-- across reinstalls/multiple devices by tying it to the account instead.
ALTER TABLE users
  -- ISO 3166-1 alpha-2 country code, e.g. "US". NULL until first resolved.
  ADD COLUMN maybe_compliance_country VARCHAR(2) DEFAULT NULL AFTER age_verified_at,

  -- Rust enum: ComplianceRegime (crates/schema/public/enums/src/by_table/users/compliance_regime.rs)
  -- 'optin' | 'optout' - mirrors compliance_region.py's shape, now server-authoritative.
  ADD COLUMN maybe_compliance_regime VARCHAR(16) DEFAULT NULL AFTER maybe_compliance_country;
