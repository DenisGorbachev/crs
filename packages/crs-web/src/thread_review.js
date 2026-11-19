const segmenter = typeof Intl.Segmenter === "function" && new Intl.Segmenter(undefined, { granularity: "sentence" });
const quotations = new WeakMap();

function mark(element, quotation, first = true) {
    element.classList.add("quote-chunk");
    if (first) element.dataset.quoteStart = "";
    quotations.set(element, quotation);
    return element;
}

function chunk(text, quotation, first) {
    const span = mark(document.createElement("span"), quotation, first);
    span.textContent = text;
    return span;
}

function prepareProse(root) {
    let pieces = [];
    let text = "";
    function flush() {
        const prose = text.replace(/\n/g, " ");
        const sentences = segmenter ? segmenter.segment(prose) : prose.matchAll(/[^.!?。！？]+[.!?。！？]*\s*|[.!?。！？]+/gu);
        const iterator = sentences[Symbol.iterator]();
        let sentence = iterator.next().value;
        let first = true;
        for (const { node, start, length } of pieces) {
            const end = start + length;
            let pos = start;
            const fragment = document.createDocumentFragment();
            while (sentence && pos < end) {
                const original = sentence.segment ?? sentence[0];
                const boundary = sentence.index + original.length;
                if (pos >= boundary) {
                    sentence = iterator.next().value;
                    first = true;
                    continue;
                }
                const part = text.slice(pos, Math.min(boundary, end));
                if (node.nodeType === Node.ELEMENT_NODE) {
                    if (node.matches("code")) {
                        mark(node, { quotation: () => node.textContent, language: "" });
                    } else {
                        mark(node, text.slice(sentence.index, Math.max(boundary, end)).trim(), first);
                        first = false;
                    }
                    break;
                }
                if (part.trim()) {
                    fragment.append(chunk(part, text.slice(sentence.index, boundary).trim(), first));
                    first = false;
                } else {
                    fragment.append(part);
                }
                pos += part.length;
            }
            if (node.nodeType === Node.TEXT_NODE) node.replaceWith(fragment);
        }
        pieces = [];
        text = "";
    }
    function visit(element) {
        for (const node of Array.from(element.childNodes)) {
            if (node.nodeType === Node.TEXT_NODE) {
                pieces.push({ node, start: text.length, length: node.length });
                text += node.data;
            } else if (node.nodeType === Node.ELEMENT_NODE) {
                if (node.matches("br")) {
                    text += "\n";
                } else if (node.matches("img, code")) {
                    const label = node.textContent || node.alt || node.closest("a")?.getAttribute("href") || "";
                    pieces.push({ node, start: text.length, length: label.length });
                    text += label;
                } else if (node.matches("p, li, h1, h2, h3, h4, h5, h6, td, th, ul, ol, table, blockquote, hr, pre")) {
                    flush();
                    if (!node.matches("pre")) visit(node);
                    flush();
                } else {
                    visit(node);
                }
            }
        }
    }
    visit(root);
    flush();
}

function prepareCode(block) {
    const bytes = new TextEncoder().encode(block.textContent);
    const decoder = new TextDecoder("utf-8", { ignoreBOM: true });
    const slice = (start, end) => decoder.decode(bytes.subarray(start, end));
    const fragment = document.createDocumentFragment();
    const language = block.dataset.language;
    let position = 0;
    function append(start, end, quotation) {
        const part = slice(start, end);
        fragment.append(part.trim() ? chunk(part, { quotation, language }) : part);
    }
    function gap(end) {
        while (position < end) {
            const from = position;
            const lf = bytes.subarray(from, end).indexOf(10);
            position = lf < 0 ? end : from + lf;
            append(from, position, () => {
                const start = from === 0 ? 0 : bytes.lastIndexOf(10, from - 1) + 1;
                const stop = bytes.indexOf(10, from);
                return slice(start, stop < 0 ? bytes.length : stop);
            });
            if (lf >= 0) {
                fragment.append("\n");
                position++;
            }
        }
    }
    for (const range of block.dataset.ranges.trim().split(/\s+/).filter(Boolean)) {
        const [start, end, quoteStart, quoteEnd] = range.split(":").map(Number);
        gap(start);
        append(start, end, () => slice(quoteStart, quoteEnd));
        position = end;
    }
    gap(bytes.length);
    block.replaceChildren(fragment);
}

