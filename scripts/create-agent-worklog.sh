#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'Usage: %s\n' "$0" >&2
  exit 2
fi

if ! repo_root=$(git rev-parse --show-toplevel 2>/dev/null); then
  printf 'Error: run this helper from inside a Git repository.\n' >&2
  exit 1
fi

log_dir="$repo_root/changelogs"
if ! mkdir -p "$log_dir"; then
  printf 'Error: cannot create worklog directory: %s\n' "$log_dir" >&2
  exit 1
fi

# Capture one local timestamp. The requested format is MM-DD-YYYY-HH:MM.
# The date separators are hyphens because '/' is a path separator.
read_clock() {
  local clock
  if ! clock=$(date '+%m %d %Y %H %M'); then
    printf 'Error: could not read the system date/time.\n' >&2
    return 1
  fi
  read -r month day year hour minute <<< "$clock"
  timestamp="${month}-${day}-${year}-${hour}:${minute}"
  filename_timestamp="$timestamp"
  portable_timestamp="${month}-${day}-${year}-${hour}-${minute}"
}

if ! read_clock; then
  exit 1
fi

# Exclusive creation prevents overwriting another agent's worklog. If a file
# already owns this minute, wait for the next minute and generate a fresh name.
while true; do
  path="$log_dir/${filename_timestamp}-agent-worklog.md"
  if (set -o noclobber; : > "$path") 2>/dev/null; then
    printf '%s\n' "$path"
    printf 'Timestamp: %s\n' "$timestamp" >&2
    exit 0
  fi

  if [[ -e "$path" ]]; then
    sleep 1
    if ! read_clock; then
      exit 1
    fi
    continue
  fi

  # ':' is invalid on some filesystems (notably Windows). Fall back only when
  # the requested filename cannot be created; the heading still records the
  # canonical timestamp and the helper reports the fallback explicitly.
  fallback_path="$log_dir/${portable_timestamp}-agent-worklog.md"
  if (set -o noclobber; : > "$fallback_path") 2>/dev/null; then
    printf 'Warning: this filesystem rejects colons; using %s instead.\n' "$(basename "$fallback_path")" >&2
    printf '%s\n' "$fallback_path"
    printf 'Timestamp: %s\n' "$timestamp" >&2
    exit 0
  fi

  if [[ -e "$fallback_path" ]]; then
    sleep 1
    if ! read_clock; then
      exit 1
    fi
    continue
  fi

  printf 'Error: cannot create a worklog file in %s. Check directory permissions and filename restrictions.\n' "$log_dir" >&2
  exit 1
done
