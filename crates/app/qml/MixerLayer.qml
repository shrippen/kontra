import QtQuick
import Kante

// The mixer's place on the practice page: a layer over the notes and what lies under them
// (fretboard, hint), from the notes' top edge down to the top of the transport (owner 13.11.:
// inside the notes area it was two rows high, the area being 250 px). It is a sibling of the
// page's column, so the column is its blur source and its room is not the column's.
//
//   rows above      uncovered
//   --- startRow     the mixer slides down from here
//   notes
//   fretboard       up to 2/3 of the page, not further than "stopRow"
//   --- stopRow  transport: never covered
Item {
    id: layer

    /** The page's column of rows; the layer shares its origin and width, and it is the blur source. */
    property Item column: null
    /** The mixer slides down from this item's top edge. */
    property Item startRow: null
    /** The row that stays free: the mixer stops above its top edge. */
    property Item stopRow: null
    property alias open: card.open
    /** The most of the page's height the mixer takes, the rest scrolls. */
    property real pageShare: 2 / 3

    readonly property real room: startRow && stopRow ? Math.max(0, stopRow.y - startRow.y) : 0

    x: column ? column.x : 0
    y: column && startRow ? column.y + startRow.y : 0
    width: column ? column.width : 0
    height: room
    // The panel is outside its place while it slides.
    clip: true

    MixerCard {
        id: card
        anchors { left: parent.left; right: parent.right; top: parent.top }
        blurSource: layer.column
        maxHeight: Math.min(layer.room, (layer.parent ? layer.parent.height : 0) * layer.pageShare)
    }
}
