#!/usr/bin/env bash
# Runs every e2e suite with the test window at one size: "minimum" (720 x 520) or "maximize"
# (the monitor's resolution). Suites that relaunch the app keep the size through E2E_SIZE.
# stage25 (title bar) and stage34 (updater build) run on their own.
cd "$(dirname "$0")"
export E2E_SIZE="$1"
node launch.mjs >/dev/null
node eval.mjs "(async () => { const i = window.__TAURI_INTERNALS__.invoke; const a = (await i('get_appearance')).appearance; await i('save_appearance', { appearance: { ...a, theme: 'light' } }); location.reload(); return 'light'; })()" >/dev/null
sleep 2
echo "== $E2E_SIZE: viewport $(node eval.mjs '[innerWidth, innerHeight].join("x")')"
for s in stage10 stage11 stage12 stage13 stage14 stage15 stage16 stage17 stage18 stage19 stage20 stage21 stage22 stage23 stage24 stage26 stage27 stage28 stage29 stage30 stage31 stage32 stage33 stage35 stage36 stage37 stage38 stage39 stage40 collections; do
  out=$(timeout 420 node $s-e2e.mjs 2>&1)
  echo "$s: $(echo "$out" | grep -c '^PASS') pass, $(echo "$out" | grep -c '^FAIL') fail"
  echo "$out" | grep '^FAIL' | cut -c1-400 | head -6
  echo "$out" | grep -E '^(Error|TypeError)|Error:' | cut -c1-300 | head -2
done
