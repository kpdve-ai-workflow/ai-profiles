#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'Usage: %s <ticket|none> <scope-prefix>-<verb>-<object>\n' "$0" >&2
  exit 2
fi

ticket=$1
slug=$2
if [[ ! $ticket =~ ^(none|[A-Za-z][A-Za-z0-9]*-[0-9]+)$ ]] ||
   [[ ! $slug =~ ^[a-z0-9]+(-[a-z0-9]+)+$ ]] ||
   (( ${#slug} < 8 || ${#slug} > 60 )); then
  printf 'Error: invalid ticket or slug (kebab-case, 8–60 characters).\n' >&2
  exit 2
fi
if ! repo_root=$(git rev-parse --show-toplevel 2>/dev/null); then
  printf 'Error: run this helper from inside a Git repository.\n' >&2
  exit 1
fi
decision_dir="$repo_root/decisions"
if ! mkdir -p "$decision_dir"; then
  printf 'Error: cannot create decision directory: %s\n' "$decision_dir" >&2
  exit 1
fi
shopt -s nullglob
existing=("$decision_dir"/*-"$ticket"-"$slug".md)
if (( ${#existing[@]} )); then
  printf 'Error: ticket and slug already exist: %s\n' "${existing[0]}" >&2
  exit 1
fi
if ! timestamp=$(date '+%Y%m%d-%H%M%S'); then
  printf 'Error: could not read the system date/time.\n' >&2
  exit 1
fi
path="$decision_dir/$timestamp-$ticket-$slug.md"
if ! (set -o noclobber; : > "$path") 2>/dev/null; then
  printf 'Error: cannot create decision file: %s\n' "$path" >&2
  exit 1
fi
printf '%s\n' "$path"
printf 'Timestamp: %s\n' "$timestamp" >&2
