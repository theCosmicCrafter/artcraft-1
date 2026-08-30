use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;
#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// Used in the `users` table in a `VARCHAR(16)` field named `maybe_compliance_regime`.
///
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §2/§5. Server-authoritative
/// replacement for the per-install client-side cache pattern the Companion inherited from
/// Stimma's `compliance_region.py`; mirrors that module's shape (`regime: optin | optout`).
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Deserialize, Serialize, Ord, PartialOrd, ToSchema)]
pub enum ComplianceRegime {
  /// Mature-content features are off by default; the user must explicitly opt in.
  #[serde(rename = "optin")]
  OptIn,

  /// Mature-content features are on by default; the user may explicitly opt out.
  #[serde(rename = "optout")]
  OptOut,
}

impl_enum_display_and_debug_using_to_str!(ComplianceRegime);
impl_mysql_enum_coders!(ComplianceRegime);
impl_mysql_from_row!(ComplianceRegime);

impl ComplianceRegime {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::OptIn => "optin",
      Self::OptOut => "optout",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "optin" => Ok(Self::OptIn),
      "optout" => Ok(Self::OptOut),
      _ => Err(format!("invalid value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([
      Self::OptIn,
      Self::OptOut,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::by_table::users::compliance_regime::ComplianceRegime;
  use crate::test_helpers::assert_serialization;

  mod serde {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(ComplianceRegime::OptIn, "optin");
      assert_serialization(ComplianceRegime::OptOut, "optout");
    }
  }

  mod impl_methods {
    use super::*;

    #[test]
    fn to_str() {
      assert_eq!(ComplianceRegime::OptIn.to_str(), "optin");
      assert_eq!(ComplianceRegime::OptOut.to_str(), "optout");
    }

    #[test]
    fn from_str() {
      assert_eq!(ComplianceRegime::from_str("optin").unwrap(), ComplianceRegime::OptIn);
      assert_eq!(ComplianceRegime::from_str("optout").unwrap(), ComplianceRegime::OptOut);
      assert!(ComplianceRegime::from_str("foo").is_err());
    }
  }

  mod manual_variant_checks {
    use super::*;

    #[test]
    fn all_variants() {
      let mut variants = ComplianceRegime::all_variants();
      assert_eq!(variants.len(), 2);
      assert_eq!(variants.pop_first(), Some(ComplianceRegime::OptIn));
      assert_eq!(variants.pop_first(), Some(ComplianceRegime::OptOut));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn variant_length() {
      assert_eq!(ComplianceRegime::all_variants().len(), ComplianceRegime::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in ComplianceRegime::all_variants() {
        assert_eq!(variant, ComplianceRegime::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, ComplianceRegime::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, ComplianceRegime::from_str(&format!("{:?}", variant)).unwrap());
      }
    }
  }
}
