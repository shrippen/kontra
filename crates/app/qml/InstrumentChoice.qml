import QtQuick
import QtQuick.Layouts
import Kante

// Which bass (LH-EIN-09, QA 27.10. S11): e-bass, double bass (P6) or bass over MIDI (P5), then
// what applies to it: strings and tuning for basses with strings, frets only on the e-bass.
// "Bass over MIDI" sets the MIDI input; the stored bass stays underneath.
ColumnLayout {
    id: choice

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var keys: ["ebass", "double", "midi"]
    // Section heading above the first row (settings), "" in the setup assistant.
    property string section: ""

    spacing: 0

    KanteSettingRow {
        Layout.fillWidth: true
        section: choice.section
        title: texts.t("settings.instrument")
    }
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 0
        Accessible.role: Accessible.List
        Accessible.name: texts.t("settings.instrument")
        Repeater {
            model: choice.keys
            delegate: KanteListRow {
                required property string modelData
                Layout.fillWidth: true
                selected: ctl.instrument === modelData
                text: texts.t("settings.instrument." + modelData)
                subtitle: texts.t("settings.instrument." + modelData + ".hint")
                onClicked: ctl.chooseInstrument(modelData)
            }
        }
    }

    // A double bass has four or five strings; over MIDI there are none to count.
    KanteSettingRow {
        visible: ctl.instrument !== "midi"
        Layout.fillWidth: true
        title: texts.t("settings.strings")
        KanteSegmented {
            Accessible.name: texts.t("settings.strings")
            model: ctl.instrument === "double" ? ["4", "5"] : ["4", "5", "6"]
            currentIndex: ctl.strings - 4
            onActivated: function(index) { ctl.changeInstrument(index + 4, ctl.fretless) }
        }
    }
    // Title above the presets once they no longer fit beside it, a list below 560.
    KanteSettingRow {
        id: tuningRow
        visible: ctl.instrument !== "midi"
        Layout.fillWidth: true
        narrow: width < KanteStyle.unit(840)
        title: texts.t("settings.tuning")
        hint: texts.t("settings.tuning.hint")
        TuningChoice { asList: tuningRow.width < KanteStyle.unit(560) }
    }
    KanteSettingRow {
        visible: ctl.instrument === "ebass"
        Layout.fillWidth: true
        title: texts.t("settings.fretless")
        KanteSwitch {
            Accessible.name: texts.t("settings.fretless")
            checked: ctl.fretless
            onToggled: ctl.changeInstrument(ctl.strings, checked)
        }
    }
}
