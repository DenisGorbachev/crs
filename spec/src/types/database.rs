#[allow(unused_imports)]
use Database::*;
use strum::Display;

#[derive(Display, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Clone, Copy, Debug)]
pub enum Database {
    File,
    #[default]
    Fjall,
}

impl Database {}
