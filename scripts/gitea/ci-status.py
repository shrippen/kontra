"""ci-status.py SHA: Gitea-Actions-Jobs zu einem Commit (Exit 0 = alle fertig, 3 = laufen noch)."""
import json, subprocess, sys
sha = sys.argv[1]
def api(path):
    return json.loads(subprocess.run(["tea","api",path],capture_output=True,text=True).stdout or "{}")
runs = [r for r in api("/repos/shrippen/Kontra/actions/runs?limit=50").get("workflow_runs", []) if r.get("head_sha","").startswith(sha)]
# Newest run first; per job only its newest run counts (an old green run must not hide a red one).
runs.sort(key=lambda r: r["id"], reverse=True)
seen = set()
done = bool(runs)
for r in runs:
    for j in api(f"/repos/shrippen/Kontra/actions/runs/{r['id']}/jobs").get("jobs", []):
        if j["name"] in seen:
            continue
        seen.add(j["name"])
        failed=[s["name"] for s in j.get("steps",[]) if s.get("conclusion")=="failure"]
        print(f"run {r['id']} {j['name']}: {j['status']} {j.get('conclusion')} {failed}")
        done &= j["status"]=="completed"
sys.exit(0 if done else 3)
