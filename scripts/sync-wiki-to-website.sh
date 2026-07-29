#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PY_ROOT="$ROOT"
SRC="$ROOT/wiki"
DST="$ROOT/website/docs"

DRYRUN=0
if [ "${1-}" = "--dry-run" ]; then
  DRYRUN=1
fi
export DRYRUN

if command -v python >/dev/null 2>&1; then
  PYTHON=(python)
elif command -v python3 >/dev/null 2>&1; then
  PYTHON=(python3)
elif command -v py >/dev/null 2>&1; then
  PYTHON=(py -3)
else
  echo 'error: no Python interpreter found' >&2
  exit 1
fi

"${PYTHON[@]}" - <<'PY'
import os
import pathlib
import re
import sys

root = pathlib.Path(os.environ['PY_ROOT'])
src = root / 'wiki'
dst = root / 'website' / 'docs'

mapping = {
    'Home': 'index.md',
    'Home.md': 'index.md',
    'Installation': 'guides/installation.md',
    'Installation.md': 'guides/installation.md',
    'Getting Started': 'guides/getting-started.md',
    'Getting-Started.md': 'guides/getting-started.md',
    'Playback and Players': 'guides/playback-and-players.md',
    'Playback-and-Players.md': 'guides/playback-and-players.md',
    'Downloads': 'guides/downloads.md',
    'Downloads.md': 'guides/downloads.md',
    'Configuration and History': 'guides/configuration.md',
    'Configuration-and-History.md': 'guides/configuration.md',
    'CLI Reference': 'reference/cli.md',
    'CLI-Reference.md': 'reference/cli.md',
    'FAQ': 'faq.md',
    'FAQ.md': 'faq.md',
    'Troubleshooting': 'support/troubleshooting.md',
    'Troubleshooting.md': 'support/troubleshooting.md',
    'Provider Architecture': 'development/architecture.md',
    'Provider-Architecture.md': 'development/architecture.md',
    'Security and Privacy': 'development/security.md',
    'Security-and-Privacy.md': 'development/security.md',
    'Building and Releasing': 'development/building.md',
    'Building-and-Releasing.md': 'development/building.md',
    'Contributing': 'development/contributing.md',
    'Contributing.md': 'development/contributing.md',
}

file_map = {
    'Home.md': 'index.md',
    'Installation.md': 'guides/installation.md',
    'Getting-Started.md': 'guides/getting-started.md',
    'Playback-and-Players.md': 'guides/playback-and-players.md',
    'Downloads.md': 'guides/downloads.md',
    'Configuration-and-History.md': 'guides/configuration.md',
    'CLI-Reference.md': 'reference/cli.md',
    'FAQ.md': 'faq.md',
    'Troubleshooting.md': 'support/troubleshooting.md',
    'Provider-Architecture.md': 'development/architecture.md',
    'Security-and-Privacy.md': 'development/security.md',
    'Building-and-Releasing.md': 'development/building.md',
    'Contributing.md': 'development/contributing.md',
}

link_pattern = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")

for filename, out_rel in file_map.items():
    in_path = src / filename
    if not in_path.exists():
        print(f"warning: source file not found: {in_path}", file=sys.stderr)
        continue

    out_path = dst / out_rel
    out_path.parent.mkdir(parents=True, exist_ok=True)
    text = in_path.read_text(encoding='utf-8')

    def rewrite_link(match):
        label = match.group(1)
        target = match.group(2)
        if '://' in target or target.startswith('#') or target.startswith('/'):  # external, anchor-only, absolute
            return match.group(0)
        base, sep, anchor = target.partition('#')
        key = base or target
        target_rel = None

        if key in mapping:
            target_rel = mapping[key]
        elif key.endswith('.md') and key in mapping:
            target_rel = mapping[key]
        elif key.lower() in {k.lower(): v for k, v in mapping.items()}:
            target_rel = {k.lower(): v for k, v in mapping.items()}[key.lower()]

        if target_rel is None:
            return match.group(0)

        target_path = dst / target_rel
        source_path = dst / out_rel
        relative = os.path.relpath(target_path, start=source_path.parent).replace(os.sep, '/')
        if anchor:
            relative = f"{relative}#{anchor}"
        return f"[{label}]({relative})"

    text = link_pattern.sub(rewrite_link, text)
    if os.getenv('DRYRUN') == '1':
        print(f"dry-run: {filename} -> {out_rel}")
    else:
        out_path.write_text(text, encoding='utf-8')
        print(f"copied: {filename} -> {out_rel}")
PY
