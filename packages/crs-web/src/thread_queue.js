export function threadQueue(root, refresh) {
    const panel = root.querySelector("#pending-messages");
    const template = panel.querySelector("template");
    const turns = root.querySelector("#turns");
    const form = root.querySelector(".thread-draft");
    const status = form.querySelector('[role="status"]');
    const rows = new Map();
    const busy = new Map();
    const promoted = new Map();
    let version = null;
    let messages = [];

    function release(row) {
        const download = row.querySelector("[download]");
        if (download.hasAttribute("href")) {
            URL.revokeObjectURL(download.href);
            download.removeAttribute("href");
        }
        download.hidden = true;
    }

    function values(editor) {
        return [...editor.querySelectorAll("textarea")].map(input => input.value);
    }

    function recoverEdit(editor) {
        if (editor.hidden || JSON.stringify(values(editor)) === editor.dataset.initial) return;
        editor.replaceWith(template.content.querySelector("form").cloneNode(true));
        editor.dataset.recovered = "";
        editor.querySelector('[type="submit"]').textContent = "Append to draft";
        for (const input of editor.querySelectorAll("textarea")) input.ariaLabel = "Unsent edit";
        for (const button of editor.querySelectorAll("button")) button.disabled = false;
        panel.before(editor);
        status.textContent = "The message left the queue. Your unsent edit is above the composer.";
        editor.addEventListener("submit", event => {
            event.preventDefault();
            if (form.elements.message.readOnly || form.querySelector("fieldset").disabled) return;
            const draft = form.elements.message;
            draft.value += (draft.value ? "\n\n" : "") + values(editor).join("\n");
            draft.dispatchEvent(new Event("input", { bubbles: true }));
            editor.remove();
            draft.focus();
        });
        editor.querySelector('[data-queue-action="discard"]').addEventListener("click", () => editor.remove());
    }

    function render() {
        const visible = new Set();
        let previous = template;
        for (const item of messages) {
            const id = item.client_id;
            if (visible.has(id)) continue;
            if (turns.querySelector('[data-client-id="' + CSS.escape(id) + '"]')) {
                promoted.delete(item.id);
                continue;
            }
            visible.add(id);
            let row = rows.get(id);
            if (!row) {
                row = template.content.firstElementChild.cloneNode(true);
                row.dataset.clientId = id;
                rows.set(id, row);
            }
            if (previous.nextElementSibling !== row) previous.after(row);
            previous = row;
            const editor = row.querySelector("form");
            const readOnly = form.querySelector("fieldset").disabled;
            const action = busy.get(id);
            const steering = !!item.turn_id || action === "steer";
            if (!editor.hidden && (steering || readOnly)) {
                recoverEdit(editor);
                if (row.contains(editor)) editor.hidden = true;
            }
            row.querySelector(".user-bubble").textContent = item.text;
            const turn = item.turn_id && turns.querySelector('[data-turn-id="' + CSS.escape(item.turn_id) + '"]');
            const uncertain = item.turn_id && (!item.confirmed || turn?.dataset.inProgress === "false");
            row.querySelector("[data-queue-state]").textContent = uncertain ? "Steering delivery unconfirmed" : steering ? "Steering" : "Queued";
            row.querySelector("[data-queue-actions]").hidden = steering || (!editor.hidden && row.contains(editor)) || readOnly;
            const download = row.querySelector("[download]");
            if (uncertain && item.recovery) {
                if (!download.hasAttribute("href")) download.href = URL.createObjectURL(new Blob([item.recovery], {type:"application/json"}));
                download.hidden = false;
            } else {
                release(row);
            }
            for (const button of row.querySelectorAll("button")) {
                button.disabled = !!action;
            }
        }
        for (const [id, row] of rows) {
            if (!visible.has(id)) {
                const editor = row.querySelector("form");
                if (editor) recoverEdit(editor);
                release(row);
                row.remove();
                rows.delete(id);
            }
        }
    }

    async function change(action, item, row) {
        const id = item.client_id;
        if (busy.has(id) || item.turn_id) return;
        const editor = row.querySelector("form");
        const inputs = editor.querySelectorAll("textarea");
        const body = new URLSearchParams({
            source: turns.dataset.source, thread_id: turns.dataset.thread, form_key: form.elements.form_key.value,
            action, queued_id: item.id,
        });
        if (action === "edit") {
            body.set("original", editor.dataset.original);
            body.set("message", JSON.stringify(values(editor)));
            for (const input of inputs) input.readOnly = true;
        }
        busy.set(id, action);
        render();
        try {
            const response = await fetch(form.action, {method: "POST", headers: {Accept: "application/json"}, body});
            if (!response.ok) {
                status.textContent = await response.text();
                return;
            }
            const updated = await response.json();
            if (updated) {
                if (action === "steer") promoted.set(item.id, updated);
                const at = messages.findIndex(entry => entry.id === item.id);
                if (at < 0) messages.push(updated);
                else messages[at] = updated;
            } else {
                messages = messages.filter(entry => entry.id !== item.id);
            }
            if (action === "edit") {
                if (editor.hasAttribute("data-recovered")) editor.remove();
                else editor.hidden = true;
            }
            status.textContent = "";
        } catch {
            status.textContent = "Could not confirm the change. Check the thread before retrying.";
        } finally {
            busy.delete(id);
            for (const input of inputs) input.readOnly = false;
            version = null;
            render();
            refresh();
        }
    }

    function itemFor(row) {
        return messages.find(item => item.client_id === row.dataset.clientId);
    }
    panel.addEventListener("click", event => {
        const button = event.target.closest("[data-queue-action]");
        if (!button) return;
        const row = button.closest("[data-client-id]");
        const item = itemFor(row);
        if (!item || item.turn_id || busy.has(item.client_id)) return;
        const editor = row.querySelector("form");
        switch (button.dataset.queueAction) {
            case "edit": {
                const parts = item.edit_texts ?? [item.text];
                const fields = parts.length ? parts : [""];
                editor.dataset.original = JSON.stringify(parts);
                editor.dataset.initial = JSON.stringify(fields);
                editor.hidden = false;
                editor.querySelectorAll("textarea").forEach(input => input.remove());
                for (const [index, part] of fields.entries()) {
                    const input = template.content.querySelector("textarea").cloneNode(true);
                    input.value = part;
                    input.required = !item.edit_texts;
                    if (fields.length > 1) input.ariaLabel = "Edit queued message, part " + (index + 1);
                    editor.insertBefore(input, editor.lastElementChild);
                }
                render();
                editor.querySelector("textarea").focus();
                break;
            }
            case "discard":
                editor.hidden = true;
                render();
                button.closest("article").querySelector('[data-queue-action="edit"]').focus();
                break;
            default:
                void change(button.dataset.queueAction, item, row);
        }
    });
    panel.addEventListener("submit", event => {
        event.preventDefault();
        const row = event.target.closest("[data-client-id]");
        const item = itemFor(row);
        if (item) void change("edit", item, row);
    });
    return {
        get version() { return version; },
        accepted(item) {
            if (item && !messages.some(entry => entry.id === item.id)) messages.push(item);
            version = null;
            render();
            refresh();
        },
        sync(snapshot) {
            if (snapshot) {
                version = snapshot.version;
                messages = snapshot.messages.map(item => item.turn_id ? item : promoted.get(item.id) ?? item);
                for (const [id, item] of promoted) {
                    if (turns.querySelector('[data-client-id="' + CSS.escape(item.client_id) + '"]')) promoted.delete(id);
                }
            }
            render();
        },
    };
}
