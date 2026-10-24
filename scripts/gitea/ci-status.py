"""ci-status.py SHA: newest status "github / test" of a commit (CI on the GitHub mirror).

Prints "<state> <description> <url>". Exit 0 = finished (success, failure, error), 3 = pending or
no status yet.
"""
import json, subprocess, sys

CONTEXT = "github / test"
sha = sys.argv[1]
out = subprocess.run(["tea", "api", f"/repos/shrippen/Kontra/commits/{sha}/statuses"],
                     capture_output=True, text=True).stdout
statuses = [s for s in json.loads(out or "[]") if s.get("context") == CONTEXT]
if not statuses:
    print("none")
    sys.exit(3)
# Only the newest status counts (pending first, then the result).
newest = max(statuses, key=lambda s: s["id"])
print(newest["status"], newest.get("description", ""), newest.get("target_url", ""))
sys.exit(3 if newest["status"] == "pending" else 0)
