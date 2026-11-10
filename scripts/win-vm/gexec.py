#!/usr/bin/env python3
"""Runs a PowerShell command in the libvirt guest `win` through the QEMU guest agent (README.md).
Usage: gexec.py [--timeout S] '<powershell>'  -> prints stdout/stderr, exits with the guest's code."""
import base64, json, subprocess, sys, time

def agent(cmd):
    out = subprocess.run(["virsh", "-c", "qemu:///system", "qemu-agent-command", "win", json.dumps(cmd), "--timeout", "30"],
                         capture_output=True, text=True)
    if out.returncode:
        sys.exit("agent: " + out.stderr.strip())
    return json.loads(out.stdout)["return"]

args = sys.argv[1:]
timeout = 120
if args and args[0] == "--timeout":
    timeout = int(args[1]); args = args[2:]
script = "$ProgressPreference='SilentlyContinue'; " + args[0]
enc = base64.b64encode(script.encode("utf-16-le")).decode()
pid = agent({"execute": "guest-exec", "arguments": {"path": "powershell.exe",
      "arg": ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", enc],
      "capture-output": True}})["pid"]
end = time.time() + timeout
while True:
    st = agent({"execute": "guest-exec-status", "arguments": {"pid": pid}})
    if st["exited"]:
        break
    if time.time() > end:
        sys.exit("timeout (guest pid %d still running)" % pid)
    time.sleep(1)
for key in ("out-data", "err-data"):
    if key in st:
        sys.stdout.write(base64.b64decode(st[key]).decode("utf-8", "replace"))
sys.exit(st.get("exitcode", 0))
