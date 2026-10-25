#!/usr/bin/env bash

function crs-env() {
  local crs_env_file crs_env_record crs_env_name crs_env_attributes
  local crs_env_status=0 crs_env_valid=1
  local -a crs_env_records=()
  local -x CRS_ENV_FD=4 || return
  crs_env_file=$(mktemp "${TMPDIR:-/tmp}/crs-env.XXXXXX") || return

  if {
    if {
      rm -f "$crs_env_file" && { "$@" || crs_env_status=$?; }
    } 4>>"$crs_env_file"; then
      while IFS= read -r -d '' crs_env_record <&3; do
        crs_env_name=${crs_env_record%%=*}
        case "$crs_env_record" in
        *=*) ;;
        *)
          error "Environment output contains a record without '='" || :
          crs_env_valid=0
          break
          ;;
        esac
        case "$crs_env_name" in
        ''|[0-9]*|*[!a-zA-Z0-9_]*|CRS_ENV_FD|crs_env_*)
          error "Environment output contains an invalid or reserved name" || :
          crs_env_valid=0
          break
          ;;
        esac
        if [[ -n ${ZSH_VERSION-} ]]; then
          crs_env_attributes=${parameters[$crs_env_name]-}
          crs_env_attributes=${crs_env_attributes//-local/}
          crs_env_attributes=${crs_env_attributes//-export/}
          crs_env_attributes=${crs_env_attributes//-tied/}
          crs_env_attributes=${crs_env_attributes//-special/}
        elif crs_env_attributes=$(declare -p "$crs_env_name" 2>/dev/null); then
          crs_env_attributes=${crs_env_attributes#declare -}
          crs_env_attributes=${crs_env_attributes%% *}
        fi
        case "$crs_env_attributes" in
        ''|-|x|scalar) ;;
        *)
          error "Cannot export a literal scalar to '$crs_env_name': $crs_env_attributes" || :
          crs_env_valid=0
          break
          ;;
        esac
        crs_env_records+=("$crs_env_record")
      done
      if [[ $crs_env_valid == 1 && -n $crs_env_record ]]; then
        error "Environment output contains an unterminated record" || :
        crs_env_valid=0
      fi
      [[ $crs_env_valid == 1 ]]
    fi
  } 3<"$crs_env_file"; then
    for crs_env_record in "${crs_env_records[@]}"; do
      if ! export "${crs_env_record%%=*}=${crs_env_record#*=}"; then
        error "Failed to export '${crs_env_record%%=*}'" || :
        [[ $crs_env_status != 0 ]] || crs_env_status=1
        break
      fi
    done
  else
    [[ $crs_env_status != 0 ]] || crs_env_status=1
    rm -f "$crs_env_file" || error "Failed to remove environment output file: $crs_env_file" || :
  fi
  return "$crs_env_status"
}

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
  if [[ ! -t 0 ]]; then
    cat >>"$draft_path" || return
  fi
  crs-edit "$draft_path" </dev/tty >&2 || return
  echo "$draft_path"
}

function crs-codex-thread-resume-edit() (
  set -o pipefail
  crs-require-sandbox || return
  if [[ -z ${CRS_CODEX_THREAD_ID-} ]]; then
    error "CRS_CODEX_THREAD_ID is not set"
    return 1
  fi

  local draft_path
  draft_path=$(crs codex thread get render-agent-message | crs-drafts-new-edit) || return
  crs-codex-thread-resume "$@" - <"$draft_path"
)
