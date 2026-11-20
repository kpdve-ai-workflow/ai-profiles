#!/bin/bash

# Emit only regular report files; find does not follow symlinks by default.
report_files() {
    local root=$1
    find "$root" -type f \
        \( -path '*/reports/*.json' -o -path '*/reports/*.png' -o -path '*/reports/*.html' \) \
        -print0
}
