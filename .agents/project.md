# CRS

CRS is a code review system.

## TODO

- Define `Locator`

## Decisions

- How to deal with file-level changes that are not tied to a specific code item?
  - Examples:
    - Adding `#![no_std]`
- How to deal with inter-dependent code items?

## Idents

```rust
use tokio::process::Command;
use globset::Glob;
use pulldown_cmark::{OffsetIter, Options};
use save_load::Format;
use nu_path::{RelativePath, RelativePathBuf};
```

## crs package

### file Cargo.toml

- Codex dependencies have the same version
- Must have dependencies:
  - `globset`
  - `save-load`
  - `pulldown-cmark`
  - `nu-path`

### struct Command

- Must have fields:
  - `config: PathBuf`
  - `db: PathBuf`
  - `user_id: UserId`
  - `session_id: SessionId`
    - `#[clap(env = "CRS_SESSION_ID")]`
  - `editor: Option<OsString>`
  - `subcommand: Subcommand`
    - `#[command(subcommand)]`
- Must have methods:
  - `run`
    - `let config = Format::load_one_as(&config)`
    - `let db = Db::open(db_config)`
    - `let now = Timestamp::now()`
    - `let editor = editor_var_os(editor)`
- Must expose `draft new` through `DraftCommand`

### Environment output

- Must require `env_file: PathBuf` with `#[arg(long, env = "CRS_ENV_FILE")]`
- Must append NUL-terminated `NAME=VALUE` records to the existing file with `OpenOptions::append`, after the corresponding state exists
- Names must match `[A-Za-z_][A-Za-z0-9_]*`, excluding `CRS_ENV_FILE` and the `crs_env_` prefix
- Values may contain any bytes except NUL

### struct ShowCommand

- Must have methods:
  - `run(config: &Config, db: &Db, user_id: UserId, session_id: SessionId, now: Timestamp)`
    - `let session = Session::get_or_create(id, &mut tx)`
    - Must iterate `config.sources()`
    - Must find the first review item that is not approved but whose dependencies are approved
      - Must descend into the first unapproved unseen dependency
        - Notes:
          - The "unseen" check is needed because two Rust code items can be inter-dependent
    - Must write `review_item` to `stdout`
    - Must set the `session.locator` to the locator of the `review_item`
    - `session.insert_at(session_id, &mut tx)`

### struct InsertVerdictCommand

- Must have fields:
  - `locator: Option<Locator>`
  - `is_approved: bool`
- Must have methods:
  - `run`
    - `let locator = locator.unwrap_or_else(|| get_locator_from_session_id_opt(session_id))`
    - Must insert a new `Verdict` at `locator`

### struct PrintVerdictCommand

### struct GitCommand

### struct NextApprovalGitCommand

- Must show one unapproved review item in a pager that exits at the end of the item

### struct RepoGitCommand

### struct InsertRepoGitCommand

- Must have fields:
  - `dir: PathBuf`
- Must have methods:
  - `run`

### struct MoveRepoGitCommand

- Must have fields:
  - `old: PathBuf`
  - `new: PathBuf`
- Must have methods:
  - `run`

### struct RemoveRepoGitCommand

- Must have fields:
  - `dir: PathBuf`
- Must have methods:
  - `run`

### struct CodexCommand

