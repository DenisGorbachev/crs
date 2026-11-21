# CRS architecture

## Clients and backend

The Dioxus web UI and the native CRS CLI access the same agent sessions through an ACP endpoint. The CLI connects directly to the endpoint for agent operations. The web application also provides a draft service for editing across devices. Its clients send draft commands over HTTP and receive ownership and sync updates over Server-Sent Events (SSE). The CLI has no dependency on that service or on Dioxus.

The agent is authoritative for its transcript and turn status. The web backend uses Fjall for shared drafts, pending web submissions, and the conversation data it caches. A CLI that submits directly retains its input until the agent confirms delivery or the user resolves an uncertain outcome.

## Sources and agent sessions

Each named source in the application configuration identifies an adapter and where it runs. Session and draft identifiers in CRS include the source because two hosts can have the same agent session ID. One persistent worker per source owns the adapter's stdio connection, uses the official Rust `agent-client-protocol` SDK, and negotiates ACP v1. The worker's lifetime is independent of the Dioxus process and individual CLI invocations. Disconnecting the last UI client during a turn leaves that turn running. An interaction that needs a client can still require someone to reconnect and respond.

| Adapter location | How the worker connects |
| --- | --- |
| On the CRS host | Launch the adapter directly and connect over stdio. |
| On a remote host | Run the adapter with `ssh -T` and use its stdio. |
| In a local Docker sandbox | Run the adapter with `docker exec -i` and use its stdio. |

The client-facing ACP endpoint and the worker-to-adapter stdio stream are separate connections. A new client attaches to the existing worker for that source. The endpoint routes requests and responses to the correct client, distributes session updates to attached clients, and routes each permission decision once. A reconnecting client can recover history and current activity before following further updates.

`agent-client-protocol-http` provides a Rust `HttpClient` and an `AcpHttpServer` router that supports HTTP/SSE and WebSocket. The router invokes its agent factory for every new ACP connection. If used here, each factory result must attach to the persistent worker. Spawning a stdio adapter in that factory would give the CLI and web application separate agent processes. [SDK server contract](https://github.com/agentclientprotocol/rust-sdk/blob/0fe20235100691d2aa1bc38f729e8ab156338b4f/src/agent-client-protocol-http/src/server.rs#L219-L234)

The supported agents are Codex, Claude Code, and Google Antigravity. On initialization the worker requires both `agentCapabilities.sessionCapabilities.list` and `agentCapabilities.loadSession == true`. If either is missing, report a hard error for that source identifying the adapter and missing capability. Use `session/list` for discovery and `session/load` when CRS needs the agent to replay saved history. [ACP initialization](https://agentclientprotocol.com/protocol/v1/initialization) · [Session listing](https://agentclientprotocol.com/protocol/v1/session-list) · [Loading history](https://agentclientprotocol.com/protocol/v1/session-setup)

For Claude Code the worker runs the Claude ACP adapter, which uses the Claude Agent SDK and supplies the path to the Claude executable. For Antigravity the worker runs `agy acp`, which speaks ACP directly over stdio. A Dioxus restart leaves these processes running. Recovery after the worker or agent process itself crashes depends on the agent. [Claude adapter](https://github.com/agentclientprotocol/claude-agent-acp/blob/v0.109.0/src/v1/acp-agent.ts#L10269) · [Antigravity ACP entry point](https://antigravity.google/docs/ide/extensions/acp#overview)

When a browser reconnects, it recovers the current view and subscribes to further changes without a gap. If the worker still has the session and its updates, it can supply that view directly. If the worker loses its adapter connection, it reconciles saved history and current activity with the agent before presenting either as current or deciding whether a pending submission was delivered.

### Codex turn lifetime

Keeping the worker alive across a Dioxus restart also keeps the Codex adapter and its child app-server alive. To let the stock Codex CLI participate in the same ongoing turn, the adapter and the stock CLI must both attach to the same independently running Codex app-server on the agent host. If the worker itself restarts in that deployment, its replacement reattaches to the daemon, recovers missed history, and follows any turn still running.

This deployment requires live-peer support in the adapter. Released `codex-acp` v2.8.0 instead starts a private child and shuts it down when ACP stdin closes. Upstream PR #348 implements attachment to an existing daemon and forwarding of turns initiated by another client. How CRS obtains that adapter support is an open decision. [Released adapter lifecycle](https://github.com/agentclientprotocol/codex-acp/blob/v2.8.0/src/index.ts#L64-L86) · [Shared app-server proposal](https://github.com/agentclientprotocol/codex-acp/pull/348)

## Draft handoff

A session has one shared draft and one current editor. Each editing client has its own identity. Acquiring the draft atomically assigns the editor and returns the latest saved text and a new revision. After committing the change, CRS notifies the previous editor over SSE. That client makes the textarea read-only. Clicking or tapping it acquires the draft again. A reconnecting client checks ownership before enabling edits.

While typing, the client keeps unacknowledged edits in device storage, saves to CRS after a short debounce, and continues saving during sustained typing. CRS accepts a save only from the current editor against the expected revision. A late save from a previous editor cannot overwrite the new editor's work.

The sync icon turns yellow immediately on an edit and stays yellow while the latest text is unsaved, in flight, or awaiting retry. It turns green when CRS acknowledges durable storage of that exact edit. An acknowledgement for an older edit cannot turn it green. Include an accessible text equivalent for each state.

On handoff the old device can discard its acknowledged copy. If takeover races the debounce or the device is offline, retain its unacknowledged text for recovery without replacing the new editor's draft.

## Queued and steering messages

Sending captures the exact draft the user submitted. An idle session can start it immediately. In a busy session, a queued message remains editable and cancellable until dispatch. Steering targets the current turn and is a one-way action. The UI reconciles pending items with the agent transcript when delivery becomes observable.

Clients must preserve user input across disconnects, server restarts, and lost agent replies. A web client may relinquish its only copy after CRS durably acknowledges the exact submitted content. A CRS acknowledgement and confirmed delivery to the agent are separate facts. If a queued or steering message may have reached the agent, reconcile it against authoritative delivery evidence. If the outcome remains ambiguous, show **Delivery unknown**, retain the exact text and intended destination, and let the user decide whether to resend. Never automatically resend an ambiguous submission or redirect a steer to a subsequent turn.

## File links

Keep the source-scoped file URL and page. Remote file reads are stubbed for this refactor.

## Open decisions

- Choose how to host the persistent workers and expose shared ACP sessions to independently connecting clients.
- Choose how to implement the web draft service's HTTP handlers.
- Choose how to ship Codex live-peer support.
- Choose how to persist and identify submission attempts. One candidate gives each web send a stable client-generated ID and atomically moves the submitted draft revision into a durable outgoing record. Retrying the web request returns the same record. A direct CLI submission also needs recoverable text and a way to match agent acknowledgements, independent of web draft synchronization.
