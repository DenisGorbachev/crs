# CRS architecture

## Clients and backend

The Dioxus web UI and the native CRS CLI use the same backend. Clients send commands over HTTP and receive changes over Server-Sent Events (SSE). One CRS server process serves the web UI, coordinates agent sessions, and synchronizes drafts between devices. Each operation has one backend implementation.

Fjall stores shared drafts and the conversation data CRS needs to serve clients. The agent is authoritative for its transcript and turn status. CRS is authoritative for the draft, its current editor, and user input CRS has acknowledged but whose delivery is still pending.

## Sources and agent sessions

Each named source in the application configuration identifies an adapter and where it runs. Session and draft identifiers in CRS include the source because two hosts can have the same agent session ID. One worker per source uses the official Rust `agent-client-protocol` SDK and negotiates ACP v1. Every UI viewing the same session shares that worker's connection.

| Adapter location | How the worker connects |
| --- | --- |
| On the CRS host | Launch the adapter directly and connect over stdio. |
| On a remote host | Run the adapter with `ssh -T` and use its stdio. |
| In a local Docker sandbox | Run the adapter with `docker exec -i` and use its stdio. |

The supported agents are Codex, Claude Code, and Google Antigravity. On initialization the worker requires both `agentCapabilities.sessionCapabilities.list` and `agentCapabilities.loadSession == true`. If either is missing, report a hard error for that source identifying the adapter and missing capability. Use `session/list` for discovery and `session/load` when CRS needs the agent to replay saved history. [ACP initialization](https://agentclientprotocol.com/protocol/v1/initialization) · [Session listing](https://agentclientprotocol.com/protocol/v1/session-list) · [Loading history](https://agentclientprotocol.com/protocol/v1/session-setup)

When a browser reconnects, it recovers the current CRS view and subscribes to further changes without a gap. If the worker stayed connected, the agent has nothing to reload. If the worker itself reconnects, it reconciles saved history and current activity with the agent before presenting either as current or deciding whether a pending submission was delivered.

### Codex turn lifetime

The Codex adapter and the stock Codex CLI must attach to the same independently running Codex app-server on the agent host. A CRS restart disconnects its adapter. The daemon continues the active turn. The replacement worker reattaches to the daemon, recovers missed history, and follows any turn still running. A turn that needs client-provided input or a tool may still depend on a connected client. Sharing a Codex home gives separate processes access to saved history, but live events and control belong to the daemon running the turn.

This deployment requires live-peer support in the adapter. Released `codex-acp` v2.8.0 instead starts a private child and shuts it down when ACP stdin closes. Upstream PR #348 implements attachment to an existing daemon and forwarding of turns initiated by another client. How CRS obtains that adapter support is an open decision. [Released adapter lifecycle](https://github.com/agentclientprotocol/codex-acp/blob/v2.8.0/src/index.ts#L64-L86) · [Shared app-server proposal](https://github.com/agentclientprotocol/codex-acp/pull/348)

## Draft handoff

A session has one shared draft and one current editor. Each editing client has its own identity. Acquiring the draft atomically assigns the editor and returns the latest saved text and a new revision. After committing the change, CRS notifies the previous editor over SSE. That client makes the textarea read-only. Clicking or tapping it acquires the draft again. A reconnecting client checks ownership before enabling edits.

While typing, the client keeps unacknowledged edits in device storage, saves to CRS after a short debounce, and continues saving during sustained typing. CRS accepts a save only from the current editor against the expected revision. A late save from a previous editor cannot overwrite the new editor's work.

The sync icon turns yellow immediately on an edit and stays yellow while the latest text is unsaved, in flight, or awaiting retry. It turns green when CRS acknowledges durable storage of that exact edit. An acknowledgement for an older edit cannot turn it green. Include an accessible text equivalent for each state.

On handoff the old device can discard its acknowledged copy. If takeover races the debounce or the device is offline, retain its unacknowledged text for recovery without replacing the new editor's draft.

## Queued and steering messages

Sending captures the exact draft the user submitted. An idle session can start it immediately. In a busy session, a queued message remains editable and cancellable until dispatch. Steering targets the current turn and is a one-way action. The UI reconciles pending items with the agent transcript when delivery becomes observable.

CRS must preserve user input across client disconnects, server restarts, and lost agent replies. A client may relinquish its only copy after CRS durably acknowledges the exact submitted content. A CRS acknowledgement and confirmed delivery to the agent are separate facts. The persistence and retry strategy for queued and steering messages is still open.

## File links

Keep the source-scoped file URL and page. Remote file reads are stubbed for this refactor.

## Open decisions

- Choose the HTTP handler framework for the shared API.
- Choose how to ship Codex live-peer support.
- Choose the submission recovery policy. One candidate gives each send a stable client-generated ID and atomically moves the submitted draft revision into a durable outgoing record. Retrying the HTTP request returns the same record. The worker can retry undispatched records and reconcile in-flight attempts using an adapter-provided identifier. If delivery remains uncertain, the user can inspect the retained text and decide whether to resend. A steering attempt also records its intended turn.
