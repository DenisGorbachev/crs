use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "kebab-case")]
pub enum MessageAction {
    Steer,
    Edit,
    Cancel,
}
