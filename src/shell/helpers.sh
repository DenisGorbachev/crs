function info() {
  echo "[I] $*" >&2
}

function warn() {
  echo "[W] $*" >&2
}

function error() {
  echo "[E] $*" >&2
  return 1
}

function crs-thread-create() {
  if [[ -n ${CRS_CODEX_THREAD_ID+x} ]]; then
    error "CRS_CODEX_THREAD_ID is already set"
    return 1
  fi

  local capture_dir
  if ! capture_dir=$(mktemp -d "${TMPDIR:-/tmp}/crs-thread-create.XXXXXX"); then
    error "Failed to create a temporary directory for codex-exec output"
    return 1
  fi

  local capture_done="$capture_dir/done"
  local stderr_file="$capture_dir/stderr"
  if ! mkfifo "$capture_done"; then
    rmdir "$capture_dir"
    error "Failed to create a pipe for codex-exec output"
    return 1
  fi

  local codex_status
  local capture_status
  local helper_status=0
  {
    if codex-exec "$@" 2> >(
      set +e
      tee "$stderr_file" >&2
      echo "$?" > "$capture_done"
    ); then
      codex_status=0
    else
      codex_status=$?
    fi

    # Synchronize with this process substitution without waiting for unrelated shell jobs.
    if ! IFS= read -r capture_status <&9; then
      error "Failed to wait for codex-exec output"
      helper_status=1
    elif [[ $capture_status != 0 ]]; then
      error "Failed to capture codex-exec output"
      helper_status=1
    fi
  } 9<>"$capture_done" || {
    if ! rm -f "$capture_done" || ! rmdir "$capture_dir"; then
      warn "Failed to remove temporary codex-exec output"
    fi
    error "Failed to open the pipe for codex-exec output"
    return 1
  }

  local thread_id
  if ! thread_id=$(sed -n 's/^thread id:[[:space:]]*//p' "$stderr_file" | tail -n 1); then
    error "Failed to parse the thread id from codex-exec output"
    helper_status=1
  elif [[ -z $thread_id ]]; then
    error "Thread id not found in codex-exec output"
    helper_status=1
  else
    export CRS_CODEX_THREAD_ID="$thread_id"
  fi

  if ! rm -f "$capture_done" "$stderr_file" || ! rmdir "$capture_dir"; then
    warn "Failed to remove temporary codex-exec output"
  fi

  if [[ $codex_status != 0 ]]; then
    return "$codex_status"
  fi
  return "$helper_status"
}

function crs-thread-resume() {
  if [[ -z ${CRS_CODEX_THREAD_ID+x} || -z $CRS_CODEX_THREAD_ID ]]; then
    error "CRS_CODEX_THREAD_ID is not set"
    return 1
  fi

  codex-exec resume "$CRS_CODEX_THREAD_ID" "$@"
}
