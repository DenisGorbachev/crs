# CRS-Web specification

CRS-Web mirrors durable Codex thread history into Fjall and accepts messages through its Dioxus UI.

## Configuration

Load a CRS-Web TOML file at startup via `--config` or `CRS_WEB_CONFIG`. Resolve relative database and Unix socket paths against the file’s directory. Each source has a stable, unique ID. Changing the list takes effect on restart.

```toml
threads_created_since = "2026-10-01T00:00:00Z"
db_path = "crs-web.db"
reconcile_interval_seconds = 60

[codex_app_servers.local]
transport = "unix-socket"
socket_path = "/path/to/app-server-control.sock"

[codex_app_servers.remote]
transport = "websocket"
url = "wss://codex.example.com/rpc"
auth_token_env = "CODEX_REMOTE_TOKEN"
```

Omit `threads_created_since` to import the full history across all working directories. When set, import threads with `createdAt >= threads_created_since` from every source. Use midnight UTC for a date-only cutoff. The reconciliation interval defaults to 60 seconds.

`min_thread_created_at` makes the comparison explicit. `thread_history_start_at` can also refer to the first imported turn within a thread. `threads_created_since` identifies which timestamp controls thread eligibility.

A dedicated file keeps sync policy with source definitions. An environment variable suits a single-value deployment override. Dioxus or Axum configuration suits framework settings. Axios configuration suits browser HTTP transport.

## Synchronization

- Create one independent worker per configured source. Use the pinned `codex-app-server-client` and `codex-app-server-protocol` crates used by CRS CLI. Map both transport variants to `RemoteAppServerEndpoint` and enable the experimental protocol.
- Drain notifications during backfill. Coalesce `thread/status/changed` and lag signals into reconciliation requests. Reconcile after reconnects and periodically to capture changes without a status transition.
- Page `thread/list` for both archive states, all providers, every supported source filter, all sections, all projects. Follow attachments from discoverable parents, including parents that precede the cutoff. Ordinary Codex listings can omit internal sources and empty threads.
- Download history for eligible persisted threads. Store thread metadata, returned read state, all turn pages, and all item output. Fetch legacy items inside full turn pages. Fetch paginated items through `thread/items/list`.
- Key snapshots by source ID and thread ID. Commit metadata and history together in Fjall. Keep the previous snapshot if a fetch fails. Retry on the next reconciliation. Continue syncing other threads and sources.
- Exclude records outside the configured sources or cutoff from all views. Retain existing records on disk when the source stops returning them or the configuration changes.

## Server state

`Server` holds config and the Fjall `Database` and derives `Deref` and `DerefMut` to the database. Attach `SharedServer = Arc<tokio::sync::Mutex<Server>>` to the Dioxus router as an Axum `Extension`, as in AnyFood. Server functions extract it. Use cloned Fjall handles for disk I/O and keep network requests outside the lock.

Axum `State` suits typed custom routes. Dioxus context providers suit component state. A process-global `Lazy` suits resources initialized independently of server startup. [Dioxus router and state documentation](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/).

## Run

Run from the repository root to connect to a local Codex daemon.

```sh
cargo run -p crs-web -- --config fixtures/crs-web/local.toml
```

Omitting `socket_path` selects the control socket under `CODEX_HOME`, which defaults to `~/.codex`. The default web address is `127.0.0.1:8080`.

