import QtQuick

// Keeps Tab and Shift+Tab inside `area` while it is shown: past its last stop to `first`,
// before `first` to its last stop. Like a dialog, for a sheet that lives in the page (the run
// summary; QA 28.10., P7: Tab went on into the sidebar). The ends are tied with KeyNavigation,
// so the focus never touches what lies outside (a stop there kept its focus frame).
QtObject {
    id: trap

    property Item area: null
    property Item first: null

    readonly property Item focusItem: area ? area.Window.activeFocusItem : null
    // The stop whose Tab leads back to `first`.
    property Item tail: null
    readonly property bool shown: area ? area.visible : false

    // Closed, the ends lead out again (the focus may still sit on a hidden stop).
    onShownChanged: {
        if (shown || !tail) {
            return
        }
        tail.KeyNavigation.tab = null
        first.KeyNavigation.backtab = null
        tail = null
    }

    function inside(item) {
        for (var p = item; p; p = p.parent) {
            if (p === area) {
                return true
            }
        }
        return false
    }

    // The area's last stop in the focus chain, starting from `first`.
    function lastStop() {
        var item = first
        for (var next = item.nextItemInFocusChain(true); next && next !== first && inside(next);
             next = next.nextItemInFocusChain(true)) {
            item = next
        }
        return item
    }

    // Each time the focus moves in the area: its content may have changed (rows shown, hidden).
    onFocusItemChanged: {
        if (!focusItem || !first || !inside(focusItem)) {
            return
        }
        var last = lastStop()
        if (tail && tail !== last) {
            tail.KeyNavigation.tab = null
        }
        tail = last
        last.KeyNavigation.tab = first
        first.KeyNavigation.backtab = last
    }
}
