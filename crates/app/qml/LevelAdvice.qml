import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Pegel-Assistent (LH-EIN-03/04): shows the advice Core derives from the level of the last
// seconds (`levelState`, `suggestLaptop`, see appcore `level.rs`); this file only words it.
ColumnLayout {
    id: advice

    /// Only shown while sound is expected (playing, or the setup's level step).
    property bool listening: true
    /// Only the advice, without what gain is: a low window keeps its room for the notes (S12).
    property bool brief: false
    signal useLaptopJack()

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property string state: listening ? ctl.levelState : ""
    readonly property bool suggestLaptop: listening && ctl.suggestLaptop

    spacing: KanteStyle.unit(6)

    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        visible: advice.state !== "" && advice.state !== "good"
        text: advice.state === "" ? "" : advice.texts.t("advice." + advice.state)
        color: KanteStyle.mutedTextColor
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }
    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        visible: !advice.brief && (advice.state === "quiet" || advice.state === "loud" || advice.state === "clip")
        text: advice.texts.t("advice.gain")
        color: KanteStyle.mutedTextColor
        font: KanteStyle.smallFont
    }
    RowLayout {
        visible: advice.suggestLaptop
        Layout.fillWidth: true
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: advice.texts.t("advice.laptop")
            color: KanteStyle.textColor
        }
        KanteButton {
            text: advice.texts.t("advice.laptop.use")
            onClicked: advice.useLaptopJack()
        }
    }
}