`dx build --server --fullstack=false -p crs-web` compiles the package’s `tailwind.css` to `assets/tailwind.css`. Plain `cargo run` embeds the checked-in output. [Dioxus Tailwind integration](https://dioxuslabs.com/learn/0.7/essentials/ui/styling/).

## Pages

Use Dioxus Components for lists, messages, navigation, disclosures and the composer. Bundle registry styles with Tailwind. Render disclosures as native HTML so they also work in streamed turns and with JavaScript disabled. [Component registry revision](https://github.com/DioxusLabs/dioxus-components/tree/9a758255ea26e2b20c8cecf4c1c946feb9e71da7).

Dioxus `Router<Route>` renders pages and typed links generate URLs. [Dioxus routing guide](https://dioxuslabs.com/learn/0.7/essentials/router/).

- `/` lists all configured Codex servers, including those with no synchronized threads.
- `/servers/<source>` lists top-level threads by most recent update. Exclude attached children and subagents, including subagents without a recorded parent ID. A standalone fork remains eligible. Show the app-server working directory below each thread name. Each link targets `#turn-<id>` for the chronologically last turn, including an ongoing, failed or interrupted turn. A thread with no turns opens without a fragment.
- `/servers/<source>/threads/new` opens from the server’s “New thread” link. Submitting the first message redirects to the new thread’s first turn.
- `/servers/<source>/threads/<id>` displays synchronized turns and items in order, omitting reasoning items. Each turn has a stable HTML ID.
- `/servers/<source>/files?thread=<id>&path=<absolute-path>` opens a file linked in that thread as plain text. Read its current bytes with the source’s `fs/readFile`. Resolve relative Markdown links against the thread’s working directory. Line references open the whole file.

Place the controls below the message textarea with the submit button at the right edge. “Open in Codex” comes first in the thread toolbar. Use an external-link icon and a native link to `codex://threads/<thread-id>` so it works with JavaScript disabled. The link contains no CRS source identifier.

Right-align user messages in dark bubbles with white text. Left-align assistant messages. Give both accessible speaker names. Collapse every other visible item to one expandable line. A command shows its command line. A web search shows its query. Expansion reveals the full item and metadata. Use compact icon buttons with accessible names for navigation, quoting and submission.

Visible thread tabs subscribe to Dioxus Server-Sent Events for their source and thread. On connection, reconnection and committed changes, fetch changed turns and thread metadata. Refresh open threads independently of backfill every `thread_refresh_interval_seconds` (default 3). During ongoing turns, display items and status changes as Codex makes them available in thread history. Update the transcript in place, preserve the draft and expanded commands, and follow new output when the reader is at the bottom.

Unknown sources, missing threads and threads outside the cutoff return HTTP 404. `/api/threads` returns summaries across all configured servers.

## Messages

Creation and follow-up forms support native HTML POST. Enable follow-ups when direct input is available. Validate the form key before submitting to Codex. Generate it once per Fjall database and reuse it after restarts. To supply a dedicated random UUID instead, set `form_key_env` to the name of its environment variable. Preserve nonblank message text and reject whitespace-only input.

Voice recording requires HTTPS or localhost and a server-side `OPENAI_API_KEY` (or `--openai-api-key`). Configure transcription under `[transcribe_params]`. With `stream = true`, insert text from the completed transcription event. [Voice transcription specification](../../.agents/transcription.md).

Start a new thread and its first turn with `thread/start` and `turn/start`. For a follow-up, resume the existing thread and call `thread/queue/add` with a fresh client message ID. Let Codex dispatch eligible idle threads and drain busy threads after their current turn.

Read all `thread/queue/list` pages for visible threads. Refresh after local mutations and received queue notifications. Periodic reads also capture changes from other clients. Below each queued message, offer **Steer**, **Edit**, **Cancel**. Edit text parts in place with `thread/queue/update`, retaining attachments and annotated input. Cancel with `thread/queue/delete`. Preserve structured input and metadata when steering.

To steer, check the active turn ID, persist the original submission in Fjall, delete the selected queue entry, then call `turn/steer` with that turn ID and the original client message ID. Steer only after confirmed deletion. A definitive pre-admission rejection returns the input to the tail of the queue. After uncertain delivery or interruption before the message reaches history, retain the original input for download, including while Codex is offline. Never automatically retry an uncertain steer. Steering previews have no edit or cancel actions. Retire the receipt when a replacement queue entry has the same client ID or its client ID is saved in history. The pinned Codex API cannot condition deletion on the revision last read, so edits made concurrently by another client can race promotion.

## Quoting thread messages

Keep one editable draft at the bottom of the thread. After appending a quotation, focus the textarea with the caret at the end. Save by source and thread in the browser until Codex acknowledges Send. Preserve conflicting edits in their respective tabs. The composer controls jump to either end or to the previous or next user or assistant response. Read-only threads retain navigation.

**Quote** starts on each time a writable thread page opens. Tap the button to toggle quoting. It stays on across selections until toggled off. File links open in either mode. Other links follow their destinations while quoting is off. Pointer hover leaves quotable text and the quote button visually unchanged.

Quote prose by sentence across inline formatting, within each paragraph, list item, heading or table cell. Prefix every line with `> `. Tapping inline code quotes that code span. In Rust, Python and JavaScript blocks, Tree-sitter selects identifiers or the construct introduced by a function, conditional or match keyword. Use line selection for unsupported languages or unresolved syntax. Preserve code inside a blockquoted fence longer than every backtick run in the selection. [Commenting design](https://chatgpt.com/share/6afd89c9-69a0-83ec-8c61-59a378ec9d61).
