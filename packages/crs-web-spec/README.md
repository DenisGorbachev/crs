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

Dioxus `Router<Route>` renders pages and typed `Link` components generate URLs. [Dioxus routing guide](https://dioxuslabs.com/learn/0.7/essentials/router/).

- `/` lists all configured Codex servers, including those with no synchronized threads.
- `/servers/<source>` lists top-level threads by most recent update. Exclude attached children and subagents, including subagents without a recorded parent ID. A standalone fork remains eligible. Show the app-server working directory below each thread name. Each link targets `#turn-<id>` for the chronologically last turn, including an ongoing, failed or interrupted turn. A thread with no turns opens without a fragment.
- `/servers/<source>/threads/<id>` displays synchronized turns and items in order, omitting reasoning items. Each turn has a stable HTML ID.

Right-align user messages in dark bubbles with white text. Left-align assistant messages. Give both accessible speaker names. Collapse every other visible item to one expandable line. A command shows its command line. A web search shows its query. Expansion reveals the full item and metadata. Use compact icon buttons with accessible names for navigation, quoting and submission.

Visible thread tabs subscribe to Dioxus Server-Sent Events for their source and thread. On connection, reconnection and committed changes, fetch changed turns and thread metadata. Refresh open threads independently of backfill every `thread_refresh_interval_seconds` (default 3). During ongoing turns, display items and status changes as Codex makes them available in thread history. Update the transcript in place, preserve the draft and expanded commands, and follow new output when the reader is at the bottom.

Unknown sources, missing threads and threads outside the cutoff return HTTP 404. `/api/threads` returns summaries across all configured servers.

## Messages

The server page accepts a first message for a new thread. The thread page accepts a follow-up when direct input is available. Both forms work with native HTML POST. Validate the form key before submitting to Codex. Generate it once per Fjall database and reuse it after restarts. To supply a dedicated random UUID instead, set `form_key_env` to the name of its environment variable. Preserve nonblank message text and reject whitespace-only input.

Use `thread/start` for a new thread or `thread/resume` with `excludeTurns` for an existing thread on its configured source. Submit one `turn/start` with a fresh client message ID and the server’s existing settings. Codex starts an idle turn or steers the active turn. Open a newly created thread directly. In an open thread, acknowledge the submission in place and reconcile its bubble by client message ID. Never retry input automatically.

## Quoting thread messages

Keep one editable draft at the bottom of the thread. After appending a quotation, focus the textarea with the caret at the end. Save by source and thread in the browser until Codex acknowledges Send. Preserve conflicting edits in their respective tabs. The composer controls jump to either end or to the previous or next user or assistant response. Read-only threads retain navigation.

**Quote** starts on each time a writable thread page opens. Tap the button to toggle quoting. It stays on across selections until toggled off. Links follow their destinations while quoting is off. Pointer hover leaves quotable text and the quote button visually unchanged.

Quote prose by sentence across inline formatting, within each paragraph, list item, heading or table cell. Prefix every line with `> `. Tapping inline code quotes that code span. In Rust, Python and JavaScript blocks, Tree-sitter selects identifiers or the construct introduced by a function, conditional or match keyword. Use line selection for unsupported languages or unresolved syntax. Preserve code inside a blockquoted fence longer than every backtick run in the selection. [Commenting design](https://chatgpt.com/share/6afd89c9-69a0-83ec-8c61-59a378ec9d61).
