#!/usr/bin/env python3
"""Drives Kontra over AT-SPI for QA (W1): find widgets by role and name, press them, read their
text, check the layout (W3). No coordinates: what a screen reader sees is what QA uses, so a
widget without a name shows up as a finding too.

Runs inside an isolated session, never against the desktop. Two ways in:

    # one shot: the session lives as long as the command
    scripts/qa-session.sh --a11y --input-file take.wav --run "scripts/qactl.py tree"

    # background session for a QA round: <workdir>/qa is this script with the session's
    # environment (DISPLAY, buses, QA_WORKDIR); see the head of scripts/qa-session.sh
    scripts/qa-session.sh start W --a11y --size 1366x800 --input-file take.wav
    W/qa shot; W/qa click 640 400; W/qa key ctrl+o; W/qa type "hello"; W/qa stop

Every command inside a background session appends a line to <workdir>/actions.log:
`time<TAB>command<TAB>arguments<TAB>shot-<nr>` (the last picture taken so far, "-" before one).
The screen commands need DISPLAY of the session and refuse to run on a Wayland desktop.

    qactl.py shot                               PNG of the whole X screen: <workdir>/shot-<nr>.png
    qactl.py click X Y | hover X Y              mouse in screen pixels = pixels of the shot
    qactl.py drag X1 Y1 X2 Y2                   press at 1, move in steps, release at 2
    qactl.py key KEYS...                        xdotool key syntax (ctrl+o Return)
    qactl.py type TEXT                          typed as is

    qactl.py tree [--all]                       visible widgets: role 'name' [x,y w×h]
    qactl.py find [--role R] [--contains] NAME  matching widgets
    qactl.py press [--role R] [--contains] NAME first matching widget's action (press, toggle)
    qactl.py text [--role R] [--contains] NAME  its text (Text interface, else name)
    qactl.py wait [--role R] [--contains] NAME [--timeout S]
    qactl.py layout [--json]                    cut-off, overlapping, nameless widgets (W3); exit 1 if any
    qactl.py focus                              the widget with keyboard focus (role, name, place)

`--a11y` of qa-session.sh gives Kontra and the --run command one private session bus.
"""
import argparse
import datetime
import json
import os
import re
import shlex
import subprocess
import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

# Kontra's process on the private bus (the binary is called "kontra").
APP_NAME = "kontra"
WAIT_S = 10.0
POLL_S = 0.2
# Actions that activate a widget, in order of preference (Qt names them like this).
ACTIONS = ("press", "toggle", "click", "activate", "showmenu")
# Widgets a user acts on or reads: these must not be cut off or lie on top of each other.
INTERACTIVE = {
    "button", "push button", "toggle button", "check box", "radio button", "combo box", "slider",
    "spin button", "text", "entry", "page tab", "menu item", "list item", "link",
}
READABLE = INTERACTIVE | {"label", "static", "heading"}
# Roles that scroll their content: what lies outside them is scrolled away, not cut off.
SCROLLING = {"scroll pane", "viewport", "list", "tree", "table", "page tab list"}
SCROLL_BAR = "scroll bar"
# Mouse movements of a drag (Qt starts a drag only after some motion) and the pause between them.
DRAG_STEPS = 10
DRAG_PAUSE_S = 0.03
ACTIONS_LOG = "actions.log"
SESSION_FILE = "session.env"
SHOT_PATTERN = re.compile(r"shot-(\d+)\.png$")
SCREEN_COMMANDS = ("shot", "click", "hover", "drag", "key", "type")
# Pixels two widgets may share before they count as overlapping (antialiased borders).
TOLERANCE_PX = 2


def lookup():
    """Kontra's accessible application, None while it has not registered yet."""
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        child = desktop.get_child_at_index(i)
        if child is not None and (child.get_name() or "").lower() == APP_NAME:
            return child
    return None


def app(timeout_s=WAIT_S):
    """Kontra's accessible application; waits up to `timeout_s` for it, then exits."""
    deadline = time.monotonic() + timeout_s
    while (found := lookup()) is None:
        if time.monotonic() > deadline:
            sys.exit(f"qactl: no {APP_NAME} on the accessibility bus; "
                     "run inside scripts/qa-session.sh --a11y")
        time.sleep(POLL_S)
    return found


def showing(node):
    states = node.get_state_set()
    return states.contains(Atspi.StateType.SHOWING) and states.contains(Atspi.StateType.VISIBLE)


def walk(node, depth=0, everything=False):
    """(depth, node) of `node` and its descendants; hidden subtrees are skipped unless
    `everything`."""
    if node is None:
        return
    if not everything and depth > 0 and not showing(node):
        return
    yield depth, node
    for i in range(node.get_child_count()):
        yield from walk(node.get_child_at_index(i), depth + 1, everything)


def extents(node):
    """(x, y, w, h) on screen."""
    r = node.get_extents(Atspi.CoordType.SCREEN)
    return r.x, r.y, r.width, r.height


def describe(node):
    x, y, w, h = extents(node)
    return f"{node.get_role_name()} '{node.get_name() or ''}' [{x},{y} {w}×{h}]"


