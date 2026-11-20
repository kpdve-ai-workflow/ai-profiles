#!/usr/bin/env bash
# Read-only test inventory. Uses Python standard library, available in most dev environments.
set -euo pipefail
ROOT="${1:-.}"
if ! command -v python3 >/dev/null 2>&1; then
  printf 'Python 3 is not available; inspect test files manually.\n' >&2
  exit 2
fi
python3 - "$ROOT" <<'PYCODE'
import json, os, re, sys
from pathlib import Path
root = Path(sys.argv[1]).resolve()
if not root.is_dir():
    print('Expected directory: ' + str(root), file=sys.stderr)
    sys.exit(2)
excluded = {'.git', 'node_modules', '.venv', 'venv', 'target', 'dist', 'build', '.next', '.tox', 'vendor', 'coverage', '__pycache__', '.mypy_cache', '.pytest_cache'}
patterns = [
    re.compile(r'.*_test\.go$'),
    re.compile(r'(^test_.*|.*_test)\.py$'),
    re.compile(r'.*\.(test|spec)\.[cm]?[jt]sx?$'),
    re.compile(r'(^test_.*\.rs$|.*_test\.rs$)'),
    re.compile(r'.*(Test|Tests)\.java$'),
    re.compile(r'.*Tests?\.cs$'),
    re.compile(r'.*_spec\.rb$'),
    re.compile(r'.*\.feature$'),
]
files=[]
hints={'mock_files':0,'snapshot_files':0,'sleep_or_retry_files':0}
for cur, dirs, names in os.walk(root):
    dirs[:]=sorted(d for d in dirs if d not in excluded)
    for name in sorted(names):
        rel = (Path(cur)/name).relative_to(root).as_posix()
        segments = Path(rel).parts
        in_test_dir = any(part in {'tests','test','__tests__','spec','specs'} for part in segments[:-1])
        if not (any(p.fullmatch(name) for p in patterns) or (in_test_dir and Path(name).suffix in {'.py','.go','.ts','.tsx','.js','.jsx','.rs','.rb','.java','.cs'})):
            continue
        files.append(rel)
        try:
            data=(root/rel).read_text(encoding='utf-8', errors='replace')[:200000]
        except OSError:
            continue
        if re.search(r'\b(mock|mockk|jest\.fn|vi\.fn|monkeypatch|sinon|stub)\b',data,re.I): hints['mock_files']+=1
        if re.search(r'\b(snapshot|toMatchSnapshot|snapshots)\b',data,re.I): hints['snapshot_files']+=1
        if re.search(r'\b(sleep|setTimeout|retry|retries)\b',data,re.I): hints['sleep_or_retry_files']+=1
print(json.dumps({'root':str(root),'test_file_count':len(files),'test_files':files,'heuristic_file_hints':hints,'disclaimer':'File name and text heuristics only; not coverage, quality judgment, or detected defects.'},ensure_ascii=False,indent=2))
PYCODE
