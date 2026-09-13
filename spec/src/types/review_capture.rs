use ReviewCapture::*;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(ValueEnum, EnumIter, Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum ReviewCapture {
    RenderedText,
    PatchSnapshot,
    PatchSnapshotWithCoordinates,
}

impl ReviewCapture {
    pub fn preserves_exact_patch(self) -> bool {
        matches!(self, PatchSnapshot | PatchSnapshotWithCoordinates)
    }

    pub fn preserves_file_and_side(self) -> bool {
        matches!(self, PatchSnapshotWithCoordinates)
    }
}
