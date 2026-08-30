use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// The primary key for `age_verification_attempts`.
/// ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §2.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct AgeVerificationAttemptToken(pub String);

impl_crockford_generator!(AgeVerificationAttemptToken, 32usize, TokenPrefix::AgeVerificationAttempt, CrockfordLower);
impl_mysql_token_from_row!(AgeVerificationAttemptToken);
impl_string_token!(AgeVerificationAttemptToken);
