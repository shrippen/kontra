import QtQuick
import Kante

// "?" for the long explanation of a setting: it sits next to the control and shows the text on
// hover, focus or click, so the row stays one line (D4). `title` names what it explains.
KanteInfoTip {
    property string title: ""
    property string key: ""
    // The shown text: the key's, or one the page puts together.
    property string body: key !== "" ? applicationWindow().texts.t(key) : ""

    term: title
    info: body
    accessibleName: applicationWindow().texts.t("help.about", { term: title })
}
