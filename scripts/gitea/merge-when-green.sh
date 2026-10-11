#!/bin/bash
# merge-when-green.sh PR SHA: wait for the newest core+app runs on SHA; merge PR when both succeed.
S=$(dirname "$0")
for i in $(seq 200); do
  out=$(python3 $S/ci-status.py $2 2>&1)
  core=$(echo "$out" | grep " core: " | head -1); app=$(echo "$out" | grep " app: " | head -1)
  if echo "$core" | grep -q completed && echo "$app" | grep -q completed; then break; fi
  sleep 30
done
echo "$core"; echo "$app"
if echo "$core" | grep -q "completed success" && echo "$app" | grep -q "completed success"; then
  head=$(tea api /repos/shrippen/Kontra/pulls/$1 | python3 -c "import json,sys;print(json.load(sys.stdin)['head']['sha'])")
  case "$head" in $2*) tea api -X POST /repos/shrippen/Kontra/pulls/$1/merge -d '{"Do":"merge"}'; echo "MERGED $1";; *) echo "head moved: $head";; esac
else echo "NOT GREEN"; fi
