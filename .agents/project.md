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

Notes:

- Codex subcommands should use internal codex crates directly
- Codex subcommands must drop backwards compatibility for codex versions less than `v0.150.0`

### struct ThreadCodexCommand

- Must have fields:
  - `thread_id: CodexThreadId` (env: `CRS_CODEX_THREAD_ID`)

### struct RenderFinalAnswerThreadCodexCommand

- Must have methods:
  - `run`
    - Must get the last item in a thread by `thread_id`
    - Must return an error if it's not `FinalAnswer`
    - Must write the text of the final answer to `stdout`

### struct

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
    - Must call `insert_archived`

### enum Locator

- Must have variants:
  - `GitPath(GitPathLocator)`

### struct GitPathLocator

- Must have fields:
  - `repo_id: GitRepoId`
  - `commit_hash: GitOid`
  - `path: PathBuf`

### type MarkdownParser

- Must be a type alias of `pulldown_cmark::Parser`

### type MarkdownEvent

- Must be a type alias of `pulldown_cmark::Event`

### fn markdown_parser_new

- Must accept a Markdown document
- Must return `MarkdownParser`
- Must return `MarkdownParser::new_ext(document, Options::all())`

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