def children(node):
    return [c for c in (node.get_child_at_index(i) for i in range(node.get_child_count()))
            if c is not None]


def separator_only(node):
    """A list entry that only holds a separator and nameless parts (the drawer's dividers)."""
    parts = children(node)
    roles = {c.get_role_name() for c in parts}
    return "separator" in roles and not any((c.get_name() or "").strip() for c in parts)


def scrolls(node):
    """A widget that scrolls its content: a scrolling role or a scroll bar of its own."""
    return node.get_role_name() in SCROLLING or any(
        c.get_role_name() == SCROLL_BAR for c in children(node))


def matches(node, args):
    if args.role and node.get_role_name() != args.role:
        return False
    name = node.get_name() or ""
    if args.contains:
        return args.name.lower() in name.lower()
    return name == args.name


def find(args):
    return [n for _, n in walk(app()) if matches(n, args)]


def first(args):
    found = find(args)
    if not found:
        sys.exit(f"qactl: no widget '{args.name}'" + (f" ({args.role})" if args.role else ""))
    return found[0]


def text_of(node):
    try:
        count = Atspi.Text.get_character_count(node)
        return Atspi.Text.get_text(node, 0, count)
    except Exception:  # noqa: BLE001 (no Text interface: GLib raises a plain error)
        return node.get_name() or ""


def press(node):
    """Runs the widget's activating action; Qt lists them by name."""
    # get_name is deprecated since AT-SPI 2.10; 2.62 no longer exposes it.
    names = [Atspi.Action.get_action_name(node, i).lower()
             for i in range(Atspi.Action.get_n_actions(node))]
    for wanted in ACTIONS:
        if wanted in names:
            return Atspi.Action.do_action(node, names.index(wanted))
    sys.exit(f"qactl: {describe(node)} has no action to press (has {names})")


def inside(inner, outer, axes="xy"):
    """Whether `inner` lies in `outer` along `axes` ("x": only left and right)."""
    ix, iy, iw, ih = inner
    ox, oy, ow, oh = outer
    across = ix >= ox - TOLERANCE_PX and ix + iw <= ox + ow + TOLERANCE_PX
    down = iy >= oy - TOLERANCE_PX and iy + ih <= oy + oh + TOLERANCE_PX
    return across and (down or axes == "x")


def overlap(a, b):
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    w = min(ax + aw, bx + bw) - max(ax, bx)
    h = min(ay + ah, by + bh) - max(ay, by)
    return w > TOLERANCE_PX and h > TOLERANCE_PX


def layout_findings(root):
    """Per window: readable widgets cut off by the window edge, interactive ones on top of each
    other, interactive ones without a name. Inside a scrolling area only the sides count: what
    runs off below is scrolled, not cut (Qt marks widgets scrolled out of view as not showing).
    Each finding is a dict with "kind", "widget" and maybe "other"."""
    findings = []
    for window in (w for w in children(root) if showing(w)):
        frame = extents(window)
        placed = []

        def visit(node, scrolled):
            role = node.get_role_name()
            box = extents(node)
            sized = box[2] > 0 and box[3] > 0
            # A nameless label shows nothing that could be cut (spacers, canvases).
            named = bool((node.get_name() or "").strip())
            if sized and named and role in READABLE \
                    and not inside(box, frame, "x" if scrolled else "xy"):
                findings.append({"kind": "outside window", "widget": describe(node)})
            if sized and role in INTERACTIVE:
                if not named and not separator_only(node):
                    findings.append({"kind": "no name", "widget": describe(node)})
                placed.append((node, box))
            for child in children(node):
                if showing(child):
                    visit(child, scrolled or scrolls(node))

        visit(window, False)

        # Interactive widgets on top of each other, unless one holds the other.
        for i, (a, abox) in enumerate(placed):
            for b, bbox in placed[i + 1:]:
                if inside(abox, bbox) or inside(bbox, abox):
                    continue
                if overlap(abox, bbox):
                    findings.append({"kind": "overlap", "widget": describe(a),
                                     "other": describe(b)})
    return findings


def workdir():
    """The session's folder (QA_WORKDIR of the wrapper), None outside a background session."""
    return os.environ.get("QA_WORKDIR")


def shot_numbers(folder):
    return sorted(int(m.group(1)) for f in os.listdir(folder) if (m := SHOT_PATTERN.match(f)))


def log_action(command, rest):
    """Appends the action to actions.log with the number of the last picture."""
    folder = workdir()
    if not folder:
        return
    numbers = shot_numbers(folder)
    last = f"shot-{numbers[-1]:03d}" if numbers else "-"
    with open(os.path.join(folder, ACTIONS_LOG), "a", encoding="utf-8") as f:
        f.write("\t".join([datetime.datetime.now().isoformat(timespec="milliseconds"),
                           command, " ".join(rest), last]) + "\n")


def session_display(folder):
    """DISPLAY recorded in the session file of `folder`, None if there is none."""
    try:
        with open(os.path.join(folder, SESSION_FILE), encoding="utf-8") as f:
            for line in f:
                key, _, value = line.partition("=")
                if key == "DISPLAY":
                    return shlex.split(value)[0]
    except OSError:
        return None
    return None


