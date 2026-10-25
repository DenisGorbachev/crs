#!/usr/bin/env bash

function crs-codex-thread-output-path() {
  crs-require-sandbox || return
  local repo_root file_stem output_dir
  repo_root=$(git rev-parse --show-toplevel) || return
  file_stem=${1:?"file stem is required"}
  output_dir="$repo_root/crs.local/threads/$CRS_CODEX_THREAD_ID"
  if ! mkdir -p "$output_dir"; then
    error "Failed to create output directory: $output_dir"
    return 1
  fi

  echo "$output_dir/$file_stem.md"
}

function crs-codex-thread-create() {
  crs-require-sandbox || return
  if [[ -n ${CRS_CODEX_THREAD_ID+x} ]]; then
    error "CRS_CODEX_THREAD_ID is already set"
    return 1
  fi

  local timestamp repo_root output_root output_temp output_path codex_status=1 label value
  repo_root=$(git rev-parse --show-toplevel) || return
  timestamp=$(crs-timestamp) || return 1
  output_root="$repo_root/crs.local"
  if ! mkdir -p "$output_root"; then
    error "Failed to create output directory: $output_root"
    return 1
  fi
  if ! output_temp=$(mktemp "$output_root/.crs-codex-thread-create.XXXXXX"); then
    error "Failed to create a temporary output file in: $output_root"
    return 1
  fi

  while IFS=: read -r label value; do
    case "$label" in
    "session id") export CRS_CODEX_THREAD_ID="${value#"${value%%[![:space:]]*}"}" ;;
    "crs-codex-thread-create status") codex_status=$value ;;
    esac
  done < <(
    set -euo pipefail
    trap 'codex_status=$?; echo; echo "crs-codex-thread-create status:$codex_status"' EXIT
    { crs-codex exec "$@" >"$output_temp"; } 2>&1 | tee /dev/stderr
  )

  if [[ -z ${CRS_CODEX_THREAD_ID-} ]]; then
    unset CRS_CODEX_THREAD_ID
    error "Session id not found in crs-codex exec output"
    [[ $codex_status != 0 ]] || codex_status=1
  elif ! output_path=$(crs-codex-thread-output-path "$timestamp") || ! mv -n "$output_temp" "$output_path" || [[ -e $output_temp ]]; then
    error "Failed to publish crs-codex exec output: $output_temp"
    [[ $codex_status != 0 ]] || codex_status=1
  fi
  if [[ -e $output_temp ]]; then
    warn "crs-codex exec stdout remains at: $output_temp"
  fi
  return "$codex_status"
}

function crs-codex-thread-resume() {
  crs-require-sandbox || return
  if [[ -z ${CRS_CODEX_THREAD_ID-} ]]; then
    error "CRS_CODEX_THREAD_ID is not set"
    return 1
  fi

  local timestamp output_path
  timestamp=$(crs-timestamp) || return 1
  output_path=$(crs-codex-thread-output-path "$timestamp") || return 1
  if [[ -e $output_path || -L $output_path ]]; then
    error "Output file already exists: $output_path"
    return 1
  fi
  crs-codex exec resume "$CRS_CODEX_THREAD_ID" "$@" >"$output_path"
}

function crs-drafts-new() {
  local repo_root timestamp drafts_dir draft_path
  repo_root=$(git rev-parse --show-toplevel) || return
  timestamp=$(crs-timestamp) || return
  drafts_dir="$repo_root/.crs.local/drafts"
  mkdir -p "$drafts_dir" || return
  draft_path=$(mktemp --suffix=.md "$drafts_dir/$timestamp.XXXXXX") || return
  info "$draft_path" || return
  echo "$draft_path"
}

function crs-drafts-new-edit() {
  local draft_path
  draft_path=$(crs-drafts-new) || return
  crs-edit "$draft_path" >&2 || return
  echo "$draft_path"
}
