use crate::markdown_view;
use dioxus::prelude::*;
use serde_json::Value;
use std::mem::take;

pub fn thread_item_view(mut item: Value) -> Element {
    let Some(fields) = item.as_object_mut() else { return structured_value(&item) };
    let kind = fields.remove("type").unwrap_or_default();
    let (heading, body) = match kind.as_str() {
        Some("userMessage") => {
            let content = fields.remove("content").unwrap_or_default();
            let inputs = content.as_array().into_iter().flatten().map(|input| {
                if let Some(text) = input.get("text").and_then(Value::as_str) {
                    rsx! {
                        {markdown_view(text)}
                        if let Some(spans) = input.get("text_elements").filter(|v| v.as_array().is_some_and(|a| !a.is_empty())) {
                            details { summary { "Text spans" } {structured_value(spans)} }
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
            ("User", rsx! { div { class: "user-bubble ms-auto box-border w-fit max-w-full rounded-3xl bg-neutral-800 px-4 py-2.5 text-white scheme-dark", {inputs} } })
        }
        Some(kind @ ("agentMessage" | "plan")) => {
            let text = fields.remove("text").unwrap_or_default();
            (if kind == "agentMessage" { "Assistant" } else { "Plan" }, markdown_view(text.as_str().unwrap_or_default()))
        }
        Some("commandExecution") => {
            let command = fields.remove("command").unwrap_or_default();
            let output = fields.remove("aggregatedOutput").unwrap_or_default();
            let command = command.as_str().unwrap_or_default();
            let preview = command.trim_start().lines().next().unwrap_or("Command");
            return rsx! { details { class: "command-item my-3",
                summary { class: "list-inside truncate text-sm", code { "{preview}" } }
                pre { code { "{command}" } }
                if let Some(output) = output.as_str() { pre { "{output}" } }
                {item_details(&item)}
            } };
        }
        kind => {
            let mut payload = take(fields);
            for field in ["id", "startedAtMs", "completedAtMs"] {
                if let Some(value) = payload.remove(field) {
                    fields.insert(field.into(), value);
                }
            }
            (kind.unwrap_or("Item"), structured_value(&Value::Object(payload)))
        }
    };
    let speaker = matches!(kind.as_str(), Some("userMessage" | "agentMessage")).then_some(heading);
    rsx! { article { class: "my-5", "data-response": speaker, aria_label: speaker,
        if speaker.is_none() { h3 { "{heading}" } }
        {body}
        {item_details(&item)}
    } }
}

fn item_details(item: &Value) -> Element {
    rsx! { details { class: "item-details mt-1 text-xs", summary { "Item details" } pre { "{item:#}" } } }
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
