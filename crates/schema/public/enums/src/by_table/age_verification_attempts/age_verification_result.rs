use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;
#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// Used in the `age_verification_attempts` table in a `VARCHAR(32)` field named `result`.
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §5.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Deserialize, Serialize, Ord, PartialOrd, ToSchema)]
pub enum AgeVerificationResult {
  #[serde(rename = "passed")]
  Passed,

  #[serde(rename = "failed")]
  Failed,

  /// Submitted but not yet resolved (e.g. pending third-party KYC review).
  #[serde(rename = "pending")]
  Pending,
}

impl_enum_display_and_debug_using_to_str!(AgeVerificationResult);
impl_mysql_enum_coders!(AgeVerificationResult);
impl_mysql_from_row!(AgeVerificationResult);

impl AgeVerificationResult {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::Passed => "passed",
      Self::Failed => "failed",
      Self::Pending => "pending",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "passed" => Ok(Self::Passed),
      "failed" => Ok(Self::Failed),
      "pending" => Ok(Self::Pending),
      _ => Err(format!("invalid value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([
      Self::Passed,
      Self::Failed,
      Self::Pending,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::by_table::age_verification_attempts::age_verification_result::AgeVerificationResult;
  use crate::test_helpers::assert_serialization;

  mod serde {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(AgeVerificationResult::Passed, "passed");
      assert_serialization(AgeVerificationResult::Failed, "failed");
      assert_serialization(AgeVerificationResult::Pending, "pending");
    }
  }

  mod impl_methods {
    use super::*;

    #[test]
    fn to_str() {
      assert_eq!(AgeVerificationResult::Passed.to_str(), "passed");
      assert_eq!(AgeVerificationResult::Failed.to_str(), "failed");
      assert_eq!(AgeVerificationResult::Pending.to_str(), "pending");
    }

    #[test]
    fn from_str() {
      assert_eq!(AgeVerificationResult::from_str("passed").unwrap(), AgeVerificationResult::Passed);
      assert_eq!(AgeVerificationResult::from_str("failed").unwrap(), AgeVerificationResult::Failed);
      assert_eq!(AgeVerificationResult::from_str("pending").unwrap(), AgeVerificationResult::Pending);
      assert!(AgeVerificationResult::from_str("foo").is_err());
    }
  }

  mod manual_variant_checks {
    use super::*;

    #[test]
    fn all_variants() {
      let mut variants = AgeVerificationResult::all_variants();
      assert_eq!(variants.len(), 3);
      assert_eq!(variants.pop_first(), Some(AgeVerificationResult::Passed));
      assert_eq!(variants.pop_first(), Some(AgeVerificationResult::Failed));
      assert_eq!(variants.pop_first(), Some(AgeVerificationResult::Pending));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn variant_length() {
      assert_eq!(AgeVerificationResult::all_variants().len(), AgeVerificationResult::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in AgeVerificationResult::all_variants() {
        assert_eq!(variant, AgeVerificationResult::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, AgeVerificationResult::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, AgeVerificationResult::from_str(&format!("{:?}", variant)).unwrap());
      }
    }
  }
}
