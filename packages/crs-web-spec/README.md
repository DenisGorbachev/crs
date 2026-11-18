# CRS-Web specification

CRS-Web mirrors durable Codex thread history into Fjall. The read-only Dioxus 0.7 UI renders the thread list on the server.

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

`threads_created_since` is a required RFC 3339 timestamp shared by all sources. Import history when `createdAt >= threads_created_since`. Use midnight UTC for a date-only cutoff. The reconciliation interval defaults to 60 seconds.

`min_thread_created_at` makes the comparison explicit. `thread_history_start_at` can also refer to the first imported turn within a thread. `threads_created_since` identifies which timestamp controls thread eligibility.

A dedicated file keeps sync policy with source definitions. An environment variable suits a single-value deployment override. Dioxus or Axum configuration suits framework settings. Axios configuration suits browser HTTP transport.

## Synchronization

- Create one independent worker per configured source. Use the pinned `codex-app-server-client` and `codex-app-server-protocol` crates used by CRS CLI. Map both transport variants to `RemoteAppServerEndpoint` and enable the experimental protocol.
- Drain notifications during backfill. Coalesce `thread/status/changed` and lag signals into reconciliation requests. Reconcile after reconnects and periodically to capture changes without a status transition.
- Page `thread/list` for both archive states, all providers, every supported source filter, all sections, all projects. Follow attachments from discoverable parents, including parents that precede the cutoff. Ordinary Codex listings can omit internal sources and empty threads.
- Download history for eligible persisted threads. Store thread metadata, returned read state, all turn pages, and all item output. Fetch legacy items inside full turn pages. Fetch paginated items through `thread/items/list`.
- Key snapshots by source ID and thread ID. Commit metadata and history together in Fjall. Keep the previous snapshot if a fetch fails. Retry on the next reconciliation. Continue syncing other threads and sources.
- Exclude records outside the configured sources or cutoff from the list. Retain existing records on disk when the source stops returning them or the configuration changes.

## Server state

`Server` holds config and the Fjall `Database` and derives `Deref` and `DerefMut` to the database. Attach `SharedServer = Arc<tokio::sync::Mutex<Server>>` to the Dioxus router as an Axum `Extension`, as in AnyFood. Server functions extract it. Use cloned Fjall handles for disk I/O and keep network requests outside the lock.

Axum `State` suits typed custom routes. Dioxus context providers suit component state. A process-global `Lazy` suits resources initialized independently of server startup. [Dioxus router and state documentation](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/).

## Run

```sh
cargo run -p crs-web -- --config /path/to/crs-web.toml
```

The default address is `127.0.0.1:8080`. Open `/` for the thread list or `/api/threads` for its JSON representation.