function formatQuote(value) {
    let text;
    if (typeof value === "string") {
        text = value;
    } else {
        const source = value.quotation().replace(/\r\n?/g, "\n");
        let length = 3;
        for (const run of source.matchAll(/`+/g)) length = Math.max(length, run[0].length + 1);
        const fence = "`".repeat(length);
        text = fence + value.language + "\n" + source + (source.endsWith("\n") ? "" : "\n") + fence;
    }
    return text.replace(/\r\n?/g, "\n").split("\n").map(line => "> " + line).join("\n");
}

function navigation(root) {
    const controls = root.querySelector(".controls");
    if (!controls) return;
    const bar = controls.querySelector("nav");
    let selected;
    let lastScroll;
    function select(message) {
        selected?.removeAttribute("aria-current");
        selected = message;
        selected?.setAttribute("aria-current", "true");
    }
    controls.hidden = false;
    bar.addEventListener("click", event => {
        const direction = event.target.closest("button")?.dataset.go;
        switch (direction) {
            case "first": select(null); window.scrollTo(0, 0); break;
            case "last": select(null); window.scrollTo(0, document.documentElement.scrollHeight); break;
            case "previous":
            case "next": {
                const top = 8;
                const messages = Array.from(root.querySelectorAll("article[data-response]"));
                selected = root.querySelector('article[aria-current="true"]');
                const previous = direction === "previous";
                let target;
                if (selected && Math.abs(window.scrollY - lastScroll) < 2) {
                    target = messages[messages.indexOf(selected) + (previous ? -1 : 1)];
                } else {
                    target = previous
                        ? messages.findLast(message => message.getBoundingClientRect().bottom < top + 2)
                        : messages.find(message => message.getBoundingClientRect().top > top + 2);
                }
                if (target) {
                    select(target);
                    target.scrollIntoView({ block: "start" });
                    lastScroll = window.scrollY;
                }
                break;
            }
        }
    });
}

function readStorage(area, key) {
    try { return window[area].getItem(key); } catch { return null; }
}

function writeStorage(area, key, value) {
    try {
        if (value) window[area].setItem(key, value);
        else window[area].removeItem(key);
        return true;
    } catch {
        return false;
    }
}

function threadDraft(form, editor, status) {
    const key = "crs.draft." + JSON.stringify([form.elements.source.value, form.elements.thread_id.value]);
    let shared = readStorage("localStorage", key) ?? "";
    const privateCopy = readStorage("sessionStorage", key);
    let conflict = privateCopy !== null && privateCopy !== shared;
    const conflictNotice = "Another tab changed this draft. Your edits are kept in this tab.";
    editor.value = privateCopy ?? shared;
    if (conflict) status.textContent = conflictNotice;

    function save() {
        const savedHere = writeStorage("sessionStorage", key, editor.value);
        const current = readStorage("localStorage", key) ?? "";
        if (conflict || current !== shared) {
            conflict = editor.value !== current;
            shared = current;
        }
        if (conflict) {
            status.textContent = savedHere ? conflictNotice : "Keep this tab open. Browser storage could not save these edits.";
            return;
        }
        if (!writeStorage("localStorage", key, editor.value)) {
            status.textContent = savedHere ? "Draft saved for this tab only." : "Keep this tab open. Browser storage could not save these edits.";
            return;
        }
        shared = editor.value;
    }
    window.addEventListener("storage", event => {
        if (event.storageArea !== localStorage || event.key !== key) return;
        const current = readStorage("localStorage", key) ?? "";
        if (current === shared || editor.readOnly) return;
        if (!conflict && editor.value === shared && !(editor === document.activeElement && readStorage("sessionStorage", key) !== null)) {
            editor.value = current;
            writeStorage("sessionStorage", key, "");
        } else if (editor.value !== current) {
            conflict = true;
            status.textContent = conflictNotice;
        }
        shared = current;
    });
    editor.addEventListener("input", save);
    return {
        save,
        acknowledge(draft) {
            if (readStorage("localStorage", key) === draft) writeStorage("localStorage", key, "");
            shared = readStorage("localStorage", key) ?? "";
            conflict = shared !== "";
            writeStorage("sessionStorage", key, "");
        },
    };
}

const prepared = new WeakSet();

function prepareQuotes(scope, mode) {
    if (mode !== "off") {
        for (const body of scope.querySelectorAll(".markdown")) {
            if (prepared.has(body)) continue;
            prepareProse(body);
            for (const code of body.querySelectorAll("pre code[data-ranges]")) prepareCode(code);
            prepared.add(body);
        }
    }
    for (const part of scope.querySelectorAll("[data-quote-start]")) {
        if (mode === "off") {
            part.removeAttribute("tabindex");
            part.removeAttribute("role");
        } else if (!part.closest("a")) {
            part.tabIndex = 0;
            part.setAttribute("role", "button");
        }
    }
}

function atTranscriptBottom() {
    return document.documentElement.scrollHeight - innerHeight - scrollY <= 4;
}

function composer(root, form) {
    const editor = form.elements.message;
    const modeButton = form.querySelector("#quote-mode");
    const hint = form.querySelector("#quote-hint");
    const preference = form.querySelector("#quote-default");
    const status = form.querySelector('[role="status"]');
    const send = form.querySelector('[type="submit"]');
    const draft = threadDraft(form, editor, status);
    editor.addEventListener("focus", () => form.classList.add("editing"));
    form.addEventListener("focusout", event => {
        if (!form.contains(event.relatedTarget)) form.classList.remove("editing");
    });
    const preferenceKey = "crs.quoteByDefault";
    preference.checked = readStorage("localStorage", preferenceKey) === "on";
    preference.addEventListener("change", () => {
        if (!writeStorage("localStorage", preferenceKey, preference.checked ? "on" : "off")) {
            status.textContent = "Could not save the default quote mode.";
        }
    });
    let mode;
    let armedAt = -Infinity;
    function setMode(next) {
        mode = next;
        root.dataset.quoteMode = mode;
        const label = { off: "Quote", once: "Quote once", persistent: "Quote multiple" }[mode];
        modeButton.setAttribute("aria-label", label);
        hint.textContent = { off: "Tap once for one quote, twice for multiple.", once: "Select a passage to quote.", persistent: "Select passages to quote." }[mode];
        modeButton.setAttribute("aria-pressed", String(mode !== "off"));
        prepareQuotes(root, mode);
    }
    const fieldset = form.querySelector("fieldset");
    function setWritable(writable) {
        if (writable === fieldset.disabled || !mode) {
            fieldset.hidden = fieldset.disabled = !writable;
            root.querySelector("#direct-input-notice").hidden = writable;
            setMode(writable && preference.checked ? "persistent" : "off");
        }
    }
    setWritable(!fieldset.disabled);
    modeButton.addEventListener("click", () => {
        const now = performance.now();
        if (mode === "off") {
            armedAt = now;
            setMode("once");
        } else {
            setMode(mode === "once" && now - armedAt <= 400 ? "persistent" : "off");
        }
    });
    function insert(event) {
        if (mode === "off" || editor.readOnly || fieldset.disabled) return;
        const link = event.target.closest(".markdown a");
        const part = event.target.closest(".quote-chunk") ?? link?.querySelector(".quote-chunk");
        if (!part && !link) return;
        event.preventDefault();
        const text = editor.value;
        const separator = !text || text.endsWith("\n\n") ? "" : text.endsWith("\n") ? "\n" : "\n\n";
        editor.value = text + separator + formatQuote(part ? quotations.get(part) : link.href) + "\n\n";
        draft.save();
        if (mode === "once") setMode("off");
        editor.focus({ preventScroll: true });
        editor.setSelectionRange(editor.value.length, editor.value.length);
        editor.scrollTop = editor.scrollHeight;
    }
    root.addEventListener("click", insert);
    root.addEventListener("keydown", event => {
        if (event.key === "Escape") setMode("off");
        else if (event.key === "Enter" || event.key === " ") insert(event);
    });
    form.addEventListener("submit", async event => {
        event.preventDefault();
        if (send.disabled || fieldset.disabled) return;
        const submitted = editor.value;
        const body = new URLSearchParams(new FormData(form));
        send.disabled = true;
        editor.readOnly = true;
        status.textContent = "Sending…";
        try {
            const response = await fetch(form.action, { method: "POST", headers: { Accept: "application/json" }, body });
            if (!response.ok) {
                status.textContent = await response.text();
                return;
            }
            const messageId = await response.json();
            const follow = atTranscriptBottom();
            draft.acknowledge(submitted);
            editor.value = "";
            status.textContent = "";
            if (!root.querySelector('#turns [data-client-id="' + CSS.escape(messageId) + '"]')) {
                const pending = root.querySelector("#pending-messages");
                const row = pending.querySelector("template").content.cloneNode(true);
                row.firstElementChild.dataset.clientId = messageId;
                row.querySelector(".user-bubble").textContent = submitted;
                pending.append(row);
            }
            if (follow) window.scrollTo(0, document.documentElement.scrollHeight);
        } catch {
            status.textContent = "Could not confirm delivery. Check the thread before sending again.";
        } finally {
            send.disabled = false;
            editor.readOnly = false;
        }
    });
    root.classList.add("has-draft");
    new ResizeObserver(() => root.style.setProperty("--draft-height", form.offsetHeight + "px")).observe(form);
    return setWritable;
}

function patchTurn(current, next) {
    if (current.id !== next.id) {
        current.replaceWith(next);
        return;
    }
    const key = el => el.hasAttribute("data-item") ? "item:" + el.dataset.item : el.tagName + (el.getAttribute("role") ?? "");
    const previous = new Map(Array.from(current.children, el => [key(el), el]));
    const children = Array.from(next.children);
    for (const [index, incoming] of children.entries()) {
        const old = previous.get(key(incoming));
        let node = incoming;
        if (old?.dataset.version && old.dataset.version === incoming.dataset.version) {
            node = old;
        } else if (old) {
            if (old.matches("details[open]")) node.open = true;
            const disclosures = node.querySelectorAll("details");
            old.querySelectorAll("details").forEach((detail, i) => {
                if (detail.open && disclosures[i]) disclosures[i].open = true;
            });
            if (old.ariaCurrent === "true") node.ariaCurrent = "true";
            if (old.isEqualNode(node)) node = old;
            else old.replaceWith(node);
        }
        const at = current.children[index];
        if (node !== at) current.insertBefore(node, at ?? null);
    }
    while (current.children.length > children.length) current.lastElementChild.remove();
    current.dataset.version = next.dataset.version;
}

function liveTranscript(root, setWritable) {
    const turns = root.querySelector("#turns");
    if (!turns) return;
    const metadata = root.querySelector("#thread-metadata");
    const sync = async () => {
        const response = await fetch("/api/thread-updates", {
            signal: AbortSignal.timeout(30000),
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({source: turns.dataset.source, id: turns.dataset.thread, header: metadata.dataset.version, versions: Array.from(turns.children, el => el.dataset.version)}),
        });
        if (response.status === 204) return;
        if (!response.ok) throw new Error("Thread updates returned HTTP " + response.status);
        const [count, changed, header] = await response.json();
        const follow = atTranscriptBottom();
        if (header) {
            metadata.textContent = header.metadata;
            metadata.dataset.version = header.version;
            root.querySelector("#thread-title").textContent = header.title;
            document.title = header.title + " · CRS-Web";
            root.querySelector("#archive-state").hidden = !header.archived;
            setWritable(header.writable);
        }
        for (const [index, html] of changed) {
            const template = document.createElement("template");
            template.innerHTML = html;
            const next = template.content.firstElementChild;
            const current = turns.children[index];
            if (current) patchTurn(current, next);
            else turns.append(next);
            if (root.dataset.quoteMode && root.dataset.quoteMode !== "off") prepareQuotes(turns.children[index], root.dataset.quoteMode);
        }
        while (turns.childElementCount > count) turns.lastElementChild.remove();
        for (const message of root.querySelectorAll("#pending-messages [data-client-id]")) {
            if (turns.querySelector('[data-client-id="' + CSS.escape(message.dataset.clientId) + '"]')) message.remove();
        }
        root.querySelector("#empty-transcript").hidden = count !== 0;
        if (follow) window.scrollTo(0, document.documentElement.scrollHeight);
    };
    const status = root.querySelector("#update-status");
    let events;
    let pending = false;
    let dirty = false;
    let retry;
    let reconnect;
    async function refresh() {
        dirty = true;
        if (!events || pending) return;
        clearTimeout(retry);
        pending = true;
        try {
            while (dirty && events) {
                dirty = false;
                await sync();
            }
            status.hidden = true;
        } catch (error) {
            status.hidden = false;
            console.warn("Could not refresh thread", error);
            if (events) retry = setTimeout(refresh, Number(root.dataset.pollMs));
        } finally {
            pending = false;
        }
    }
    function subscribe() {
        events?.close();
        events = null;
        clearTimeout(retry);
        clearTimeout(reconnect);
        if (document.hidden) return;
        const query = new URLSearchParams({source: turns.dataset.source, id: turns.dataset.thread});
        events = new EventSource("/api/thread-events?" + query);
        events.onmessage = event => {
            const key = JSON.parse(event.data);
            const form = root.querySelector(".thread-draft");
            if (key && form) form.elements.form_key.value = key;
            refresh();
        };
        events.onerror = () => {
            status.hidden = false;
            if (events?.readyState === EventSource.CLOSED) {
                clearTimeout(reconnect);
                reconnect = setTimeout(subscribe, Number(root.dataset.pollMs));
            }
        };
    }
    document.addEventListener("visibilitychange", subscribe);
    subscribe();
}

const thread = document.querySelector("#thread-review");
if (thread) {
    navigation(thread);
    const form = thread.querySelector(".thread-draft");
    if (form) liveTranscript(thread, composer(thread, form));
}
