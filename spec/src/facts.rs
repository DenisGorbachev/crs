pub fn programs_may_output_content_with_escape_sequences() -> bool {
    true
}

pub fn rust_items_may_be_interdependent() -> bool {
    !interdependent_rust_item_examples().is_empty()
}

pub fn interdependent_rust_item_examples() -> &'static [&'static str] {
    &[include_str!("../../fixtures/basic/src/interdependent.rs")]
}

pub fn rust_files_may_contain_meaningful_non_item_lines() -> bool {
    !meaningful_non_item_rust_lines().is_empty()
}

pub fn meaningful_non_item_rust_lines() -> &'static [&'static str] {
    &[
        "#![no_std]",
        "/// Doc comment",
        "// Regular single-line comment",
    ]
}

pub fn rust_analyzer_syntax_tree_retains_all_comments() -> bool {
    true
}

pub fn git_commits_store_files_not_hunks() -> bool {
    true
}

/// Local and SSH sandbox shells access the same Codex session storage.
pub fn codex_sessions_are_on_sandbox_volume() -> bool {
    true
}

/// Resolves the control socket in the sandbox's CODEX_HOME without host socket sharing.
pub fn codex_app_server_default_endpoint() -> &'static str {
    "unix://"
}

/// CRS connects to the app-server directly inside the sandbox.
pub fn codex_sessions_must_be_accessed_via_wyrc() -> bool {
    false
}

/// Enter a persistent sandbox shell with wyr; stateful helpers require SANDBOX=1 before creating files or running commands.
pub fn crs_commands_must_run_in_sandbox() -> bool {
    true
}

/// Helpers export CRS variables in the invoking shell; child processes cannot update their parent's environment.
pub fn crs_context_is_shell_local() -> bool {
    true
}

/// Docker exec uses the container's environment unless the caller explicitly forwards variables.
pub fn host_environment_is_automatically_forwarded_to_sandbox() -> bool {
    false
}

/// Start the sandbox before running CRS so lifecycle messages stay outside command output.
pub fn container_startup_may_write_to_stdout() -> bool {
    true
}
