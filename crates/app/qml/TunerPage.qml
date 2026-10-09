import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Tuner (LH-TUN-01/03): note, cents and what to do, in words and symbol.
Kirigami.Page {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property bool hearing: ctl.tunerNote !== ""
    readonly property string noteName: hearing ? ctl.tunerNote.replace(/-?[0-9]+$/, "") : ""
    readonly property int octave: hearing ? Number(ctl.tunerNote.match(/-?[0-9]+$/)) : 0

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("tuner.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: ctl.startTuner()
    Component.onDestruction: ctl.stopTuner()

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("tuner.title"); pageTitle: true }

        KanteTunerGauge {
            Layout.fillWidth: true
            Layout.fillHeight: true
            noteName: page.noteName
            octave: page.octave
            cents: ctl.tunerCents
            active: page.hearing
            hint: !page.hearing ? texts.t("tuner.silent")
                : ctl.tunerCents < -3 ? texts.t("tuner.up")
                : ctl.tunerCents > 3 ? texts.t("tuner.down") : texts.t("tuner.ok")
            inTuneText: texts.t("tuner.ok")
            accessibleName: texts.t("tuner.name")
        }

        KanteLevelMeter {
            id: meter
            Layout.fillWidth: true
            peakDb: ctl.peakDb
            rmsDb: ctl.rmsDb
            clipped: ctl.clipped
            quietText: texts.t("level.quiet")
            goodText: texts.t("level.good")
            loudText: texts.t("level.loud")
            clipText: texts.t("level.clip")
            title: texts.t("level.title")
            accessibleName: texts.t("level.name")
        }
        LevelAdvice {
            Layout.fillWidth: true
            peakDb: ctl.peakDb
            clipped: ctl.clipped
            listening: ctl.tunerActive
            laptopJack: ctl.laptopJack
            onUseLaptopJack: ctl.changeInstrument(ctl.strings, ctl.fretless, true)
        }
    }
}
