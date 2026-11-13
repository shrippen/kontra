import QtQuick
import Kante

// The practice page's room for the notes, clipped: on small screens a staff taller than its
// room drew over the transport (QA 13.10.).
//
// A row coming or going above (mode, a callout) moves the room; the notes glide to it
// in one motion instead of jumping first (owner 12.11.). While the window is resized they
// follow at once.
//
//   change:  room moves down 80 px, notes drawn 80 px higher (still where they stood)
//   glide:   offset 80 → 0
Item {
    id: area

    default property alias content: inner.data

    readonly property bool glides: KanteStyle.animate && !applicationWindow().resizing
    property real lastY: 0
    // The first place comes from the layout as the page opens: the notes stand there at once.
    property bool placed: false

    clip: true
    onYChanged: {
        const moved = y - lastY
        lastY = y
        if (!glides || !placed) {
            return
        }
        glide.stop()
        shift.y -= moved
        glide.start()
    }

    Connections {
        target: area.Window.window
        enabled: !area.placed
        function onFrameSwapped() { area.placed = true }
    }

    Item {
        id: inner
        anchors.fill: parent
        transform: Translate { id: shift }
        // The page's ground under the notes: a layer that blurs them blurs a whole picture,
        // not lines over nothing (the sharp notes showed through).
        Rectangle {
            anchors.fill: parent
            z: -1
            color: KanteStyle.backgroundColor
        }
    }

    NumberAnimation {
        id: glide
        target: shift
        property: "y"
        to: 0
        duration: KanteStyle.duration
        easing.type: Easing.OutCubic
    }
}
