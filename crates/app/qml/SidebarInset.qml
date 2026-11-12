import QtQuick
import Kante

// The page's left margin beside the sidebar. The page gets its width once the sidebar has
// settled, not on every frame of the slide: each width change relays out the notes (60 ms and
// more), and the slide stuttered (QA 11.11., P4). Meanwhile the sidebar slides over the page.
//
//   frame:  page |#### sidebar slides ####|        page keeps its old width
//   settled: [sidebar][ page, new width    ]       one relayout
QtObject {
    id: inset

    /// How far the sidebar reaches into the window now.
    required property real edge
    /// The presentation hides the sidebar at once, so the page takes the room at once too.
    property bool presenting: false
    /// The margin the page uses.
    property real margin: 0
    /// At start: the sidebar has its width and place (no change for `restMs`), the window may
    /// show. It found them in steps (none, collapsed, open), each one moving the page (owner
    /// 12.11.). Until then the page follows at once.
    property bool ready: false
    readonly property int restMs: 150

    onEdgeChanged: {
        if (!ready) {
            rest.restart()
        }
        if (!KanteStyle.animate || presenting || !ready) {
            margin = edge
            return
        }
        settle.restart()
    }

    readonly property Timer rest: Timer {
        interval: inset.restMs
        running: true
        onTriggered: inset.ready = true
    }
    readonly property Timer settle: Timer {
        interval: KanteStyle.durationSlow + 50
        onTriggered: inset.margin = inset.edge
    }
}
