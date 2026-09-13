use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Clone, Copy, Debug)]
#[serde(transparent)]
pub struct ReviewKeystrokes(pub u64);

impl From<u64> for ReviewKeystrokes {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
