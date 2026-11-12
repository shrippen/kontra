import QtQuick
import Kante

// The practice page's room for the notes, clipped: on small screens a staff taller than its
// room drew over the transport (QA 13.10.).
//
// A row coming or going above (mode, mixer, a callout) moves the room; the notes glide to it
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

    clip: true
    onYChanged: {
        const moved = y - lastY
        lastY = y
        if (!glides) {
            return
        }
        glide.stop()
        shift.y -= moved
        glide.start()
    }

    Item {
        id: inner
        anchors.fill: parent
        transform: Translate { id: shift }
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
