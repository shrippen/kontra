#!/bin/bash
# merge-when-green.sh PR SHA: wait for the newest core+app runs on SHA; merge PR when both succeed
# and PR-Agent has reviewed the head commit (agent.md: findings are answered before; this script
# only checks that a review exists). Reports MERGED only if Gitea says the PR is merged.
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

# A PR-Agent review of the head commit: a review comment posted after it, or the persistent
# review (one comment, edited in place) saying "updated until commit <head>".
reviewed=$(python3 - "$1" "$head" <<'EOF'
import json, re, subprocess, sys
from datetime import datetime
pr, sha = sys.argv[1], sys.argv[2]
# Commits carry the committer's zone (+02:00), comments UTC (Z): compare instants, not text.
def at(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00"))
def api(path):
    return json.loads(subprocess.run(["tea", "api", path], capture_output=True, text=True).stdout or "null")
commit = api(f"/repos/shrippen/Kontra/git/commits/{sha}") or {}
since = at(commit.get("created") or commit["commit"]["committer"]["date"])
# A review naming a commit counts only for that commit; one naming none (the first) counts if
# posted after the head commit.
def reviews_head(c):
    named = re.search(r"updated until commit \S*/commit/([0-9a-f]+)", c["body"])
    if named:
        return sha.startswith(named.group(1)) or named.group(1).startswith(sha)
    return at(c["created_at"]) >= since
reviews = [c for c in api(f"/repos/shrippen/Kontra/issues/{pr}/comments") or []
           if "PR Reviewer Guide" in c.get("body", "") and reviews_head(c)]
print("yes" if reviews else "no")
EOF
)
if [ "$reviewed" != "yes" ]; then echo "NO REVIEW for $head"; exit 1; fi

# head_commit_id: Gitea refuses the merge if a push moved the head since the checks above.
tea api -X POST /repos/shrippen/Kontra/pulls/$1/merge -d "{\"Do\":\"merge\",\"head_commit_id\":\"$head\"}"
merged=$(tea api /repos/shrippen/Kontra/pulls/$1 | python3 -c "import json,sys;print(json.load(sys.stdin).get('merged'))")
if [ "$merged" = "True" ]; then echo "MERGED $1"; else echo "MERGE FAILED $1"; exit 1; fi