- Must satisfy [Codex API](#codex-api)
- Must have fields:
  - `app_server: RemoteAppServerEndpoint`
    - `#[clap(long, env = "CRS_CODEX_APP_SERVER", value_parser = codex_tui::resolve_remote_addr)]`
  - `subcommand: CodexSubcommand`
    - `#[clap(subcommand)]`
- Must have methods:
  - `run`
    - Must connect using `RemoteAppServerClient::connect` from `codex-app-server-client`
      - Must use `crs` and the CRS package version as the client identity
      - Must use `RemoteAppServerConnectArgs` with `experimental_api = true`
    - Must pass `&mut RemoteAppServerClient` to the selected subcommand
    - Must shut down the client connection after the subcommand completes, including on failure

### Codex API

- Each app-server request must use the corresponding `ClientRequest` variant from `codex-app-server-protocol`
- Must call `client.request_typed::<Response>(request).await` on the client created by `CodexCommand::run`, where `Response` is the response type declared for that `ClientRequest` variant
- Must use `client.next_event()` when consuming notifications or server-initiated requests
- Must not spawn an app-server or fall back to local storage when connection or protocol operations fail

### Codex message input

- Must have fields:
  - `prompt: Option<String>`
    - Optional positional argument
  - `edit: bool`
    - `#[arg(long)]`
  - `draft_dir: Option<PathBuf>`
    - `#[arg(long)]`
- When `prompt` is `Some("-")`, must read UTF-8 text from stdin until EOF
- When `prompt` is `Some(text)` other than `"-"`, must use `text` verbatim
- When `prompt` is `None`, must require `edit`
- With `edit`, must create a draft satisfying [Draft files](#draft-files)
  - Must initialize it with the resolved prompt text when provided
  - For an existing thread without a prompt, must use [Codex agent message selection](#codex-agent-message-selection) with `index = 0`
  - For a new thread without a prompt, must create an empty draft
- With `edit`, must satisfy [Draft editing](#draft-editing)
- Must return success without creating or resuming a thread or submitting input when the resulting text is empty after trimming whitespace
- Must preserve whitespace in nonempty input
- Must generate one UUID `client_user_message_id` per submission
- Must not automatically resubmit after an ambiguous transport failure

### Codex foreground submissions

- Must accept `output_dir: Option<PathBuf>` with `#[arg(long)]`, defaulting to `<git-root>/crs.local/threads`
- Must resolve `output_dir` on the CRS client filesystem
- Before submitting input, must exclusively create `<output_dir>/<thread_id>/<UTC timestamp>.md`
- Must consume `next_event()` until `turn/completed` for the thread and turn returned by the submission response
  - Must retain notifications received before that response
  - Must stream progress to stderr
  - Must write completed final agent messages for that turn to the output file exactly once per item
  - Must handle approval and user-input requests through the terminal
- Must print the output path to stdout
- Must preserve output on failure and include its path in the error
- Must return nonzero for a failed or interrupted turn
- Ctrl+C must disconnect the client without interrupting the server's turn

### struct CreateThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must satisfy [Environment output](#environment-output)
- Must satisfy [Codex message input](#codex-message-input)
- Must satisfy [Codex foreground submissions](#codex-foreground-submissions)
- Must have `cwd: Option<PathBuf>` with `#[arg(long)]`
- Must error if `CRS_CODEX_THREAD_ID` is set
- Must call `thread/start` with `ThreadStartParams` and decode `ThreadStartResponse`
  - Must pass `cwd` as a path on the app-server host when provided
- Must export the returned thread ID as `CRS_CODEX_THREAD_ID` through the environment output file before submitting the first turn
- Must submit the input through `turn/start` with `TurnStartParams` and decode `TurnStartResponse`

### Existing-thread submissions

- Before submitting input, must call `thread/resume` with `ThreadResumeParams` and decode `ThreadResumeResponse`
  - Must set `exclude_turns = true`
  - Must omit configuration overrides
- Must finish reading or editing input before resuming the thread

### struct SendGetThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must satisfy [Codex message input](#codex-message-input)
- Must satisfy [Codex foreground submissions](#codex-foreground-submissions)
- Must satisfy [Existing-thread submissions](#existing-thread-submissions)
- Must call `turn/start` with `TurnStartParams` and decode `TurnStartResponse`
- Must let the server start an idle thread's turn or steer its active turn

### struct SteerGetThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must satisfy [Codex message input](#codex-message-input)
- Must satisfy [Codex foreground submissions](#codex-foreground-submissions)
- Must satisfy [Existing-thread submissions](#existing-thread-submissions)
- Must have `expected_turn_id: Option<String>` with `#[arg(long)]`
- When omitted, must select the latest turn through `thread/turns/list` with `limit: Some(1)`, descending order, and summary items
  - Must error unless that turn is in progress
- Must call `turn/steer` with `TurnSteerParams`, including `expected_turn_id`, and decode `TurnSteerResponse`
- Must propagate a missing, changed, or non-steerable active turn as an error

### struct QueueGetThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must satisfy [Codex message input](#codex-message-input)
- Must call `thread/queue/add` with `ThreadQueueAddParams` and decode `ThreadQueueAddResponse`
- Must write the returned `QueuedSubmission` to stdout via `write_jsonl` and return after acknowledgement
- Must leave delivery to the server's durable queue
  - Loaded eligible idle threads may start immediately
  - Busy threads retain the input for a later turn
  - Unloaded or interrupted threads may retain it until further user action
- Must propagate an unavailable queue as an error

### struct ThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must have fields:
  - `subcommand: ThreadCodexSubcommand`
    - `#[clap(subcommand)]`
- Must have methods:
  - `run`
    - Must pass the mutable app-server client to the selected subcommand
- Subcommands: `create`, `get`, `filter`

### struct GetThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must have fields:
  - `thread_id: CodexThreadId`
    - `#[clap(env = "CRS_CODEX_THREAD_ID")]`
  - `subcommand: GetThreadCodexSubcommand`
    - `#[clap(subcommand)]`
- Must have methods:
  - `run`
    - Must pass the mutable app-server client and `thread_id` to the selected subcommand
- Subcommands: `send`, `steer`, `queue`, `render-agent-message`, `view-agent-message`

### Codex agent message selection

- Must call `thread/items/list` with `ThreadItemsListParams` and decode `ThreadItemsListResponse`
  - Must set `thread_id`, omit `turn_id`, start without a cursor, and set `sort_direction: Some(Desc)`
  - Must request `u32::MAX` items and let the server apply its page-size cap
- Must filter the returned `ThreadItemEntry.item` values by the `AgentMessage` variant
- Must get the agent message at `index` across pages, newest first
  - The index counts agent messages across pages, not all items
  - Must stop after the first page when it contains the requested agent message
  - Otherwise, must follow `next_cursor` lazily until the requested agent message is found or history is exhausted
  - Must return an error when history contains too few agent messages

### struct RenderAgentMessageGetThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must satisfy [Codex agent message selection](#codex-agent-message-selection)
- Must have `index: usize` with `#[arg(long, default_value_t)]`
- Must write the text of the agent message to `stdout`

### struct ViewAgentMessageGetThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must satisfy [Codex agent message selection](#codex-agent-message-selection)
- Must have fields:
  - `index: usize`
    - `#[arg(long, default_value_t)]`
  - `pager: OsString`
    - `#[arg(long, default_value = "glow")]`
- Must pass the selected message's text to the pager's stdin

### struct FilterThreadCodexCommand

- Must satisfy [Codex API](#codex-api)
- Must have fields:
  - `search_term: Option<String>`
    - `#[clap(long)]`
  - `cwd_filters: Vec<String>`
    - `#[clap(long, num_args = 1..)]`
    - Paths refer to the app-server host
  - `offset: usize`
    - `#[clap(long, default_value_t)]`
  - `limit: usize`
    - `#[clap(long, default_value_t = 10)]`
- Must have methods:
  - `run`
    - Must call `thread/list` with `ThreadListParams` and decode `ThreadListResponse`
      - Must sort by creation time descending
      - Must include all source categories explicitly because an omitted or empty `source_kinds` restricts results to interactive sources; `SubAgent` includes every subagent subtype
      - Must include all model providers and non-archived threads across all sections and projects
      - Must pass `search_term` and `cwd_filters` to the server
      - Must set `use_state_db_only = true`
    - Must follow `next_cursor` lazily, skip `offset` matching threads, and write at most `limit` threads to `stdout` newest first via `write_jsonl`
      - Must emit each app-server `Thread` object using its protocol JSON representation
      - A zero `limit` must emit nothing and make no `thread/list` requests
    - Must cap the requested page size at offset plus limit and `u32::MAX`, using checked arithmetic and conversions; the server may cap the page further

### struct MessageCommand

- Must have methods:
  - `run`
    - `let messages = messages_keyspace(&db)`

### struct InsertMessageCommand

- Must have methods:
  - `run`
    - `let message_id = create_message_id(now)`
    - `let message = Message::default()`
    - `save(&messages, message_id, &message)`
    - Must write `message_id` to `stdout`

### struct GetMessageCommand

- Must have fields:
  - `message_id: MessageId`
    - `#[clap(env = "CRS_MESSAGE_ID")]`

### struct ExtendGetMessageCommand

- Must have fields:
  - `items: Vec<String>`
- Must have methods:
  - `run`
    - `let message = load::<Message>::(messages, message_id)`
    - `message.extend(items)`
    - `save(messages, message_id, message)`

### struct RenderGetMessageCommand

- Must have methods:
  - `run`
    - `let message = load::<Message>::(messages, message_id)`
    - `message.write_as_markdown(&mut stdout)`

### struct GitApprovalSet

- Must have fields:
  - `value: bool`
    - `#[clap(action = clap::ArgAction::Set)]`
  - `path: PathBuf`
  - `commit: Option<GitCommitHash>`
- Must have methods:
  - `run`
    - Must unwrap `commit`, defaulting to the current commit in the repo
    - `todo!()`

### struct Config

- Must have fields:
  - `sources: Vec<Source>`

### enum Source

- Must have variants:
  - `GitRepo(GitRepoSource)`
  - `Codex(CodexSource)`

### impl SourceLike for Source

- Must delegate to the `impl SourceLike` of corresponding variant

### enum GitRepoSource

- Must have variants:
  - `Path(PathBuf)`
  - `Glob(Glob)`
- Must have methods:
  - `repos(&self) -> Result<impl Iterator<Item = Result<Repository, GitError>>, GitRepoSourceReposError>`

### impl SourceLike for GitRepoSource

- Must have methods:
  - `review_items`
    - Must iterate `repos`
      - Must iterate branches
        - Must iterate commits in reverse order (latest commit first)
          - Must get the first unapproved path within a commit

### struct CodexSource

- Must have fields:
  - `home: PathBuf`
- Must have methods:
  - `command(&self)`
    - Must have output inner type: `Command`
    - Must construct a `codex` command
      - Must set `CODEX_HOME` var to `self.home`

### impl SourceLike for CodexSource

### struct GitRepoApproval

- Must be an [archived type](#archived-type)
- Must have fields:
  - `commits: FxHashMap<GitCommitHash, GitCommitApproval>`

### struct GitCommitApproval

- Must have fields:
  - `paths: FxHashMap<PathBuf, FxHashMap<UserId, Verdict>>`

### struct Verdict

- Must have fields:
  - `is_approved: bool`
  - `timestamp: Timestamp`

### Archived type

- Must have derives:
  - `rkyv::Archive`
  - `rkyv::Serialize`
  - `rkyv::Deserialize`

### struct Session

- Must be an [archived type](#archived-type)
- Must have fields:
  - `locator: Option<Locator>`
- Must have methods:
  - `get_or_create<'t>(id: SessionId, tx: &mut Tx<'t>)`
    - Must not insert
  - `insert_at<'t>(&self, id: SessionId, tx: &mut Tx<'t>)`
    - Must call `save`

### enum Locator

- Must have variants:
  - `GitPath(GitPathLocator)`

### struct GitPathLocator

- Must have fields:
  - `repo_id: GitRepoId`
  - `commit_hash: GitOid`
  - `path: PathBuf`

### struct Message

- Must have fields:
  - `destination: Option<MessageDestination>`
  - `items: Vec<String>`
- Must have methods:
  - `write_as_markdown`
    - `self.items.iter().map()`

### enum MessageDestination

- Must have variants:
  - `Codex(CodexMessageDestination)`

### struct CodexMessageDestination

- Must have fields:
  - `thread_id: CodexThreadId`

### struct MessageItem

- Must have fields:
  - `quote: Option<String>`
  - `comment: String`
- Must have methods:
  - `write_as_markdown(writer: &mut impl Write)`
    - `if let Some(quote) = self.quote.as_ref()`
      - Must call `write_markdown_blockquote(writer, quote)`
      - Must write a newline
    - Must write `comment`

### impl TryFrom<Vec<String>> for MessageItem

- Must have functions:
  - `try_from`
    - `match parts.len()`
      - 0 => `Err`
      - 1 => `MessageItem::new(None, parts[0])`
      - 2 => `MessageItem::new(Some(parts[0]), parts[1])`
      - _ => `Err`

### struct Review

- Must have fields:
  - `items: Vec<ReviewItem>`
- Must have methods:
  - `write_as_markdown(writer: &mut impl Write, source: &str)`
    - Must iterate items
      - Must call `item.write_as_markdown`
    - Must separate the items with a horizontal line ("-----")

### struct ReviewItem

- Must have fields:
  - `interval: CharInterval`
  - `comment: String`
- Must have methods:
  - `write_as_markdown(writer: &mut impl Write, source: &str)`
    - Must call `write_markdown_blockquote(writer, source[interval])`
    - Must write a newline
    - Must write `comment`

### type MarkdownParser

- Must be a type alias of `pulldown_cmark::Parser`

### type MarkdownEvent

- Must be a type alias of `pulldown_cmark::Event`

### fn markdown_parser_new

- Must accept a Markdown document
- Must return `MarkdownParser`
- Must return `MarkdownParser::new_ext(document, Options::all())`

### fn write_markdown_blockquote

- Must have inputs:
  - `writer: &mut impl Write`
  - `quote: &impl AsRef<str>`
- Must have body:
  - Must write a Markdown blockquote
    - Must prefix each line with `>`
    - Must escape each line

### fn iter_write_as_markdown

- Must have inputs:
  - `writer: &mut impl Write`
  - `iter: impl IntoIterator<Item = &'a str>`
- Must have body:
  - Must write the items from iter separated with a horizontal line ("-----")
- Must be written as a loop.
  - Rationale: it's clearer.

### struct MarkdownLocator

- Must be an [archived type](#archived-type)
- Must have fields:
  - `range: Range<usize>`
    - /// The half-open UTF-8 byte range returned by `OffsetIter::next`
  - `occurrence: usize`
    - /// The zero-based occurrence among the non-`MarkdownEvent::End` events whose source range equals `range`
- Must have methods:
  - `locate`
    - Must have inputs:
      - `iter: &mut OffsetIter`
    - Must ignore `MarkdownEvent::End` events
    - Must select occurrence `occurrence` among the events whose source range equals `source_range`
    - Must return the selected `MarkdownEvent` and its source range

### enum PathBufPrefix

- Must have variants:
  - `Root`
  - `Home`
  - `Empty`
- Must have methods:
  - `resolve(self, home: &Path, target: &Path)`

### struct PrefixedPathBuf

- Must have at least the following derives:
  - `new`
  - `Deref`
  - `AsRef`
  - `Borrow`
- Must have fields:
  - `prefix: PathBufPrefix`
  - `value: RelativePathBuf`
    - #[deref]
    - #[as_ref]
    - #[borrow]
- Must have methods:
  - `resolve(&self, home: &Path)`

### impl TryFrom<(PathBufPrefix, PathBuf)> for PrefixedPathBuf

### struct DraftCommand

- Must dispatch its `new` subcommand to `NewDraftCommand`

### Draft files

- Must resolve the draft directory on the CRS client filesystem
- Must default the draft directory to `<git-root>/.crs.local/drafts`
- Must create a persistent, unique Markdown file whose name starts with a UTC timestamp

### Draft editing

- Must run the editor selected by `Command` in the foreground with terminal stdin, stdout, and stderr
- Must print the draft path to stderr before editing
- Must retain the draft on editor failure and include its path in the error

### struct NewDraftCommand

- Must satisfy [Draft files](#draft-files)
- Must have fields:
  - `dir: Option<PathBuf>`
    - `#[arg(long)]`
  - `edit: bool`
    - `#[arg(long)]`
- Must copy piped stdin into the draft, or create an empty draft when stdin is a terminal
- With `edit`, must satisfy [Draft editing](#draft-editing)
- Must print the draft path to stdout on success

### file src/shell/helpers.sh

- Must contain only the Bash/Zsh `crs-env` wrapper

#### function crs-env

- Must run `"$@"` with `CRS_ENV_FILE` pointing to a private temporary file unique to the invocation
- Must export each record literally, splitting at the first `=`
- Must apply output even on command failure, preserving its status
- Must remove the temporary file before returning
- Wrapper failures must return nonzero when the command succeeded
- Must scope `CRS_ENV_FILE` to the invocation
