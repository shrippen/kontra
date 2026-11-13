import QtQuick
import QtQuick.Controls as QQC2

// Vertical scroll bar of a Flickable whose top lies under a sticky header (settings page): the
// handle runs below the header, so the header never hides it (review round 24). Padding, not y
// and height: the attached bar sets its own height to the Flickable's (review of #321).
QQC2.ScrollBar {
    id: bar

    // Height of the header over the Flickable's top edge.
    property real headerHeight: 0

    topPadding: headerHeight + bottomPadding
}
