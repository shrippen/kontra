#!/usr/bin/env python3
"""Drives Kontra over AT-SPI for QA (W1): find widgets by role and name, press them, read their
text, check the layout (W3). No coordinates: what a screen reader sees is what QA uses, so a
widget without a name shows up as a finding too.

Runs inside an isolated session, never against the desktop:

    scripts/qa-session.sh --a11y --input-file take.wav --run "scripts/qactl.py tree"

    qactl.py tree [--all]                       visible widgets: role 'name' [x,y w×h]
    qactl.py find [--role R] [--contains] NAME  matching widgets
    qactl.py press [--role R] [--contains] NAME first matching widget's action (press, toggle)
    qactl.py text [--role R] [--contains] NAME  its text (Text interface, else name)
    qactl.py wait [--role R] [--contains] NAME [--timeout S]
    qactl.py layout [--json]                    cut-off, overlapping, nameless widgets (W3); exit 1 if any

`--a11y` of qa-session.sh gives Kontra and the --run command one private session bus.
"""
import argparse
import json
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


def main():
    Atspi.init()
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
    layout = sub.add_parser("layout")
    layout.add_argument("--json", action="store_true")
    args = parser.parse_args()

    if args.command == "tree":
        for depth, node in walk(app(), everything=args.all):
            print("  " * depth + describe(node))
    elif args.command == "find":
        for node in find(args):
            print(describe(node))
    elif args.command == "press":
        node = first(args)
        press(node)
        print(f"pressed {describe(node)}")
    elif args.command == "text":
        print(text_of(first(args)))
    elif args.command == "wait":
        deadline = time.monotonic() + args.timeout
        while not find(args):
            if time.monotonic() > deadline:
                sys.exit(f"qactl: '{args.name}' did not appear within {args.timeout} s")
            time.sleep(POLL_S)
        print(describe(find(args)[0]))
    elif args.command == "layout":
        findings = layout_findings(app())
        if args.json:
            print(json.dumps(findings, ensure_ascii=False, indent=1))
        else:
            for f in findings:
                rest = f.get("other", "")
                print(f"{f['kind']}: {f['widget']}" + (f"  /  {rest}" if rest else ""))
        sys.exit(1 if findings else 0)


if __name__ == "__main__":
    main()
