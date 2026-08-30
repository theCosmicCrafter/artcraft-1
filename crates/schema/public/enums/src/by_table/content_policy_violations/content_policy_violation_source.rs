use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;
#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// Used in the `content_policy_violations` table in a `VARCHAR(32)` field named `source`.
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §5.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Deserialize, Serialize, Ord, PartialOrd, ToSchema)]
pub enum ContentPolicyViolationSource {
  /// The Companion's local, fast, non-authoritative keyword-blocklist pre-check.
  #[serde(rename = "client_precheck")]
  ClientPrecheck,

  /// The server-side authority check (storyteller-web's content-policy endpoint).
  #[serde(rename = "server_scan")]
  ServerScan,
}

impl_enum_display_and_debug_using_to_str!(ContentPolicyViolationSource);
impl_mysql_enum_coders!(ContentPolicyViolationSource);
impl_mysql_from_row!(ContentPolicyViolationSource);

impl ContentPolicyViolationSource {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::ClientPrecheck => "client_precheck",
      Self::ServerScan => "server_scan",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "client_precheck" => Ok(Self::ClientPrecheck),
      "server_scan" => Ok(Self::ServerScan),
      _ => Err(format!("invalid value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([
      Self::ClientPrecheck,
      Self::ServerScan,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::by_table::content_policy_violations::content_policy_violation_source::ContentPolicyViolationSource;
  use crate::test_helpers::assert_serialization;

  mod serde {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(ContentPolicyViolationSource::ClientPrecheck, "client_precheck");
      assert_serialization(ContentPolicyViolationSource::ServerScan, "server_scan");
    }
  }

  mod impl_methods {
    use super::*;

    #[test]
    fn to_str() {
      assert_eq!(ContentPolicyViolationSource::ClientPrecheck.to_str(), "client_precheck");
      assert_eq!(ContentPolicyViolationSource::ServerScan.to_str(), "server_scan");
    }

    #[test]
    fn from_str() {
      assert_eq!(ContentPolicyViolationSource::from_str("client_precheck").unwrap(), ContentPolicyViolationSource::ClientPrecheck);
      assert_eq!(ContentPolicyViolationSource::from_str("server_scan").unwrap(), ContentPolicyViolationSource::ServerScan);
      assert!(ContentPolicyViolationSource::from_str("foo").is_err());
    }
  }

  mod manual_variant_checks {
    use super::*;

    #[test]
    fn all_variants() {
      let mut variants = ContentPolicyViolationSource::all_variants();
      assert_eq!(variants.len(), 2);
      assert_eq!(variants.pop_first(), Some(ContentPolicyViolationSource::ClientPrecheck));
      assert_eq!(variants.pop_first(), Some(ContentPolicyViolationSource::ServerScan));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn variant_length() {
      assert_eq!(ContentPolicyViolationSource::all_variants().len(), ContentPolicyViolationSource::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in ContentPolicyViolationSource::all_variants() {
        assert_eq!(variant, ContentPolicyViolationSource::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, ContentPolicyViolationSource::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, ContentPolicyViolationSource::from_str(&format!("{:?}", variant)).unwrap());
      }
    }
  }
}
