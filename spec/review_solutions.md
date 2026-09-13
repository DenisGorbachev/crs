# Reviewing diffs with few keystrokes

The catalog is the `ReviewSolution` enum in [review_solution.rs](src/types/review_solution.rs). Every variant describes a complete proposed workflow: capture a diff fragment, compose a comment, append both to a review, and deliver the completed review to the originating agent. These are design alternatives, not implemented integrations. `properties()` describes their requirements, selection granularity, capture fidelity, workflow, and tradeoff. `ReviewDelivery` models the independent handoff choice.

The first candidates I would evaluate are `TuicrAdapter`, `NeovimCommentOperator`, and `Iterm2SelectionScript`. The first reuses a purpose-built reviewer; the second can combine selection and comment entry; the third keeps the present terminal. If exact patch bytes and locations are required, prefer a diff-aware adapter or editor over terminal text capture. A custom TUI becomes attractive if measurements show that these integrations still cost too many keys. These rankings are design judgments, not measured benchmarks.

## Count the whole interaction

For a fixed representative review, count:

1. Opening the reviewer and choosing the correct diff and originating thread.
2. Navigating to each target, entering selection mode, and extending each selection.
3. Opening composition, entering the comment, saving, and returning focus to the diff.
4. Finishing the review, opening the correct agent thread if necessary, and sending.

Count prefix keys, modifier presses, confirmation keys, and editor save/quit sequences consistently. A shortcut activation is not necessarily one physical keystroke. For example, a tmux prefix followed by a key takes more input than a direct binding. Keep comment wording and selection targets fixed across candidates. Learning time is deliberately outside this recurring cost. Record mouse actions separately; selecting with a mouse must not be described as a free keyboard selection.

The main optimization target is **select → comment → type → save-and-return**, followed by one **Finish-and-send** action per review. Capture the review identity once, restore cursor and scroll position automatically, and keep the reviewer open between comments. A bound action should transfer the selection as data instead of making the user copy it, focus another pane, paste it, and navigate back.

| Interaction design                       | Input after selecting, excluding comment text                            | Additional recurring cost                                                    |
|------------------------------------------|--------------------------------------------------------------------------|------------------------------------------------------------------------------|
| Manual clipboard and editor              | Copy, focus editor, paste, separate quote from comment, save, focus diff | Formatting and focus restoration                                             |
| Clipboard hotkey or ZLE clipboard widget | Copy, invoke composer, save-and-return                                   | ZLE also needs control of the shell prompt                                   |
| Terminal API or copy-pipe adapter        | Invoke composer, save-and-return                                         | Copy-mode entry, selection, and any prefix keys                              |
| Editor visual mapping                    | Comment binding, save-and-return                                         | Entering visual mode and selecting                                           |
| Single-key comment operator              | Motion completes selection and opens composition, then save-and-return   | Operator key and motion; avoids separate visual entry and comment invocation |
| Dedicated current-hunk action            | Comment-current-hunk, save-and-return                                    | Valid only when the intended fragment is the whole hunk                      |
| Inline review document                   | Enter annotation, return to navigation                                   | Delimiter insertion and extraction must be automated                         |

These are proposed action sequences. They are not claims that every named tool ships those bindings. A one-key operator can save a key compared with visual selection plus a comment key; a two-key operator may merely tie it. Avoid adding a leader prefix to frequently repeated commands unless its other benefits justify the extra input. Provide a multiline comment binding that actually reaches the application through the chosen terminal and multiplexer.

