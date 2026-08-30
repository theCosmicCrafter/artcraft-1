use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// The primary key for `media_file_local_index_links` - the thin join between a
/// cloud `media_files` row and the local Companion library entry it was
/// promoted from. ArtCraft Studio - see artcraft-studio-plan/SYSTEM_DESIGN.md §2.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct MediaFileLocalIndexLinkToken(pub String);

impl_crockford_generator!(MediaFileLocalIndexLinkToken, 32usize, TokenPrefix::MediaFileLocalIndexLink, CrockfordLower);
impl_mysql_token_from_row!(MediaFileLocalIndexLinkToken);
impl_string_token!(MediaFileLocalIndexLinkToken);
