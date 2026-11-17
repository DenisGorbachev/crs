#[allow(unused_imports)]
use ApprovalLocation::*;
use strum::Display;

#[derive(Display, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Clone, Copy, Debug)]
pub enum ApprovalLocation {
    #[default]
    Home,
    Server,
    Repository,
}

impl ApprovalLocation {
    pub fn requires_explicit_commit(self) -> bool {
        match self {
            Home => false,
            Server => false,
            Repository => true,
        }
    }

    pub fn requires_backup(self) -> bool {
        match self {
            Home => true,
            Server => true,
            Repository => false,
        }
    }

    pub fn supports_multiple_users(self) -> bool {
        match self {
            Home => false,
            Server => true,
            Repository => {
                // requires to resolve merge conflicts
                true
            }
        }
    }
}
