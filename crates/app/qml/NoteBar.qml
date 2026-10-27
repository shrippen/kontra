import QtQuick
import QtQuick.Layouts
import Kante

// Notes for the user (errors, hints, a stand-in input) in a bar under the page; Main.qml places it.
Rectangle {
    id: noteBar

    required property var ctl
    required property var texts

    color: KanteStyle.backgroundColor
    // From the texts, not from the children: a child is only visible while the bar is, so the
    // bar stayed hidden after the first close (QA 27.10., J-B1: no import or error message).
    visible: ctl.offerInput !== "" || notice.text !== ""
    height: notes.implicitHeight + KanteStyle.unit(24)

    ColumnLayout {
        id: notes
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: KanteStyle.unit(12) }
        spacing: KanteStyle.unit(8)

        // LH-ALL-04: the interface was unplugged; another input can stand in until it is back.
        RowLayout {
            id: offer
            Layout.fillWidth: true
            visible: noteBar.ctl.offerInput !== ""
            KanteCallout {
                Layout.fillWidth: true
                kind: KanteCallout.Kind.Info
                text: noteBar.ctl.offerText
                autoHide: false
            }
            KanteButton {
                text: noteBar.texts.t("unplug.use")
                emphasis: KanteButton.Emphasis.Primary
                Accessible.name: noteBar.texts.t("unplug.use")
                onClicked: noteBar.ctl.useOfferedInput()
            }
        }

        KanteCallout {
            id: notice
            Layout.fillWidth: true
            visible: text !== ""
            kind: noteBar.ctl.errorText !== "" ? KanteCallout.Kind.Danger : KanteCallout.Kind.Info
            text: noteBar.ctl.errorText !== "" ? noteBar.ctl.errorText : noteBar.ctl.message
            dismissible: true
            autoHide: false
            onDismissed: noteBar.ctl.dismissMessage()
        }
    }
}
