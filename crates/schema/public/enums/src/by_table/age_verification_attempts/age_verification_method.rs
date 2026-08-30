use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;
#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// Used in the `age_verification_attempts` table in a `VARCHAR(32)` field named `method`.
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §5.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Deserialize, Serialize, Ord, PartialOrd, ToSchema)]
pub enum AgeVerificationMethod {
  /// The user asserted their own age/DOB without third-party proof.
  #[serde(rename = "self_attestation")]
  SelfAttestation,

  /// The user uploaded a government-issued ID for manual/automated review.
  #[serde(rename = "id_upload")]
  IdUpload,

  /// Verification was performed by a third-party KYC/ID-verification provider.
  #[serde(rename = "third_party_kyc")]
  ThirdPartyKyc,
}

impl_enum_display_and_debug_using_to_str!(AgeVerificationMethod);
impl_mysql_enum_coders!(AgeVerificationMethod);
impl_mysql_from_row!(AgeVerificationMethod);

impl AgeVerificationMethod {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::SelfAttestation => "self_attestation",
      Self::IdUpload => "id_upload",
      Self::ThirdPartyKyc => "third_party_kyc",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "self_attestation" => Ok(Self::SelfAttestation),
      "id_upload" => Ok(Self::IdUpload),
      "third_party_kyc" => Ok(Self::ThirdPartyKyc),
      _ => Err(format!("invalid value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([
      Self::SelfAttestation,
      Self::IdUpload,
      Self::ThirdPartyKyc,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::by_table::age_verification_attempts::age_verification_method::AgeVerificationMethod;
  use crate::test_helpers::assert_serialization;

  mod serde {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(AgeVerificationMethod::SelfAttestation, "self_attestation");
      assert_serialization(AgeVerificationMethod::IdUpload, "id_upload");
      assert_serialization(AgeVerificationMethod::ThirdPartyKyc, "third_party_kyc");
    }
  }

  mod impl_methods {
    use super::*;

    #[test]
    fn to_str() {
      assert_eq!(AgeVerificationMethod::SelfAttestation.to_str(), "self_attestation");
      assert_eq!(AgeVerificationMethod::IdUpload.to_str(), "id_upload");
      assert_eq!(AgeVerificationMethod::ThirdPartyKyc.to_str(), "third_party_kyc");
    }

    #[test]
    fn from_str() {
      assert_eq!(AgeVerificationMethod::from_str("self_attestation").unwrap(), AgeVerificationMethod::SelfAttestation);
      assert_eq!(AgeVerificationMethod::from_str("id_upload").unwrap(), AgeVerificationMethod::IdUpload);
      assert_eq!(AgeVerificationMethod::from_str("third_party_kyc").unwrap(), AgeVerificationMethod::ThirdPartyKyc);
      assert!(AgeVerificationMethod::from_str("foo").is_err());
    }
  }

  mod manual_variant_checks {
    use super::*;

    #[test]
    fn all_variants() {
      let mut variants = AgeVerificationMethod::all_variants();
      assert_eq!(variants.len(), 3);
      assert_eq!(variants.pop_first(), Some(AgeVerificationMethod::SelfAttestation));
      assert_eq!(variants.pop_first(), Some(AgeVerificationMethod::IdUpload));
      assert_eq!(variants.pop_first(), Some(AgeVerificationMethod::ThirdPartyKyc));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn variant_length() {
      assert_eq!(AgeVerificationMethod::all_variants().len(), AgeVerificationMethod::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in AgeVerificationMethod::all_variants() {
        assert_eq!(variant, AgeVerificationMethod::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, AgeVerificationMethod::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, AgeVerificationMethod::from_str(&format!("{:?}", variant)).unwrap());
      }
    }
  }
}
