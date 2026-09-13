use crate::{ReviewCapture, ReviewEffort, ReviewSelection, ReviewSurface};

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct ReviewSolutionProperties {
    pub surface: ReviewSurface,
    pub effort: ReviewEffort,
    pub capture: ReviewCapture,
    pub selection: ReviewSelection,
    pub keyboard_selection: bool,
    pub requires_hosted_review: bool,
    pub workflow: &'static str,
    pub tradeoff: &'static str,
}
