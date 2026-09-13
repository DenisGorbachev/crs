use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(ValueEnum, EnumIter, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum ReviewEffort {
    Configuration,
    Adapter,
    Extension,
    Application,
}
