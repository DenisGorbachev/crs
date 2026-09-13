use crate::ReviewEffort;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Copy, Debug)]
pub struct ReviewRequirements {
    pub maximum_effort: ReviewEffort,
    pub keyboard_only: bool,
    pub local_only: bool,
    pub require_exact_patch: bool,
    pub require_file_and_side: bool,
    pub require_partial_hunks: bool,
    pub require_multiline_ranges: bool,
    pub require_character_selection: bool,
}
