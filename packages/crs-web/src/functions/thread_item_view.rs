use crate::{ChatMessage, Disclosure, FileLinks, content_version, item_anchor, markdown_view};
use dioxus::prelude::*;
use serde_json::Value;

pub fn thread_item_view(turn_id: &str, mut item: Value, files: &FileLinks) -> Element {
    let version = content_version(&item);
    let Some(fields) = item.as_object_mut() else { return structured_value(&item) };
    let id = fields.remove("id").unwrap_or_default();
    let id = id.as_str().unwrap_or_default();
    let anchor = item_anchor(turn_id, id);
    let kind = fields.remove("type").unwrap_or_default();
    let speaker = match kind.as_str() {
        Some("userMessage") => Some("User"),
        Some("agentMessage") => Some("Assistant"),
        _ => None,
    };
    let body = match kind.as_str() {
        Some("userMessage") => {
            let content = fields.remove("content").unwrap_or_default();
            let inputs = content.as_array().into_iter().flatten().map(|input| {
                if let Some(text) = input.get("text").and_then(Value::as_str) {
                    rsx! {
                        {markdown_view(text, files)}
                        if let Some(spans) = input.get("text_elements").filter(|v| v.as_array().is_some_and(|a| !a.is_empty())) {
                            Disclosure { label: rsx! { "Text spans" }, {structured_value(spans)} }
                        }
                    }
                } else if let (Some(kind), Some(url)) = (input.get("type").and_then(Value::as_str), input.get("url").and_then(Value::as_str)) {
                    match kind {
                        "image" if url.starts_with("data:image/") => rsx! {
                            img { src: url, alt: "User image attachment", loading: "lazy" }
                            if let Some(detail) = input.get("detail").and_then(Value::as_str) { small { "{detail}" } }
                        },
                        "audio" if url.starts_with("data:audio/") => rsx! { audio { src: url, controls: true, preload: "none" } },
                        _ => structured_value(input),
                    }
                } else {
                    structured_value(input)
                }
            });
            rsx! { div { class: "user-bubble ms-auto box-border w-fit max-w-full rounded-3xl bg-neutral-800 px-4 py-2.5 text-white scheme-dark", {inputs} } }
        }
        Some("agentMessage" | "plan") => markdown_view(
            fields
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            files,
        ),
        _ => {
            for field in ["startedAtMs", "completedAtMs"] {
                fields.remove(field);
            }
            structured_value(&item)
        }
    };
    if let Some(speaker) = speaker {
        let client_id = item.get("clientId").and_then(Value::as_str);
        rsx! { ChatMessage { speaker, id: anchor, "data-item": id, "data-version": version, "data-client-id": client_id,
            {body}
        } }
    } else {
        let preview = item
            .get("text")
            .and_then(Value::as_str)
            .and_then(|text| text.trim().lines().next())
            .unwrap_or_else(|| kind.as_str().unwrap_or("Item"));
        rsx! { Disclosure { class: "activity-item my-3", id: anchor, "data-item": id, "data-version": version, summary_class: "list-inside truncate text-sm",
            label: rsx! { "{preview}" },
            {body}
        } }
    }
}

fn structured_value(value: &Value) -> Element {
    match value {
        Value::Object(fields) if !fields.is_empty() => rsx! { dl {
            for (name, value) in fields {
                dt { "{name}" }
                dd { {structured_value(value)} }
            }
        } },
        Value::Array(values) if !values.is_empty() => rsx! { ol {
            for value in values { li { {structured_value(value)} } }
        } },
        Value::String(text) => rsx! { pre { span { "{text}" } } },
        _ => rsx! { code { "{value}" } },
    }
}
