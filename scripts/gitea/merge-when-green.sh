#!/bin/bash
# merge-when-green.sh PR SHA: wait for the newest core+app runs on SHA; merge PR when both succeed.
# PR-Agent runs only on the owner's /review and is no merge condition (owner, 18.10.2026).
# Reports MERGED only if Gitea says the PR is merged.
S=$(dirname "$0")
for i in $(seq 200); do
  out=$(python3 $S/ci-status.py $2 2>&1)
  core=$(echo "$out" | grep " core: " | head -1); app=$(echo "$out" | grep " app: " | head -1)
  if echo "$core" | grep -q completed && echo "$app" | grep -q completed; then break; fi
  sleep 30
done
echo "$core"; echo "$app"
if ! { echo "$core" | grep -q "completed success" && echo "$app" | grep -q "completed success"; }; then
  echo "NOT GREEN"; exit 1
fi

head=$(tea api /repos/shrippen/Kontra/pulls/$1 | python3 -c "import json,sys;print(json.load(sys.stdin)['head']['sha'])")
case "$head" in $2*) ;; *) echo "head moved: $head"; exit 1;; esac

# head_commit_id: Gitea refuses the merge if a push moved the head since the checks above.
tea api -X POST /repos/shrippen/Kontra/pulls/$1/merge -d "{\"Do\":\"merge\",\"head_commit_id\":\"$head\"}"
merged=$(tea api /repos/shrippen/Kontra/pulls/$1 | python3 -c "import json,sys;print(json.load(sys.stdin).get('merged'))")
if [ "$merged" = "True" ]; then echo "MERGED $1"; else echo "MERGE FAILED $1"; exit 1; fi
