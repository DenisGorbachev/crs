use ReviewDelivery::*;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(ValueEnum, EnumIter, Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum ReviewDelivery {
    ClipboardPaste,
    ReviewFileAttachment,
    AgentResumeCommand,
    AgentThreadApi,
}

impl ReviewDelivery {
    pub fn requires_manual_agent_focus(self) -> bool {
        matches!(self, ClipboardPaste | ReviewFileAttachment)
    }

    pub fn requires_agent_adapter(self) -> bool {
        matches!(self, AgentResumeCommand | AgentThreadApi)
    }

    pub fn workflow(self) -> &'static str {
        match self {
            ClipboardPaste => "Copy the assembled review, focus the originating agent thread, paste, and send.",
            ReviewFileAttachment => "Persist the assembled review, attach that file in the originating thread, and send a request to address it.",
            AgentResumeCommand => "Bind Finish to persist the review and invoke the agent's supported resume command with the recorded thread identifier and review as data.",
            AgentThreadApi => "Bind Finish to persist the review and submit it through the agent's supported API to the recorded originating thread.",
        }
    }

    pub fn tradeoff(self) -> &'static str {
        match self {
            ClipboardPaste => "Broad compatibility; focus, paste, and send add keystrokes, and the user must identify the correct thread.",
            ReviewFileAttachment => "Useful for large reviews; attachment navigation costs keystrokes and the agent must be able to read the file.",
            AgentResumeCommand => "Can combine completion and delivery into one binding; requires a compatible agent CLI and coordination with any active turn.",
            AgentThreadApi => "Can deliver without changing focus; requires a supported API, explicit routing, and an acknowledgment before marking the review sent.",
        }
    }
}
