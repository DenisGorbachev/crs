use crate::{ChatMessage, Disclosure, content_version, markdown_view};
use dioxus::prelude::*;
use serde_json::Value;
use std::mem::take;

pub fn thread_item_view(mut item: Value) -> Element {
    let version = content_version(&item);
    let Some(fields) = item.as_object_mut() else { return structured_value(&item) };
    let id = fields
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let kind = fields.remove("type").unwrap_or_default();
    let speaker = match kind.as_str() {
        Some("userMessage") => Some("User"),
        Some("agentMessage") => Some("Assistant"),
        _ => None,
    };
    let preview = ["command", "query", "tool", "name", "path", "text"]
        .into_iter()
        .filter_map(|key| fields.get(key).and_then(Value::as_str))
        .find_map(|value| value.trim().lines().next())
        .unwrap_or_else(|| kind.as_str().unwrap_or("Item"))
        .to_owned();
    let body = match kind.as_str() {
        Some("userMessage") => {
            let content = fields.remove("content").unwrap_or_default();
            let inputs = content.as_array().into_iter().flatten().map(|input| {
                if let Some(text) = input.get("text").and_then(Value::as_str) {
                    rsx! {
                        {markdown_view(text)}
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
        Some("agentMessage" | "plan") => {
            let text = fields.remove("text").unwrap_or_default();
            markdown_view(text.as_str().unwrap_or_default())
        }
        Some("commandExecution") => {
            let command = fields.remove("command").unwrap_or_default();
            let output = fields.remove("aggregatedOutput").unwrap_or_default();
            let command = command.as_str().unwrap_or_default();
            rsx! { pre { code { "{command}" } } if let Some(output) = output.as_str() { pre { "{output}" } } }
        }
        _ => {
            let mut payload = take(fields);
            for field in ["id", "startedAtMs", "completedAtMs"] {
                if let Some(value) = payload.remove(field) {
                    fields.insert(field.into(), value);
                }
            }
            structured_value(&Value::Object(payload))
        }
    };
    if let Some(speaker) = speaker {
        let client_id = item.get("clientId").and_then(Value::as_str);
        rsx! { ChatMessage { speaker, "data-item": id, "data-version": version, "data-client-id": client_id,
            {body}
            {item_details(&item)}
        } }
    } else {
        rsx! { Disclosure { class: "activity-item my-3", "data-item": id, "data-version": version, summary_class: "list-inside truncate text-sm",
            label: rsx! {
                if kind == "commandExecution" { code { "{preview}" } } else { "{preview}" }
            },
            {body}
            {item_details(&item)}
        } }
    }
}

fn item_details(item: &Value) -> Element {
    rsx! { Disclosure { class: "item-details mt-1 text-xs", label: rsx! { "Item details" }, pre { "{item:#}" } } }
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
