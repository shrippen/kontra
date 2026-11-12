import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Call and response (LH-LRN-19): the switch for a piece with chords, and while the tab is
// hidden why it is (the player answers by ear; it shows after two failed tries).
ColumnLayout {
    id: callResponse

    /// The piece has chords and at least one call's worth of bars.
    property bool possible: false

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts

    spacing: KanteStyle.unit(8)
    // Only as high as its content: a nested layout with no child shown fills by default and
    // took half the notes' room on the practice page (owner 12.11.).
    Layout.fillHeight: false

    Row {
        visible: callResponse.possible || callResponse.ctl.call
        spacing: KanteStyle.unit(8)
        height: KanteStyle.heightMedium
        KanteSwitch {
            anchors.verticalCenter: parent.verticalCenter
            checked: callResponse.ctl.call
            Accessible.name: callResponse.texts.t("call.title")
            Accessible.description: callResponse.texts.t("call.hint")
            onToggled: callResponse.ctl.changeCall(checked)
        }
        QQC2.Label {
            anchors.verticalCenter: parent.verticalCenter
            text: callResponse.texts.t("call.title")
            color: KanteStyle.textColor
        }
        KanteInfoTip {
            anchors.verticalCenter: parent.verticalCenter
            term: callResponse.texts.t("call.title")
            info: callResponse.texts.t("call.hint")
            accessibleName: callResponse.texts.t("help.about", { term: callResponse.texts.t("call.title") })
        }
    }

    KanteCallout {
        Layout.fillWidth: true
        visible: callResponse.ctl.callHidden
        kind: KanteCallout.Kind.Info
        autoHide: false
        title: callResponse.texts.t("call.listen")
        onVisibleChanged: { if (visible) { Accessible.announce(title) } }
    }
}