def require_x():
    """The screen commands act on the session's X server only: QA_WORKDIR is set and DISPLAY is
    the one in that session's file (so never a Wayland desktop or the owner's X server)."""
    folder = workdir()
    wanted = session_display(folder) if folder else None
    if not wanted or os.environ.get("WAYLAND_DISPLAY") or os.environ.get("DISPLAY") != wanted:
        sys.exit("qactl: not inside a session (QA_WORKDIR and DISPLAY must match its session.env); "
                 "use <workdir>/qa of scripts/qa-session.sh start")


def xdo(*argv):
    subprocess.run(["xdotool", *argv], check=True)


def shot():
    """Saves the X screen as the next shot-<nr>.png of the session and prints the path."""
    folder = workdir()
    if not folder:
        sys.exit("qactl: shot needs a background session (QA_WORKDIR)")
    numbers = shot_numbers(folder)
    path = os.path.join(folder, f"shot-{(numbers[-1] + 1 if numbers else 1):03d}.png")
    subprocess.run(["import", "-window", "root", path], check=True)
    print(path)


def screen_command(args):
    require_x()
    if args.command == "shot":
        shot()
    elif args.command in ("click", "hover"):
        xdo("mousemove", "--sync", str(args.x), str(args.y))
        if args.command == "click":
            xdo("click", "1")
    elif args.command == "drag":
        xdo("mousemove", "--sync", str(args.x1), str(args.y1), "mousedown", "1")
        for i in range(1, DRAG_STEPS + 1):
            time.sleep(DRAG_PAUSE_S)
            x = args.x1 + (args.x2 - args.x1) * i // DRAG_STEPS
            y = args.y1 + (args.y2 - args.y1) * i // DRAG_STEPS
            xdo("mousemove", str(x), str(y))
        xdo("mouseup", "1")
    elif args.command == "key":
        xdo("key", "--", *args.keys)
    elif args.command == "type":
        xdo("type", "--", args.text)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    tree = sub.add_parser("tree")
    tree.add_argument("--all", action="store_true", help="hidden widgets too")
    for name in ("find", "press", "text", "wait"):
        p = sub.add_parser(name)
        p.add_argument("--role")
        p.add_argument("--contains", action="store_true", help="name contains NAME, any case")
        p.add_argument("name")
        if name == "wait":
            p.add_argument("--timeout", type=float, default=WAIT_S)
    sub.add_parser("focus")
    sub.add_parser("shot")
    for name in ("click", "hover"):
        p = sub.add_parser(name)
        p.add_argument("x", type=int)
        p.add_argument("y", type=int)
    drag = sub.add_parser("drag")
    for name in ("x1", "y1", "x2", "y2"):
        drag.add_argument(name, type=int)
    sub.add_parser("key").add_argument("keys", nargs="+")
    sub.add_parser("type").add_argument("text")
    layout = sub.add_parser("layout")
    layout.add_argument("--json", action="store_true")
    args = parser.parse_args()
    # Logged after the action, so a shot is already the last picture in its own line.
    try:
        run(args)
    finally:
        log_action(args.command, sys.argv[2:])


def run(args):
    if args.command in SCREEN_COMMANDS:
        screen_command(args)
        return
    # No private AT-SPI bus: libatspi would start or find another one.
    if not os.environ.get("AT_SPI_BUS_ADDRESS"):
        sys.exit("qactl: no AT_SPI_BUS_ADDRESS; start the session with --a11y")
    Atspi.init()
    atspi_command(args)


def cmd_tree(args):
    for depth, node in walk(app(), everything=args.all):
        print("  " * depth + describe(node))


def cmd_find(args):
    for node in find(args):
        print(describe(node))


def cmd_press(args):
    node = first(args)
    press(node)
    print(f"pressed {describe(node)}")


def cmd_text(args):
    print(text_of(first(args)))


def cmd_wait(args):
    deadline = time.monotonic() + args.timeout
    while not find(args):
        if time.monotonic() > deadline:
            sys.exit(f"qactl: '{args.name}' did not appear within {args.timeout} s")
        time.sleep(POLL_S)
    print(describe(find(args)[0]))


def cmd_focus(_args):
    # The deepest focused widget: Qt marks the window and the item both.
    focused = [node for _, node in walk(app())
               if node.get_state_set().contains(Atspi.StateType.FOCUSED)]
    print(describe(focused[-1]) if focused else "qactl: nothing has focus")


def cmd_layout(args):
    findings = layout_findings(app())
    if args.json:
        print(json.dumps(findings, ensure_ascii=False, indent=1))
    else:
        for f in findings:
            rest = f.get("other", "")
            print(f"{f['kind']}: {f['widget']}" + (f"  /  {rest}" if rest else ""))
    sys.exit(1 if findings else 0)


def atspi_command(args):
    {"tree": cmd_tree, "find": cmd_find, "press": cmd_press, "text": cmd_text,
     "wait": cmd_wait, "focus": cmd_focus, "layout": cmd_layout}[args.command](args)


if __name__ == "__main__":
    main()
