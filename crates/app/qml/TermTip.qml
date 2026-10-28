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
    // A click opens the entry instead of pinning the tooltip.
    onClicked: {
        checked = false
        applicationWindow().glossary(entry.id)
    }
}
