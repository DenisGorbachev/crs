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

  local codex_status=1 label value
  {
    while IFS=: read -r label value; do
      case "$label" in
        "thread id") export CRS_CODEX_THREAD_ID="${value#"${value%%[![:space:]]*}"}" ;;
        "crs-thread-create status") codex_status=$value ;;
      esac
    done < <(
      set -euo pipefail
      trap 'codex_status=$?; echo; echo "crs-thread-create status:$codex_status"' EXIT
      { codex-exec "$@" >&3; } 2>&1 | tee /dev/stderr
    )
  } 3>&1

  if [[ -z ${CRS_CODEX_THREAD_ID-} ]]; then
    unset CRS_CODEX_THREAD_ID
    error "Thread id not found in codex-exec output"
    [[ $codex_status != 0 ]] || codex_status=1
  fi
  return "$codex_status"
}

function crs-thread-resume() {
  if [[ -z ${CRS_CODEX_THREAD_ID-} ]]; then
    error "CRS_CODEX_THREAD_ID is not set"
    return 1
  fi

  codex-exec resume "$CRS_CODEX_THREAD_ID" "$@"
}
