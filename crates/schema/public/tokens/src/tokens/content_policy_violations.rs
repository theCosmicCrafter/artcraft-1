use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// The primary key for `content_policy_violations`.
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §2/§5.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct ContentPolicyViolationToken(pub String);

impl_crockford_generator!(ContentPolicyViolationToken, 32usize, TokenPrefix::ContentPolicyViolation, CrockfordLower);
impl_mysql_token_from_row!(ContentPolicyViolationToken);
impl_string_token!(ContentPolicyViolationToken);
