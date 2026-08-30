use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;
#[cfg(test)]
use strum::EnumCount;
#[cfg(test)]
use strum::EnumIter;
use utoipa::ToSchema;

/// Used in the `content_policy_violations` table in a `VARCHAR(32)` field named `action_taken`.
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §5.
#[cfg_attr(test, derive(EnumIter, EnumCount))]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Deserialize, Serialize, Ord, PartialOrd, ToSchema)]
pub enum ContentPolicyViolationAction {
  /// The action was fully prevented (e.g. generation request rejected).
  #[serde(rename = "blocked")]
  Blocked,

  /// The action was allowed to proceed but recorded for review.
  #[serde(rename = "flagged")]
  Flagged,

  /// The action was allowed to proceed, with a warning surfaced to the user.
  #[serde(rename = "allowed_with_warning")]
  AllowedWithWarning,
}

impl_enum_display_and_debug_using_to_str!(ContentPolicyViolationAction);
impl_mysql_enum_coders!(ContentPolicyViolationAction);
impl_mysql_from_row!(ContentPolicyViolationAction);

impl ContentPolicyViolationAction {
  pub fn to_str(&self) -> &'static str {
    match self {
      Self::Blocked => "blocked",
      Self::Flagged => "flagged",
      Self::AllowedWithWarning => "allowed_with_warning",
    }
  }

  pub fn from_str(value: &str) -> Result<Self, String> {
    match value {
      "blocked" => Ok(Self::Blocked),
      "flagged" => Ok(Self::Flagged),
      "allowed_with_warning" => Ok(Self::AllowedWithWarning),
      _ => Err(format!("invalid value: {:?}", value)),
    }
  }

  pub fn all_variants() -> BTreeSet<Self> {
    BTreeSet::from([
      Self::Blocked,
      Self::Flagged,
      Self::AllowedWithWarning,
    ])
  }
}

#[cfg(test)]
mod tests {
  use crate::by_table::content_policy_violations::content_policy_violation_action::ContentPolicyViolationAction;
  use crate::test_helpers::assert_serialization;

  mod serde {
    use super::*;

    #[test]
    fn test_serialization() {
      assert_serialization(ContentPolicyViolationAction::Blocked, "blocked");
      assert_serialization(ContentPolicyViolationAction::Flagged, "flagged");
      assert_serialization(ContentPolicyViolationAction::AllowedWithWarning, "allowed_with_warning");
    }
  }

  mod impl_methods {
    use super::*;

    #[test]
    fn to_str() {
      assert_eq!(ContentPolicyViolationAction::Blocked.to_str(), "blocked");
      assert_eq!(ContentPolicyViolationAction::Flagged.to_str(), "flagged");
      assert_eq!(ContentPolicyViolationAction::AllowedWithWarning.to_str(), "allowed_with_warning");
    }

    #[test]
    fn from_str() {
      assert_eq!(ContentPolicyViolationAction::from_str("blocked").unwrap(), ContentPolicyViolationAction::Blocked);
      assert_eq!(ContentPolicyViolationAction::from_str("flagged").unwrap(), ContentPolicyViolationAction::Flagged);
      assert_eq!(ContentPolicyViolationAction::from_str("allowed_with_warning").unwrap(), ContentPolicyViolationAction::AllowedWithWarning);
      assert!(ContentPolicyViolationAction::from_str("foo").is_err());
    }
  }

  mod manual_variant_checks {
    use super::*;

    #[test]
    fn all_variants() {
      let mut variants = ContentPolicyViolationAction::all_variants();
      assert_eq!(variants.len(), 3);
      assert_eq!(variants.pop_first(), Some(ContentPolicyViolationAction::Blocked));
      assert_eq!(variants.pop_first(), Some(ContentPolicyViolationAction::Flagged));
      assert_eq!(variants.pop_first(), Some(ContentPolicyViolationAction::AllowedWithWarning));
      assert_eq!(variants.pop_first(), None);
    }
  }

  mod mechanical_checks {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn variant_length() {
      assert_eq!(ContentPolicyViolationAction::all_variants().len(), ContentPolicyViolationAction::iter().len());
    }

    #[test]
    fn round_trip() {
      for variant in ContentPolicyViolationAction::all_variants() {
        assert_eq!(variant, ContentPolicyViolationAction::from_str(variant.to_str()).unwrap());
        assert_eq!(variant, ContentPolicyViolationAction::from_str(&format!("{}", variant)).unwrap());
        assert_eq!(variant, ContentPolicyViolationAction::from_str(&format!("{:?}", variant)).unwrap());
      }
    }
  }
}
