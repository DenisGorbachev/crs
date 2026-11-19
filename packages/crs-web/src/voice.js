function wav(chunks, size, rate) {
    const header = new DataView(new ArrayBuffer(44));
    const text = (offset, value) => {
        for (let i = 0; i < value.length; i++) header.setUint8(offset + i, value.charCodeAt(i));
    };
    text(0, "RIFF");
    header.setUint32(4, size + 36, true);
    text(8, "WAVEfmt ");
    header.setUint32(16, 16, true);
    header.setUint16(20, 1, true);
    header.setUint16(22, 1, true);
    header.setUint32(24, rate, true);
    header.setUint32(28, rate * 2, true);
    header.setUint16(32, 2, true);
    header.setUint16(34, 16, true);
    text(36, "data");
    header.setUint32(40, size, true);
    return new Blob([header, ...chunks], {type: "audio/wav"});
}

export function voiceInput(controls) {
    const form = controls.closest("form");
    const editor = form.elements.message;
    const send = form.querySelector('[type="submit"]');
    const status = form.querySelector("[data-voice-status]");
    const buttons = Object.fromEntries([...controls.querySelectorAll("[data-voice-action]")].map(b => [b.dataset.voiceAction, b]));
    const maxSamples = Math.floor((Number(controls.dataset.voice) - 44) / 2);
    let active;

    function render() {
        const state = active?.state;
        buttons.record.hidden = editor.disabled;
        buttons.record.disabled = !!active || editor.readOnly || !controls.hasAttribute("data-configured");
        buttons.cancel.hidden = !["starting", "recording", "stopping"].includes(state);
        buttons.stop.hidden = state !== "recording";
        buttons.transcribe.hidden = state !== "ready";
        buttons.retry.hidden = state !== "failed";
        buttons.discard.hidden = !["ready", "failed", "uploading"].includes(state);
        send.inert = !!active;
    }

    function state(session, next, message) {
        session.state = next;
        status.textContent = message;
        render();
    }

    function release(session) {
        session.stream?.getTracks().forEach(track => track.stop());
        session.node?.disconnect();
        session.node?.port.close();
        if (session.context) {
            session.context.onstatechange = null;
            if (session.context.state !== "closed") void session.context.close().catch(() => {});
        }
        session.stream = session.node = session.context = null;
    }

    function clear() {
        if (active) {
            active.request?.abort();
            release(active);
        }
        active = null;
        window.removeEventListener("beforeunload", guard);
        status.textContent = "";
        render();
    }

    async function insert(session) {
        if (!session?.audio || session !== active || session.state === "uploading") return;
        try {
            if (!session.transcript) {
                state(session, "uploading", "Transcribing…");
                session.request = new AbortController();
                const response = await fetch("/api/transcribe", {
                    method: "POST", headers: {"Content-Type": "audio/wav", "X-CRS-Form-Key": form.elements.form_key.value},
                    body: session.audio, signal: session.request.signal,
                });
                if (!response.ok) throw new Error((await response.text()) || "Transcription failed.");
                const {text} = await response.json();
                if (typeof text !== "string" || !text.trim()) throw new Error("No speech was transcribed.");
                session.transcript = text;
            }
            if (session !== active) return;
            if (!form.isConnected || editor.disabled || editor.readOnly) throw new Error("The draft is unavailable. Retry when it is writable.");
            editor.setRangeText(session.transcript, editor.selectionStart, editor.selectionEnd, "end");
            clear();
            editor.dispatchEvent(new Event("input", {bubbles: true}));
            editor.focus();
        } catch (error) {
            if (session === active) state(session, "failed", error.message);
        }
    }

    function finish(session, limit, failure) {
        if (session !== active || !session.chunks) return;
        if (session.size) session.audio = wav(session.chunks, session.size, session.context.sampleRate);
        session.chunks = null;
        release(session);
        if (failure || !session.audio) {
            state(session, "failed", failure || "No audio was recorded. Retry recording.");
        } else if (limit) {
            state(session, "ready", "Recording reached the upload limit.");
        } else void insert(session);
    }

    async function record() {
        if (active || buttons.record.disabled || editor.disabled || editor.readOnly) return;
        const session = {chunks: [], size: 0};
        active = session;
        window.addEventListener("beforeunload", guard);
        state(session, "starting", "Opening microphone…");
        try {
            if (!window.isSecureContext) throw new Error("Microphone access requires HTTPS or localhost.");
            if (!navigator.mediaDevices?.getUserMedia || !window.AudioWorkletNode) throw new Error("Microphone recording is unavailable in this browser.");
            const context = new AudioContext();
            session.context = context;
            const microphone = navigator.mediaDevices.getUserMedia({audio: {channelCount: 1}}).then(stream => {
                if (session !== active || session.state !== "starting") stream.getTracks().forEach(track => track.stop());
                else session.stream = stream;
            });
            await Promise.all([microphone, context.resume(), context.audioWorklet.addModule("/voice-pcm.js")]);
            if (session !== active || session.state !== "starting") return;
            if (!session.stream.active) throw new Error("Microphone disconnected.");
            const node = new AudioWorkletNode(context, "voice-pcm", {
                channelCount: 1, channelCountMode: "explicit", processorOptions: {maxSamples},
            });
            session.node = node;
            node.port.onmessage = ({data}) => {
                if (session !== active || !session.chunks) return;
                if (data instanceof ArrayBuffer) {
                    session.chunks.push(data);
                    session.size += data.byteLength;
                } else finish(session, data.limit, session.interruption);
            };
            node.onprocessorerror = () => finish(session, false, "Audio capture was interrupted.");
            for (const track of session.stream.getAudioTracks()) {
                track.onended = () => stop(session, "Microphone disconnected.");
            }
            context.createMediaStreamSource(session.stream).connect(node);
            node.connect(context.destination);
            state(session, "recording", "Recording…");
            context.onstatechange = () => {
                if (context.state !== "running") finish(session, false, "Audio capture was interrupted.");
            };
            context.onstatechange();
        } catch (error) {
            if (session === active) {
                finish(session, false, error.message);
            }
        }
    }

    function stop(session, interruption) {
        if (session?.state !== "recording") return;
        session.interruption = interruption;
        state(session, "stopping", "Finishing recording…");
        session.node.port.postMessage("stop");
    }

    buttons.record.onclick = record;
    buttons.cancel.onclick = buttons.discard.onclick = clear;
    buttons.stop.onclick = () => stop(active);
    buttons.transcribe.onclick = () => insert(active);
    buttons.retry.onclick = () => {
        if (active?.audio) return insert(active);
        clear();
        return record();
    };
    const guard = event => {
        event.preventDefault();
        event.returnValue = true;
    };
    form.addEventListener("submit", event => {
        if (active) {
            event.preventDefault();
            event.stopImmediatePropagation();
        }
    }, true);
    window.addEventListener("pagehide", () => {
        if (active?.state === "starting") clear();
        else if (active?.state === "recording") stop(active, "Recording paused when you left the page.");
    });
    new MutationObserver(() => {
        if (editor.disabled || editor.readOnly) {
            if (active?.state === "starting") clear();
            if (active?.state === "recording") stop(active, "The draft became read-only.");
        }
        render();
    }).observe(editor, {attributes: true, attributeFilter: ["disabled", "readonly"]});
    render();
}
