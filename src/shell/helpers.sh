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

function crs-thread-timestamp() {
  if ! date -u +%Y-%m-%d-%H-%M-%S; then
    error "Failed to get the current UTC time"
  fi
}

function crs-thread-output-path() {
  local output_dir="$PWD/crs.local/$CRS_CODEX_THREAD_ID"
  if ! mkdir -p "$output_dir"; then
    error "Failed to create output directory: $output_dir"
    return 1
  fi

  echo "$output_dir/$1.md"
}

function crs-thread-create() {
  if [[ -n ${CRS_CODEX_THREAD_ID+x} ]]; then
    error "CRS_CODEX_THREAD_ID is already set"
    return 1
  fi

  local timestamp output_root="$PWD/crs.local" output_temp output_path codex_status=1 label value
  timestamp=$(crs-thread-timestamp) || return 1
  if ! mkdir -p "$output_root"; then
    error "Failed to create output directory: $output_root"
    return 1
  fi
  if ! output_temp=$(mktemp "$output_root/.crs-thread-create.XXXXXX"); then
    error "Failed to create a temporary output file in: $output_root"
    return 1
  fi

  while IFS=: read -r label value; do
    case "$label" in
      "session id") export CRS_CODEX_THREAD_ID="${value#"${value%%[![:space:]]*}"}" ;;
      "crs-thread-create status") codex_status=$value ;;
    esac
  done < <(
    set -euo pipefail
    trap 'codex_status=$?; echo; echo "crs-thread-create status:$codex_status"' EXIT
    { codex-exec "$@" > "$output_temp"; } 2>&1 | tee /dev/stderr
  )

  if [[ -z ${CRS_CODEX_THREAD_ID-} ]]; then
    unset CRS_CODEX_THREAD_ID
    error "Session id not found in codex-exec output"
    [[ $codex_status != 0 ]] || codex_status=1
  elif ! output_path=$(crs-thread-output-path "$timestamp") || ! mv -n "$output_temp" "$output_path" || [[ -e $output_temp ]]; then
    error "Failed to publish codex-exec output: $output_temp"
    [[ $codex_status != 0 ]] || codex_status=1
  fi
  if [[ -e $output_temp ]]; then
    warn "codex-exec stdout remains at: $output_temp"
  fi
  return "$codex_status"
}

function crs-thread-resume() {
  if [[ -z ${CRS_CODEX_THREAD_ID-} ]]; then
    error "CRS_CODEX_THREAD_ID is not set"
    return 1
  fi

  local timestamp output_path
  timestamp=$(crs-thread-timestamp) || return 1
  output_path=$(crs-thread-output-path "$timestamp") || return 1
  if [[ -e $output_path || -L $output_path ]]; then
    error "Output file already exists: $output_path"
    return 1
  fi
  codex-exec resume "$CRS_CODEX_THREAD_ID" "$@" > "$output_path"
}
