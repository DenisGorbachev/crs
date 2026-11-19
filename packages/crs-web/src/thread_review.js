import { threadQueue } from "/thread-queue.js";
import { voiceInput } from "/voice.js";

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

function arrowNavigation(container, selector) {
    let current;
    function sync(active = current) {
        const items = [...container.querySelectorAll(selector)].filter(item => item.getClientRects().length);
        current = items.includes(active) ? active : items[0];
        items.forEach(item => { item.tabIndex = item === current ? 0 : -1; });
        return items;
    }
    sync();
    container.addEventListener("focusin", event => sync(event.target));
    container.addEventListener("keydown", event => {
        if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
        const items = sync(event.target);
        const index = items.indexOf(event.target);
        if (index < 0) return;
        if (event.key === " " && event.target.matches('a[role="menuitem"]')) {
            event.preventDefault();
            event.target.click();
            return;
        }
        const target = { ArrowLeft: Math.max(0, index - 1), ArrowRight: Math.min(items.length - 1, index + 1), Home: 0, End: items.length - 1 }[event.key];
        if (target === undefined) return;
        event.preventDefault();
        items[target].focus();
    });
    return sync;
}

function navigation(root) {
    const controls = root.querySelector('.thread-draft [role="toolbar"]');
    let selected;
    let lastScroll;
    function select(message) {
        selected?.removeAttribute("aria-current");
        selected = message;
        selected?.setAttribute("aria-current", "true");
    }
    controls.querySelectorAll("button[hidden]").forEach(button => { button.hidden = false; });
    const sync = arrowNavigation(controls, "a[href], button:not(:disabled)");
    controls.addEventListener("click", event => {
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
    return sync;
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

function prepareQuotes(scope, enabled) {
    if (enabled) {
        for (const body of scope.querySelectorAll(".markdown")) {
            if (prepared.has(body)) continue;
            prepareProse(body);
            for (const code of body.querySelectorAll("pre code[data-ranges]")) prepareCode(code);
            prepared.add(body);
        }
    }
    for (const part of scope.querySelectorAll("[data-quote-start]")) {
        if (!enabled) {
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

function composer(root, form, syncControls, queue) {
    const editor = form.elements.message;
    const quoteButton = form.querySelector("#quote-mode");
    const status = form.querySelector('[role="status"]');
    const send = form.querySelector('[type="submit"]');
    const draft = threadDraft(form, editor, status);
    editor.addEventListener("focus", () => form.classList.add("editing"));
    form.addEventListener("focusout", event => {
        if (!form.contains(event.relatedTarget)) form.classList.remove("editing");
    });
    function quoting() {
        return !editor.disabled && quoteButton.ariaPressed === "true";
    }
    function syncQuotes(scope) {
        prepareQuotes(scope, quoting());
    }
    syncQuotes(root);
    function setWritable(writable) {
        if (writable === editor.disabled) {
            editor.hidden = editor.disabled = !writable;
            send.hidden = !writable;
            send.disabled = !writable || editor.readOnly;
            root.querySelector("#direct-input-notice").hidden = writable;
            syncControls();
            syncQuotes(root);
        }
    }
    quoteButton.addEventListener("click", () => {
        const enabled = !quoting();
        quoteButton.ariaPressed = String(enabled);
        quoteButton.dataset.state = enabled ? "on" : "off";
        syncQuotes(root);
    });
    function insert(event) {
        if (!quoting() || editor.readOnly) return;
        const link = event.target.closest(".markdown a");
        if (link?.getAttribute("href")?.startsWith(root.querySelector("#turns").dataset.fileRoute + "/")) return;
        const part = event.target.closest(".quote-chunk") ?? link?.querySelector(".quote-chunk");
        if (!part && !link) return;
        event.preventDefault();
        const text = editor.value;
        const separator = !text || text.endsWith("\n\n") ? "" : text.endsWith("\n") ? "\n" : "\n\n";
        editor.value = text + separator + formatQuote(part ? quotations.get(part) : link.href) + "\n\n";
        draft.save();
        editor.focus({ preventScroll: true });
        editor.setSelectionRange(editor.value.length, editor.value.length);
        editor.scrollTop = editor.scrollHeight;
    }
    root.addEventListener("click", insert);
    root.addEventListener("keydown", event => {
        if (event.key === "Enter" || event.key === " ") insert(event);
    });
    form.addEventListener("submit", async event => {
        event.preventDefault();
        if (send.disabled) return;
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
            const follow = atTranscriptBottom();
            queue.accepted(await response.json());
            draft.acknowledge(submitted);
            editor.value = "";
            status.textContent = "";
            if (follow) window.scrollTo(0, document.documentElement.scrollHeight);
        } catch {
            status.textContent = "Could not confirm delivery. Check the thread before sending again.";
        } finally {
            send.disabled = editor.disabled;
            editor.readOnly = false;
        }
    });
    root.classList.add("has-draft");
    new ResizeObserver(() => root.style.setProperty("--draft-height", form.offsetHeight + "px")).observe(form);
    return { setWritable, syncQuotes };
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
            else {
                const focused = old.contains(document.activeElement) ? [...old.querySelectorAll("summary")].indexOf(document.activeElement) : -1;
                old.replaceWith(node);
                if (focused >= 0) node.querySelectorAll("summary")[focused]?.focus({ preventScroll: true });
            }
        }
        const at = current.children[index];
        if (node !== at) current.insertBefore(node, at ?? null);
    }
    while (current.children.length > children.length) current.lastElementChild.remove();
    current.dataset.version = next.dataset.version;
    current.dataset.inProgress = next.dataset.inProgress;
}

function liveTranscript(root, form) {
    const queue = threadQueue(root, refresh);
    const {setWritable, syncQuotes} = composer(root, form, navigation(root), queue);
    const turns = root.querySelector("#turns");
    if (!turns) return;
    const metadata = root.querySelector("#thread-metadata");
    const sync = async () => {
        const response = await fetch("/api/thread-updates", {
            signal: AbortSignal.timeout(30000),
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({source: turns.dataset.source, id: turns.dataset.thread, header: metadata.dataset.version, queue_version: queue.version, versions: Array.from(turns.children, el => el.dataset.version)}),
        });
        if (response.status === 204) return;
        if (!response.ok) throw new Error("Thread updates returned HTTP " + response.status);
        const [count, changed, header, queued] = await response.json();
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
            syncQuotes(turns.children[index]);
        }
        while (turns.childElementCount > count) turns.lastElementChild.remove();
        queue.sync(queued);
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

const pageNav = document.querySelector(".page-nav");
if (pageNav) {
    pageNav.tabIndex = -1;
    arrowNavigation(pageNav, "a");
}

const thread = document.querySelector("#thread-review");
for (const controls of document.querySelectorAll("[data-voice]")) voiceInput(controls);
if (thread) {
    const form = thread.querySelector(".thread-draft");
    if (form) liveTranscript(thread, form);
}
