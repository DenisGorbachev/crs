use crate::{Button, ButtonSize, ButtonVariant, MAX_RECORDING_BYTES, icon};
use dioxus::prelude::*;

#[component]
pub fn VoiceControls() -> Element {
    let configured = use_context::<bool>();
    let record_label = if configured { "Record" } else { "Set OPENAI_API_KEY on the CRS-Web server to record" };
    rsx! { div { class: "ms-auto flex flex-wrap items-center justify-end gap-1", "data-voice": "{MAX_RECORDING_BYTES}", "data-configured": configured.then_some(""),
        for (action, label, path) in [
            ("record", record_label, "M9 5a3 3 0 0 1 6 0v6a3 3 0 0 1-6 0ZM5 11a7 7 0 0 0 14 0M12 18v4m-4 0h8"),
            ("stop", "Stop", "M6 6h12v12H6Z"),
            ("cancel", "Cancel recording", "m6 6 12 12M18 6 6 18"),
            ("transcribe", "Transcribe", "M5 4h14v16H5ZM8 8h8m-8 4h8m-8 4h5"),
            ("retry", "Retry transcription", "M4 10a8 8 0 1 1 1 7M4 4v6h6"),
            ("discard", "Discard recording", "M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15"),
        ] {
            Button { hidden: true, variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, r#type: "button", title: label, aria_label: label, "data-voice-action": action, {icon(path)} }
        }
    } }
}
