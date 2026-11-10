#!/usr/bin/env python3
"""Copies a file out of the libvirt guest `win` through the QEMU guest agent.
Usage: gpull.py <guest path> <host file>"""
import base64, json, subprocess, sys


def agent(cmd):
    out = subprocess.run(
        ["virsh", "-c", "qemu:///system", "qemu-agent-command", "win", json.dumps(cmd), "--timeout", "60"],
        capture_output=True, text=True)
    if out.returncode:
        sys.exit("agent: " + out.stderr.strip())
    return json.loads(out.stdout)["return"]


src, dst = sys.argv[1], sys.argv[2]
handle = agent({"execute": "guest-file-open", "arguments": {"path": src, "mode": "rb"}})
got = 0
with open(dst, "wb") as f:
    while True:
        part = agent({"execute": "guest-file-read", "arguments": {"handle": handle, "count": 1 << 20}})
        data = base64.b64decode(part["buf-b64"])
        f.write(data)
        got += len(data)
        if part["eof"]:
            break
agent({"execute": "guest-file-close", "arguments": {"handle": handle}})
print("got", got, "bytes")
