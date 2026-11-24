# Voice transcription

- **Record** starts microphone capture in a writable message form.
- **Cancel** discards the recording without uploading it.
- **Stop** ends capture and uploads the recording through CRS-Web to OpenAI for transcription.
- Stop capture automatically before the finalized audio file would exceed 25 MB. Offer **Transcribe** or **Discard**.
- Insert the transcript once at the current caret in the originating draft, replacing any selected text.
- Retain the recording in browser memory until insertion succeeds or the user discards it.
- Disable **Record** while a recording is pending.
- On failure, offer **Retry**. Reuse the transcript if available, otherwise reprocess the retained audio.

## CRS-Web configuration

- Use `openai_dive` with model `gpt-transcribe`.
- Annotate `Command.openai_api_key` with `#[arg(long, env = "OPENAI_API_KEY")]`.
- Define `Config.transcribe_params: TranscribeParams` for optional, nondeprecated request params supported by `gpt-transcribe`, including `prompt`, `languages`, `keywords`.
- Set `transcribe_params.languages = ["en"]` in `../../fixtures/crs-web/local.toml`.
