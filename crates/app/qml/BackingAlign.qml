import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Lines the bars up with the beats of the backing track (LH-BIB-06): automatic beat detection,
// then corrections by hand (bar 1 by a beat, all bars or one bar by milliseconds).
KanteCard {
    id: sheet

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var info: ({})
    property int bar: 1

    function refresh() { info = JSON.parse(ctl.backingJson()) }
    function open() { refresh(); visible = true }
    function close() { visible = false }

    visible: false
    height: content.implicitHeight + KanteStyle.unit(32)
    barColor: KanteStyle.accentColor
    color: KanteStyle.backgroundColor
    borderColor: KanteStyle.frameColor

    Connections {
        target: ctl
        function onArrangementLoaded() { if (sheet.visible) sheet.refresh() }
    }

    ColumnLayout {
        id: content
        x: KanteStyle.unit(16)
        y: KanteStyle.unit(16)
        width: sheet.width - KanteStyle.unit(32)
        spacing: KanteStyle.unit(12)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: texts.t("backing.align.title"); Layout.fillWidth: true }
            KanteButton { text: texts.t("summary.close"); emphasis: KanteButton.Emphasis.Quiet; onClicked: sheet.close() }
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t("backing.align.hint")
        }
        KanteCallout {
            Layout.fillWidth: true
            kind: sheet.info.analysed ? KanteCallout.Kind.Ok : KanteCallout.Kind.Info
            autoHide: false
            title: sheet.info.analysed ? sheet.info.text : texts.t("backing.notaligned")
            Accessible.name: title
            KanteButton {
                text: texts.t("backing.analyse")
                emphasis: sheet.info.analysed ? KanteButton.Emphasis.Normal : KanteButton.Emphasis.Primary
                onClicked: { ctl.backingAnalyse(); sheet.refresh() }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: sheet.info.analysed === true
            spacing: KanteStyle.unit(8)

            Flow {
                Layout.fillWidth: true
                spacing: KanteStyle.unit(8)
                KanteButton { text: texts.t("backing.earlier"); onClicked: ctl.backingAdjust(-1, 0, -1, 0) }
                KanteButton { text: texts.t("backing.later"); onClicked: ctl.backingAdjust(1, 0, -1, 0) }
            }
            KanteSettingRow {
                Layout.fillWidth: true
                title: texts.t("backing.shift")
                RowLayout {
                    KanteButton { text: "−10 ms"; Accessible.name: texts.t("backing.shift") + " −10 ms"; onClicked: ctl.backingAdjust(0, -10, -1, 0) }
                    KanteButton { text: "+10 ms"; Accessible.name: texts.t("backing.shift") + " +10 ms"; onClicked: ctl.backingAdjust(0, 10, -1, 0) }
                }
            }
            KanteSettingRow {
                Layout.fillWidth: true
                title: texts.t("backing.bar")
                RowLayout {
                    QQC2.SpinBox {
                        Accessible.name: texts.t("backing.barnumber")
                        from: 1
                        to: Math.max(1, sheet.info.bars || 1)
                        value: sheet.bar
                        editable: true
                        onValueModified: sheet.bar = value
                    }
                    KanteButton { text: "−10 ms"; Accessible.name: texts.t("backing.barnumber") + " " + sheet.bar + " −10 ms"; onClicked: ctl.backingAdjust(0, 0, sheet.bar - 1, -10) }
                    KanteButton { text: "+10 ms"; Accessible.name: texts.t("backing.barnumber") + " " + sheet.bar + " +10 ms"; onClicked: ctl.backingAdjust(0, 0, sheet.bar - 1, 10) }
                }
            }
            KanteButton {
                text: texts.t("backing.clear")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: { ctl.backingClear(); sheet.refresh() }
            }
        }
    }
}
