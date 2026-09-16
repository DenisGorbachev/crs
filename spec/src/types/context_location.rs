#[allow(unused_imports)]
use ContextLocation::*;
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Display, EnumIter, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub enum ContextLocation {
    Database,
    ShellExportedVars,
}

impl ContextLocation {
    pub fn decide() -> Option<Self> {
        Self::iter().find(|x| Self::iter().all(|other| x.subsumes(other)))
    }

    /// Must be reflexive
    pub fn subsumes(self, other: Self) -> bool {
        match (self, other) {
            (Database, ShellExportedVars) => true,
            (Database, Database) => true,
            (ShellExportedVars, Database) => false,
            (ShellExportedVars, ShellExportedVars) => true,
        }
    }

    pub fn is_available_in_new_shell(self) -> bool {
        match self {
            ShellExportedVars => false,
            Database => true,
        }
    }

    pub fn supports_multiple_contexts(self) -> bool {
        match self {
            ShellExportedVars => true,
            Database => {
                // just create multiple Context values
                true
            }
        }
    }

    pub fn supports_inheritance(self) -> bool {
        match self {
            ShellExportedVars => false,
            Database => {
                // can implement it
                true
            }
        }
    }
}
