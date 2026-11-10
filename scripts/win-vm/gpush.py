#!/usr/bin/env python3
"""Copies a host file into the libvirt guest `win` through the QEMU guest agent.
Usage: gpush.py <host file> <guest path>   (60 kB chunks, one argument of virsh holds no more; the host firewall blocks the VM network)"""
import base64, json, subprocess, sys


def agent(cmd):
    out = subprocess.run(
        ["virsh", "-c", "qemu:///system", "qemu-agent-command", "win", json.dumps(cmd), "--timeout", "60"],
        capture_output=True, text=True)
    if out.returncode:
        sys.exit("agent: " + out.stderr.strip())
    return json.loads(out.stdout)["return"]


src, dst = sys.argv[1], sys.argv[2]
handle = agent({"execute": "guest-file-open", "arguments": {"path": dst, "mode": "wb"}})
sent = 0
with open(src, "rb") as f:
    while chunk := f.read(60_000):
        agent({"execute": "guest-file-write",
               "arguments": {"handle": handle, "buf-b64": base64.b64encode(chunk).decode()}})
        sent += len(chunk)
agent({"execute": "guest-file-close", "arguments": {"handle": handle}})
print("sent", sent, "bytes to", dst)
