import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// "Play on the click" counting along (QA 14.10.: P1 heard clicks only and did not know whether
// she was in the count-in or already measured): the click in words and how far it is.
ColumnLayout {
    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property int beat: ctl.calibrationBeat
    // The count-in's four clicks, then the eight measured ones.
    readonly property int total: 12

    visible: ctl.calibrating
    spacing: KanteStyle.unit(6)

    QQC2.Label {
        Layout.fillWidth: true
        text: parent.beat < -4 || parent.beat > 7 ? texts.t("calibration.listen")
            : parent.beat < 0 ? texts.t("calibration.countin", { n: parent.beat + 5 })
            : texts.t("calibration.beat", { n: parent.beat + 1 })
        color: KanteStyle.strongTextColor
        font.pointSize: KanteStyle.defaultFont.pointSize * 1.6
        font.bold: true
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }
    KanteProgressBar {
        Layout.fillWidth: true
        Layout.maximumWidth: KanteStyle.unit(360)
        value: parent.beat < -4 ? 0 : Math.min(1, (parent.beat + 5) / parent.total)
    }
}
