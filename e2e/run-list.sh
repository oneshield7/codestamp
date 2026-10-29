#!/usr/bin/env bash
# Runs the given suites with the test window at one size: run-list.sh <minimum|maximize|default> s1 s2 ...
# A fresh test copy is started first; test collections left behind by earlier runs are removed.
cd "$(dirname "$0")"
size="$1"
shift
if [ "$size" = "default" ]; then unset E2E_SIZE; else export E2E_SIZE="$size"; fi
node launch.mjs >/dev/null
node eval.mjs "(async () => { const i = window.__TAURI_INTERNALS__.invoke; for (const c of await i('list_collections')) if (c.name.startsWith('E2E')) await i('delete_collection', { id: c.id }); const a = (await i('get_appearance')).appearance; await i('save_appearance', { appearance: { ...a, theme: 'light' } }); location.reload(); return 'clean'; })()" >/dev/null
sleep 2
echo "== $size: viewport $(node eval.mjs '[innerWidth, innerHeight].join("x")')"
for s in "$@"; do
  out=$(timeout 420 node $s-e2e.mjs 2>&1)
  echo "$s: $(echo "$out" | grep -c '^PASS') pass, $(echo "$out" | grep -c '^FAIL') fail"
  echo "$out" | grep '^FAIL' | cut -c1-400 | head -6
  echo "$out" | grep -E 'Error:' | cut -c1-300 | head -2
done
