use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;
#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// Used in the `users` table in a `VARCHAR(32)` field named `age_verification_status`.
///
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §2/§5 and PRD.md §8.
/// Client-side (Companion/frontend) age-gates library display of mature content based
/// on this status; server-side spend-adjacent enforcement (can this account request a
/// mature-tagged model) must independently check this on the server, since a modified
/// client could otherwise bypass a client-only gate.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Deserialize, Serialize, Ord, PartialOrd, ToSchema)]
pub enum AgeVerificationStatus {
  /// Default for every account. No attempt has been made.
  #[serde(rename = "unverified")]
  Unverified,

  /// The user asserted their own age/DOB without third-party proof.
  #[serde(rename = "self_attested")]
  SelfAttested,

  /// A third-party ID verification provider confirmed the account holder's age.
  #[serde(rename = "verified_id")]
  VerifiedId,

  /// A verification attempt was made and did not succeed.
  #[serde(rename = "failed")]
  Failed,
}

impl_enum_display_and_debug_using_to_str!(AgeVerificationStatus);
impl_mysql_enum_coders!(AgeVerificationStatus);
impl_mysql_from_row!(AgeVerificationStatus);

impl AgeVerificationStatus {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::Unverified => "unverified",
      Self::SelfAttested => "self_attested",
      Self::VerifiedId => "verified_id",
      Self::Failed => "failed",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "unverified" => Ok(Self::Unverified),
      "self_attested" => Ok(Self::SelfAttested),
      "verified_id" => Ok(Self::VerifiedId),
      "failed" => Ok(Self::Failed),
      _ => Err(format!("invalid value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([
      Self::Unverified,
      Self::SelfAttested,
      Self::VerifiedId,
      Self::Failed,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::by_table::users::age_verification_status::AgeVerificationStatus;
  use crate::test_helpers::assert_serialization;

  mod serde {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(AgeVerificationStatus::Unverified, "unverified");
      assert_serialization(AgeVerificationStatus::SelfAttested, "self_attested");
      assert_serialization(AgeVerificationStatus::VerifiedId, "verified_id");
      assert_serialization(AgeVerificationStatus::Failed, "failed");
    }
  }

  mod impl_methods {
    use super::*;

    #[test]
    fn to_str() {
      assert_eq!(AgeVerificationStatus::Unverified.to_str(), "unverified");
      assert_eq!(AgeVerificationStatus::SelfAttested.to_str(), "self_attested");
      assert_eq!(AgeVerificationStatus::VerifiedId.to_str(), "verified_id");
      assert_eq!(AgeVerificationStatus::Failed.to_str(), "failed");
    }

    #[test]
    fn from_str() {
      assert_eq!(AgeVerificationStatus::from_str("unverified").unwrap(), AgeVerificationStatus::Unverified);
      assert_eq!(AgeVerificationStatus::from_str("self_attested").unwrap(), AgeVerificationStatus::SelfAttested);
      assert_eq!(AgeVerificationStatus::from_str("verified_id").unwrap(), AgeVerificationStatus::VerifiedId);
      assert_eq!(AgeVerificationStatus::from_str("failed").unwrap(), AgeVerificationStatus::Failed);
      assert!(AgeVerificationStatus::from_str("foo").is_err());
    }
  }

  mod manual_variant_checks {
    use super::*;

    #[test]
    fn all_variants() {
      let mut variants = AgeVerificationStatus::all_variants();
      assert_eq!(variants.len(), 4);
      assert_eq!(variants.pop_first(), Some(AgeVerificationStatus::Unverified));
      assert_eq!(variants.pop_first(), Some(AgeVerificationStatus::SelfAttested));
      assert_eq!(variants.pop_first(), Some(AgeVerificationStatus::VerifiedId));
      assert_eq!(variants.pop_first(), Some(AgeVerificationStatus::Failed));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn variant_length() {
      assert_eq!(AgeVerificationStatus::all_variants().len(), AgeVerificationStatus::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in AgeVerificationStatus::all_variants() {
        assert_eq!(variant, AgeVerificationStatus::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, AgeVerificationStatus::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, AgeVerificationStatus::from_str(&format!("{:?}", variant)).unwrap());
      }
    }
  }
}
