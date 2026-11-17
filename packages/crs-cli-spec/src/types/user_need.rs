#[allow(unused_imports)]
use UserNeed::*;
use strum::Display;

#[derive(Display, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub enum UserNeed {
    Focus,
    Automation,
}

impl UserNeed {
    pub fn description(self) -> &'static str {
        match self {
            Focus => include_str!("./user_need/focus.md"),
            Automation => {
                todo!()
            }
        }
    }
}
