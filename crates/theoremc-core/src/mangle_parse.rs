//! String parsing for [`CanonicalActionName`].

use super::{CanonicalActionName, InvalidCanonicalActionName};

impl std::str::FromStr for CanonicalActionName {
    type Err = InvalidCanonicalActionName;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}
