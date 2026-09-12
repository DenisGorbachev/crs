# CRS

CRS is a code review system.

## TODO

- Define `Locator`

## Decisions

- How to compose a message?
  - Options
    - Use `codex exec`, open the artifacts via commands
    - Use "External editor", open the artifacts in popups
      - How to implement popups?
        - Options
          - Via editor plugin
          - Via mux pane
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
```

## crs package

- Must have dependencies:
  - `globset`
  - `save-load`
  - `pulldown-cmark`

### struct Command

- Must have fields:
  - `config: PathBuf`
  - `db: PathBuf`
  - `user_id: UserId`
  - `session_id: SessionId` (env: `CRS_SESSION_ID`)
- Must have methods:
  - `run`
    - `let config = Format::load_one_as(&config)`
    - `let db = Db::open(db_config)`
    - `let now = Timestamp::now()`

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

- Must have fields:
  - `dir: Option<PathBuf>` /// Scope for codex commands (defaults to current dir)
  - `subcommand: CodexSubcommand`
- Must have methods:
  - `run`
    - Must load the Codex config, construct one `LocalThreadStore`, and pass a borrowed `ThreadStore` through the selected subcommands

Notes:

- Codex subcommands should use internal codex crates directly
- Codex subcommands must drop backwards compatibility for codex versions less than `v0.150.0`

### struct ThreadCodexCommand

- Must have fields:
  - `subcommand: ThreadCodexSubcommand`

### struct GetThreadCodexCommand

- Must have fields:
  - `thread_id: CodexThreadId` (positional, env: `CRS_CODEX_THREAD_ID`)
  - `subcommand: GetThreadCodexSubcommand`
- Must have methods:
  - `run`
    - Must pass `thread_id` to the selected subcommand

### struct RenderAgentMessageGetThreadCodexCommand

- Must have fields:
  - `index: usize` (`default_value_t = 0`)
- Must have methods:
  - `run`
    - `let params = list_items_params_all_reverse(thread_id)`
    - Must call `store.list_items(params)`
    - Must filter by `AgentMessage` variant
    - Must get the agent message at `index` within the first page, newest first
    - Must write the text of the agent message to `stdout`

### struct FilterThreadCodexCommand

- Must have fields:
  - `search_term: Option<String>`
  - `cwd_filters: Option<Vec<PathBuf>>`
  - `offset: usize` (`default_value_t = 0`)
  - `limit: usize` (`default_value_t = 10`)
- Must have methods:
  - `run`
    - `let params = list_threads_params_all_reverse(cwd_filters, search_term)`
    - Must paginate matching threads, skip `offset` threads, and write at most `limit` threads to `stdout` newest first via `write_jsonl`
    - Must cap the requested page size at offset plus limit and the store's maximum page size

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
  - `message_id: MessageId` (`env = "CRS_MESSAGE_ID"`)

### struct PushGetMessageCommand

- Must have fields:
  - `parts: Vec<String>`
- Must have methods:
  - `run`
    - `let item = MessageItem::try_from(parts)`
    - `let message = load::<Message>::(messages, message_id)`
    - `message.push(item)`
    - `save(messages, message_id, message)`

### struct RenderGetMessageCommand

- Must have methods:
  - `run`
    - `let message = load::<Message>::(messages, message_id)`
    - `message.write_as_markdown(&mut stdout)`

### struct GitApprovalSet

- Must have fields:
  - `value: bool` (positional)
  - `path: PathBuf` (positional)
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
  - `items: Vec<MessageItem>`

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

### file src/shell/helpers.sh

- Must target Bash and Zsh
- Must assume the existence of callables:
  - `crs-codex`
  - `crs-timestamp`
  - `info`
  - `warn`
  - `error`
- Shared helpers must be defined in the dotfiles repository and must not be redefined here

#### requirements for crs-codex exec invoker

- Every `crs-codex exec` invocation:
  - Must not add `--json`
  - Must redirect stdout to `$PWD/crs.local/$CRS_CODEX_THREAD_ID/$timestamp.md`
    - `timestamp` must be obtained by calling `crs-timestamp`
    - Must not overwrite an existing output file
  - Must stream stderr to shell
    - Rationale: the user wants to see the progress

#### function crs-codex-thread-create

- Must satisfy [requirements for crs-codex exec invoker](#requirements-for-crs-codex-exec-invoker)
- Must error if `CRS_CODEX_THREAD_ID` is set
- Must call `crs-codex exec` with "$@"
- Must parse the session id out of `crs-codex exec` output
- Must export `CRS_CODEX_THREAD_ID`

#### function crs-codex-thread-resume

- Must satisfy [requirements for crs-codex exec invoker](#requirements-for-crs-codex-exec-invoker)
- Must error if `CRS_CODEX_THREAD_ID` is not set
- Must error before calling `crs-codex exec` if the output file exists
- Must call `crs-codex exec resume` with `CRS_CODEX_THREAD_ID` and "$@"

#### function crs-codex-thread-render-message

- Must call `crs codex thread get render-agent-message "$@" | glow`

#### function crs-git-approval-next

- TODO:
  - Must show a single item
  - Must exit the pager when the end of the item is reached
