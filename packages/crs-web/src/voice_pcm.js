class VoicePcm extends AudioWorkletProcessor {
    constructor({ processorOptions }) {
        super();
        this.remaining = processorOptions.maxSamples;
        this.block = new DataView(new ArrayBuffer(8192));
        this.used = 0;
        this.done = false;
        this.port.onmessage = () => this.finish(false);
    }

    flush() {
        if (!this.used) return;
        const bytes = this.block.buffer.slice(0, this.used);
        this.port.postMessage(bytes, [bytes]);
        this.used = 0;
    }

    finish(limit) {
        if (this.done) return;
        this.done = true;
        this.flush();
        this.port.postMessage({ limit });
    }

    process(inputs) {
        if (this.done) return false;
        const channel = inputs[0]?.[0];
        if (channel) {
            const count = Math.min(channel.length, this.remaining);
            for (let i = 0; i < count; i++) {
                const sample = Math.max(-1, Math.min(1, channel[i]));
                this.block.setInt16(this.used, sample * (sample < 0 ? 32768 : 32767), true);
                this.used += 2;
                if (this.used === this.block.byteLength) this.flush();
            }
            this.remaining -= count;
            if (!this.remaining) this.finish(true);
        }
        return !this.done;
    }
}

registerProcessor("voice-pcm", VoicePcm);
