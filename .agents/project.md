# crs

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
use save_load::Format;
```

## crs package

- Must have dependencies:
  - `globset`
  - `save-load`

### struct Command

- Must have fields:
  - `config: PathBuf`
  - `db: PathBuf`
  - `user_id: UserId`
  - `session_id: SessionId` (env: `SESSION_ID`)
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
