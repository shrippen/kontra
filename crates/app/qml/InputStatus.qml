import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// The input's state (LH-EIN-15, appcore `input_state.rs`). Two parts: "connected" as one line
// where the level shows (`Part.Line`), a fault as a callout with "Erneut versuchen" in the note
// bar on every page (`Part.Faults`). A fault stays until the input works, not until dismissed.
// Each part tells screen readers its own changes; the sign and the words carry it, not the colour.
ColumnLayout {
    id: status

    enum Part {
        Line,
        Faults
    }

    required property var ctl
    required property var texts
    property int part: InputStatus.Part.Faults

    readonly property string state: ctl.inputState
    readonly property bool fault: state === "nosignal" || state === "failed" || state === "denied"
    readonly property bool shown: part === InputStatus.Part.Faults ? fault : state === "connected"
    readonly property string line: ctl.inputName !== ""
        ? texts.t("input.state.connected", { name: ctl.inputName })
        : texts.t("input.state.connected.plain")

    visible: shown
    spacing: 0

    // After the bindings followed the new state; also from one fault to another.
    onStateChanged: Qt.callLater(tell)
    function tell() {
        if (shown) {
            Accessible.announce(part === InputStatus.Part.Faults ? callout.title + ". " + callout.text : line)
        }
    }

    RowLayout {
        visible: status.part === InputStatus.Part.Line
        Layout.fillWidth: true
        spacing: KanteStyle.unit(8)
        KanteLamp {
            color: KanteStyle.positiveTextColor
            Layout.alignment: Qt.AlignVCenter
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: status.line
            color: KanteStyle.mutedTextColor
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }
    }

    KanteCallout {
        id: callout
        visible: status.part === InputStatus.Part.Faults
        Layout.fillWidth: true
        kind: status.state === "nosignal" ? KanteCallout.Kind.Warn : KanteCallout.Kind.Danger
        title: status.fault ? status.texts.t("input.state." + status.state) : ""
        text: status.fault ? status.texts.t("input.state." + status.state + ".text") : ""
        autoHide: false
        Accessible.role: Accessible.AlertMessage
        Accessible.name: title
        Accessible.description: text

        KanteButton {
            objectName: "inputRetry"
            text: status.texts.t("input.retry")
            emphasis: KanteButton.Emphasis.Primary
            Accessible.name: text
            onClicked: status.ctl.retryInput()
        }
    }
}
