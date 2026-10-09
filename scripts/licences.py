#!/usr/bin/env python3
"""Licence list for the About section (LH-NF-11): every crate linked into the app, from
`cargo metadata`, written to crates/appcore/licences.json. Run after changing dependencies;
`--check` fails when the checked-in file is out of date (CI).

    scripts/licences.py [--check]
"""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "crates/appcore/licences.json"
APP = "kontra-app"
PLATFORM = "x86_64-unknown-linux-gnu"


def linked_packages(meta):
    """Packages reachable from the app through normal (not build or dev) dependencies."""
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    packages = {p["id"]: p for p in meta["packages"]}
    start = next(p["id"] for p in meta["packages"] if p["name"] == APP)
    seen, todo = set(), [start]
    while todo:
        node = nodes[todo.pop()]
        for dep in node["deps"]:
            normal = any(k["kind"] is None for k in dep["dep_kinds"])
            if normal and dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                todo.append(dep["pkg"])
    members = set(meta["workspace_members"])
    return [packages[i] for i in seen if i not in members]


def main():
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", PLATFORM],
        cwd=ROOT))
    crates = sorted(
        ({"name": p["name"], "version": p["version"], "license": p.get("license") or "?"}
         for p in linked_packages(meta)),
        key=lambda c: (c["name"], c["version"]))
    # One crate per line: readable diffs when a dependency changes.
    text = "[\n" + ",\n".join(json.dumps(c) for c in crates) + "\n]\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit("licences.json is out of date: run scripts/licences.py")
        print(f"licences: {len(crates)} crates, up to date")
        return
    OUT.write_text(text)
    print(f"licences: {len(crates)} crates -> {OUT.relative_to(ROOT)}")


main()
