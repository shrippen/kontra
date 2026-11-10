#!/bin/bash
# merge-when-green.sh PR SHA: wait for the status "github / test" on SHA (CI on the GitHub mirror);
# merge PR when it is success.
# Checks CI only: the review (PR-Agent, closing Claude agent) is a merge condition the session
# checks before it calls this (skill pr-review, owner 22.10.2026).
# Reports MERGED only if Gitea says the PR is merged.
S=$(dirname "$0")
for i in $(seq 200); do
  out=$(python3 $S/ci-status.py $2 2>&1) && break
  sleep 30
done
echo "$out"
case "$out" in success*) ;; *) echo "NOT GREEN"; exit 1;; esac

head=$(gt api /repos/shrippen/Kontra/pulls/$1 --jq .head.sha)
case "$head" in $2*) ;; *) echo "head moved: $head"; exit 1;; esac

# head_commit_id: Gitea refuses the merge if a push moved the head since the checks above.
gt api -X POST /repos/shrippen/Kontra/pulls/$1/merge -f Do=merge -f head_commit_id="$head"
merged=$(gt api /repos/shrippen/Kontra/pulls/$1 --jq .merged)
if [ "$merged" = "true" ]; then echo "MERGED $1"; else echo "MERGE FAILED $1"; exit 1; fi
