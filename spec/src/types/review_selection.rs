use ReviewSelection::*;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(ValueEnum, EnumIter, Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum ReviewSelection {
    SingleLine,
    Characters,
    Lines,
    WholeHunks,
}

impl ReviewSelection {
    pub fn supports_partial_hunks(self) -> bool {
        matches!(self, SingleLine | Characters | Lines)
    }

    pub fn supports_multiline_ranges(self) -> bool {
        matches!(self, Characters | Lines)
    }
}
