#[allow(unused_imports)]
use CodexCommunicationProtocol::*;
use strum::Display;

#[derive(Display, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub enum CodexCommunicationProtocol {
    AppServer,
    HomeDir,
}

impl CodexCommunicationProtocol {
    pub fn works_over_ssh(self) -> bool {
        match self {
            AppServer => true,
            HomeDir => false,
        }
    }

    pub fn works_for_local_sandbox(self) -> bool {
        match self {
            AppServer => true,
            HomeDir => {
                // requires mounting ~/.codex-sandbox as ~/.codex in sandbox
                // requires setting CODEX_HOME before loading `Config`
                true
            }
        }
    }

    pub fn allows_querying_state_db_directly(self) -> bool {
        match self {
            AppServer => false,
            HomeDir => true,
        }
    }
}
