#[allow(unused_imports)]
use ContextStorage::*;
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Display, EnumIter, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub enum ContextStorage {
    Database,
    ShellGlobalVarMap,
}

impl ContextStorage {
    pub fn decide() -> Option<Self> {
        Self::iter().find(|x| Self::iter().all(|other| x.subsumes(other)))
    }

    /// Must be reflexive
    pub fn subsumes(self, other: Self) -> bool {
        match (self, other) {
            (Database, ShellGlobalVarMap) => {
                // User can set CRS_CONTEXT_ID to switch context
                true
            }
            (Database, Database) => true,
            (ShellGlobalVarMap, Database) => false,
            (ShellGlobalVarMap, ShellGlobalVarMap) => true,
        }
    }

    pub fn is_available_in_new_shell(self) -> bool {
        match self {
            ShellGlobalVarMap => false,
            Database => true,
        }
    }

    pub fn supports_multiple_contexts(self) -> bool {
        match self {
            ShellGlobalVarMap => true,
            Database => {
                // just create multiple Context values
                true
            }
        }
    }

    pub fn supports_inheritance(self) -> bool {
        match self {
            ShellGlobalVarMap => false,
            Database => {
                // can implement it
                true
            }
        }
    }
}