`tuicr` documents `v`/`V` for selecting a range, `c` or `Enter` to start its comment, and `Enter` to save with the default non-modal comment editor. That gives a concrete baseline: `v`, selection motions, `c`, comment text, `Enter`. `y` copies the assembled review, after which focus, paste, and send still remain. Its documented Markdown export shows file/line anchors and comments; the proposed adapter additionally includes the selected patch text. [tuicr keybindings](https://github.com/agavra/tuicr/blob/main/docs/KEYBINDINGS.md), [tuicr export](https://github.com/agavra/tuicr#to-your-coding-agent).

## Catalog

Each row corresponds to an enum variant. “Adapter” includes capture, persistence, and delivery glue around an existing UI. “Extension” adds UI behavior such as range selection or an editor operator. “Application” builds the reviewer itself. Selection and fidelity describe the completed proposal, including its custom code. A false `keyboard_selection` means the baseline relies on mouse/copy operations or a complete keyboard path has not been established; it is not a claim that customization is impossible.

| Variant                      | Why consider it when minimizing input?                  | Principal limitation                                          |
|------------------------------|---------------------------------------------------------|---------------------------------------------------------------|
| `ClipboardAndReviewEditor`   | Immediate baseline with familiar tools                  | Repeated copy, paste, formatting, and focus changes           |
| `DesktopClipboardHotkey`     | One composer shortcut works across viewers              | Explicit copying and reliable review routing still needed     |
| `Iterm2SelectionScript`      | Read selection and open composition in one action       | Rendered text lacks dependable patch coordinates              |
| `Iterm2ScrollbackEditor`     | Use editor motions over terminal output                 | Additional transition; scrollback can already be lossy        |
| `Iterm2HunkActions`          | Choose a stable hunk label instead of marking it        | Custom renderer and picker; whole hunks only                  |
| `KittySelectionOverlay`      | Selection becomes overlay stdin directly                | Terminal switch; keyboard selection needs more integration    |
| `KittyScrollbackNeovim`      | Reuse Neovim selection over scrollback                  | Two integrations; transcript rather than raw patch            |
| `KittyHunkHints`             | Short labels jump directly to hunk composition          | Custom hints handler; whole hunks only                        |
| `WeztermCopyModeLua`         | Keyboard selection plus a Lua action                    | Terminal switch and rendered-text limitations                 |
| `WeztermQuickSelectHunks`    | Pattern labels avoid long cursor travel                 | Stable hunk identifiers must be supplied                      |
| `TmuxCopyPipePopup`          | Selected text opens composition without copy/paste      | Copy-mode entry and prefixes still count                      |
| `TmuxPersistentReviewPane`   | Open the patch editor once per review                   | Screen space and explicit pane/thread association             |
| `TmuxHunkPickerPopup`        | Compact hunk picker beside the coding agent             | Cannot select part of a hunk                                  |
| `ZellijCopyCommand`          | Capturing text can open the comment pane                | Global copy hook must be armed for review use                 |
| `ZellijScrollbackEditor`     | Learn editor motions instead of mouse selection         | Editor transition and transcript limitations                  |
| `ZellijReviewPlugin`         | Direct range/comment bindings inside the multiplexer    | Custom renderer and selection implementation                  |
| `ZleClipboardWidget`         | Compose with familiar shell editing keys                | ZLE cannot select pager output or run over a foreground pager |
| `ZleFzfHunkWidget`           | One shell binding opens a persistent review picker      | Whole hunks only                                              |
| `ZleDiffBufferWidget`        | Reuse region operations in a dedicated ZLE context      | Large multiline patches fit the shell editor poorly           |
| `LessMarkedRangePipe`        | Keep pager navigation and pipe a marked region          | Mark/screen semantics and absent surrounding headers          |
| `FzfHunkPicker`              | Search targets directly by file or hunk text            | Preview text is not an arbitrary range selector               |
| `FzfDiffLinePicker`          | Select disjoint patch lines without a full new renderer | Repeated toggles; reconstruction must preserve original order |
| `VimPatchBuffer`             | Visual selection, comment mapping, save-and-return      | Raw selections may omit location headers                      |
| `NeovimVisualComment`        | Floating composition with automatic focus restoration   | Custom patch mapping and draft store                          |
| `NeovimCommentOperator`      | Combine selection by motion and comment entry           | Requires a custom operator and diff text objects              |
| `NeovimDiffviewComment`      | Reuse diff navigation and side-specific buffers         | Must reconstruct actual patch excerpts from source selections |
| `EmacsDiffModeComment`       | Region capture and composition in the same editor       | Raw regions may omit patch metadata                           |
| `MagitRegionComment`         | Exploit hunk and section navigation                     | Region semantics and snapshot association need care           |
| `EmacsEdiffComment`          | Comment on the current difference or a region           | Must map comparison buffers to the frozen patch               |
| `VsCodeSelectionCommand`     | Direct selection-to-composer command                    | Old/new sides and stale buffers need explicit mapping         |
| `VsCodeCommentController`    | Inline comments with configurable keyboard commands     | Local storage and Finish-and-send are custom                  |
| `TuicrAdapter`               | Existing range/comment/save keyboard loop               | Add original excerpts and originating-thread delivery         |
| `RevdiffAdapter`             | Existing line annotation and output hooks               | No documented arbitrary multiline range selection             |
| `RevdiffRangeExtension`      | Reuse revdiff while adding the missing range workflow   | Requires extending its selection and output model             |
| `CritBrowserAdapter`         | Existing local diff and inline comment forms            | Documented range interaction uses mouse dragging              |
| `GithubPullRequestReview`    | Existing pending review accumulation                    | Requires a PR and separate agent delivery                     |
| `GitlabMergeRequestReview`   | Existing draft review accumulation                      | Requires an MR and separate agent delivery                    |
| `BrowserReviewExtension`     | Add modal shortcuts to a hosted diff UI                 | Host-specific DOM and raw-patch mapping                       |
| `LocalBrowserReviewApp`      | Design exact keyboard behavior with browser rendering   | Local service and custom keyboard interaction                 |
| `StandaloneReviewTui`        | Own selection, text objects, comments, and handoff keys | Full application development                                  |
| `StandaloneDesktopReviewApp` | Rich visual layout with direct bindings                 | Desktop packaging does not itself reduce input                |
| `AgentEmbeddedReviewPane`    | Originating thread is already known                     | Requires an extensible agent UI or upstream changes           |
| `AnnotatedPatchDocument`     | Keep review prose and selected fragments in one editor  | Unambiguous annotation encoding and extraction                |

## Selection and handoff contracts

The low-input path must still preserve what was reviewed:

1. At review start, bind the draft to the producing agent's identity, explicit thread/session identifier, repository/worktree, and immutable diff snapshot. A branch name or whichever pane happens to be active is insufficient. For uncommitted changes, retain the actual patch and relevant old/new contents; commits alone cannot identify that state.
2. Each entry stores the literal selected fragment and comment. A diff-aware entry also keeps path, old/new side, coordinates, and snapshot identity. For a subline selection, preserve the exact selected bytes plus enough enclosing patch context to interpret it. Repeated identical text must not be resolved by searching for the first occurrence. File headers, renames, and mode changes need locations within the patch even when they have no source line.
3. Save appends the entry durably before restoring the review cursor. Canceled composition adds nothing and leaves earlier entries intact. Rendering styles, gutter columns, Unicode display width, and soft wrapping are not source offsets. `RenderedText` solutions retain only the selected display text and deliberately fail an exact-patch requirement.
4. Finish renders all entries and sends to the recorded origin. Do not discard the draft after sending; retain delivery state and acknowledgment so failures can be retried. If acknowledgment is ambiguous, resolve whether the prior send arrived before retrying. An API may support idempotency; a CLI adapter may require explicit reconciliation. Sending must be an explicit finish action, not a side effect of saving each comment or closing a window.
5. Agent handoff happens where the agent can read the review. If the viewer and agent are on different hosts or opposite sides of a sandbox boundary, transfer the payload or supply it as input; a local file path alone is insufficient. Pass quotes and comments as data, never executable shell text. Coordinate delivery with an active agent turn according to that agent's supported behavior.

`ReviewDelivery::ClipboardPaste` is broadly applicable but pays the final focus/paste/send cost. `ReviewFileAttachment` is useful for large drafts but may add navigation. `AgentResumeCommand` and `AgentThreadApi` are proposed adapters for combining Finish with delivery; they require that the particular agent actually supports those interfaces. A PR review submission and a prompt to the coding agent are separate events.

## Decision method

`ReviewSolution::decide(candidates, requirements, keystrokes_for_review) -> Option<ReviewSolution>` filters hard requirements, then selects the lowest supplied total keystroke count. Lower implementation effort breaks a cost tie. If both tie, the first candidate wins. No eligible candidate returns `None`; the function does not weaken requirements or substitute a mouse workflow silently. The cost callback runs once per eligible candidate and is not called for excluded candidates. Enumeration uses `ReviewSolution::iter()` from `strum::IntoEnumIterator`.

Supply only candidates available in the intended environment, including any integrations you are willing to develop. `surface` is the primary integration point, not a complete dependency or operating-system compatibility list: for example, the kitty scrollback variant also requires Neovim. The catalog has no installation or host detection. Requirements are explicit inputs because restricting implementation effort, requiring character selection, or requiring exact patch bytes changes the answer.

`ReviewKeystrokes` represents the **total for the same representative review**, including selection, navigation, typing, and handoff with a chosen `ReviewDelivery`. It is supplied by the caller so the enum does not invent benchmark results or hardcode a supposedly universal winner. Use comparable measurements, or clearly labeled estimates for unimplemented proposals. The same fixed comment text may be omitted from every estimate when it contributes exactly the same cost. Do not omit navigation merely because comment composition is identical.

The following hypothetical comparison supplies costs as arguments. It is an example of using the decision API, not an additional implementation or a claim about measured performance:

```rust
use spec::{ReviewEffort, ReviewKeystrokes, ReviewRequirements, ReviewSolution};

fn choose_review_solution(tuicr_keys: ReviewKeystrokes, operator_keys: ReviewKeystrokes) -> Option<ReviewSolution> {
    use ReviewSolution::*;
    let requirements = ReviewRequirements {
        maximum_effort: ReviewEffort::Extension,
        keyboard_only: true,
        local_only: true,
        require_exact_patch: true,
        require_file_and_side: true,
        require_partial_hunks: true,
        require_multiline_ranges: true,
        require_character_selection: false,
    };
    ReviewSolution::decide([TuicrAdapter, NeovimCommentOperator], &requirements, |solution| {
        match solution {
            NeovimCommentOperator => operator_keys,
            _ => tuicr_keys,
        }
    })
}
```

For equal costs this example chooses `TuicrAdapter` because it requires an adapter rather than an editor extension. A lower operator cost chooses `NeovimCommentOperator`. Requiring character selection excludes `TuicrAdapter` regardless of cost. Restricting effort to configuration with these other requirements produces `None`. To compare clipboard delivery with API delivery for the same UI, include that delivery's entire cost when evaluating the UI; `decide` chooses a UI solution, not an agent transport.

## Evidence behind the proposals

The following upstream documentation was consulted on 2026-09-12. It supports the underlying capabilities; the CRS capture, persistence, and delivery integrations described above remain proposals.

1. iTerm2 provides keyboard selection in Copy Mode and a Python API example that accesses selected text from a registered action. The proposed comment form and thread association are additions. [Copy Mode](https://iterm2.com/documentation-copymode.html), [selection API example](https://iterm2.com/python-api/examples/sumselection.html).
2. kitty can launch a program using selection text as stdin, and hints allow custom matching and actions. `kitty-scrollback.nvim` supplies Neovim access to scrollback. Review-specific commands are additions. [Launch](https://sw.kovidgoyal.net/kitty/launch/), [hints](https://sw.kovidgoyal.net/kitty/kittens/hints/), [kitty-scrollback.nvim](https://github.com/mikesmithgh/kitty-scrollback.nvim).
3. WezTerm exposes selection text to Lua and provides pattern-based Quick Select. The stable hunk labels and composer are additions. [Selection API](https://wezterm.org/config/lua/window/get_selection_text_for_pane.html), [Quick Select](https://wezterm.org/quickselect.html).
4. tmux supports piping copied text to a command. Zellij offers `copy_command`, a scrollback editor option, and a plugin API. Review-aware composition and routing are additions. [tmux clipboard integration](https://github.com/tmux/tmux/wiki/Clipboard), [Zellij options](https://zellij.dev/documentation/options.html), [Zellij plugin API](https://zellij.dev/documentation/plugin-api.html).
5. ZLE operates on editable command-line buffers and custom widgets; terminal scrollback capture needs another component. fzf supports multi-selection, previewing, and bound external actions. A patch index and comment store are additions. [ZLE](https://zsh.sourceforge.io/Doc/Release/Zsh-Line-Editor.html), [fzf](https://github.com/junegunn/fzf/blob/master/README.md).
6. Neovim supports custom operators through `operatorfunc`, and Diffview supports visual-mode function mappings. The review operator, patch text objects, and capture mappings are additions. [Neovim motions](https://neovim.io/doc/user/motion/), [Diffview mappings](https://github.com/sindrets/diffview.nvim).
7. Emacs offers Diff mode, Magit region/section operations, and Ediff comparison regions. Review composition and handoff commands are additions. [Diff mode](https://www.gnu.org/s/emacs/manual/html_node/emacs/Diff-Mode.html), [Magit selection](https://docs.magit.vc/magit/The-Selection.html), [Ediff](https://www.gnu.org/software/emacs/manual/html_node/ediff/Introduction.html).
8. VS Code provides comment controllers and commenting range providers for extensions. Local snapshot-backed reviews and delivery are extension responsibilities. [VS Code API](https://code.visualstudio.com/api/references/vscode-api#CommentController).
9. revdiff documents line annotations, output on quit or flush, configurable bindings, and hunk expansion based on comment text. Arbitrary selected ranges are a separate proposed extension here. [revdiff](https://github.com/umputun/revdiff).
10. Crit documents local diffs, inline line/range comments, and saved reviews, with range selection by dragging. [Crit](https://github.com/tomasz-tomczyk/crit).
11. GitHub and GitLab support collecting comments as reviews. The separate bridge to the coding agent and reconstruction of selected patch excerpts are proposals. [GitHub reviews](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/reviewing-proposed-changes-in-a-pull-request), [GitLab reviews](https://docs.gitlab.com/user/project/merge_requests/reviews/).
12. less can pipe an input-file region to a shell command using marks and screen boundaries. The review composer and convenient bindings are additions. [less manual source](https://github.com/gwsw/less/blob/master/less.nro.VER).
