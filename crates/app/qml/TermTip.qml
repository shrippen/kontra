import QtQuick
import Kante

// "?" at a technical term outside the lessons (LH-LRN-09, J-G2): hover or focus shows the
// term's first sentence, a click opens its glossary entry as a layer over the page, so going
// back lands where the player was. Hidden where the place has no term (or one of another
// instrument). `topic` names the place, e.g. "rhythm.eighths" (see `Core::term_json`).
KanteInfoTip {
    id: tip

    property string topic: ""
    // Read anew when the language changes (texts.map).
    readonly property var entry: {
        var texts = applicationWindow().texts.map
        var json = topic !== "" ? applicationWindow().ctl.termJson(topic) : ""
        return json !== "" ? JSON.parse(json) : null
    }

    visible: entry !== null
    term: entry ? entry.title : ""
    info: entry ? entry.text : ""
    accessibleName: applicationWindow().texts.t("help.about", { term: term })
    // In a list row (a focus scope) the row handed its focus on to a "?" that had it once, so
    // Tab skipped the row (QA 28.10. B5). Leaving, the "?" gives the scope's focus back.
    onActiveFocusChanged: {
        if (!activeFocus) {
            focus = false
        }
    }
    // Qt's Shift+Tab passes over the list row that holds the "?": it goes to the row first.
    Keys.onBacktabPressed: function(event) {
        var row = listRow()
        event.accepted = row !== null
        if (row) {
            // Off first: the row (a focus scope) would hand the focus straight back.
            focus = false
            row.forceActiveFocus(Qt.BacktabFocusReason)
        }
    }
    // The list row the "?" sits in, or null.
    function listRow() {
        for (var p = parent; p; p = p.parent) {
            if (p instanceof KanteListRow) {
                return p
            }
        }
        return null
    }
    // A click opens the entry instead of pinning the tooltip.
    onClicked: {
        checked = false
        applicationWindow().glossary(entry.id)
    }
}
