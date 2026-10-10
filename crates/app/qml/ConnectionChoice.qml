import QtQuick
import QtQuick.Layouts
import Kante

// How the bass reaches Kontra (LH-EIN-01, LH-EIN-10): cable through an interface or the laptop
// jack; MIDI and microphone are listed as coming later (QA P5, P6), not left out.
ColumnLayout {
    id: choice

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    // [key, laptop jack, available]
    readonly property var entries: [["line", false, true], ["laptop", true, true],
                                    ["midi", false, false], ["mic", false, false]]

    spacing: 0
    Accessible.role: Accessible.List
    Accessible.name: texts.t("settings.connection")

    Repeater {
        model: choice.entries
        delegate: KanteListRow {
            required property var modelData
            Layout.fillWidth: true
            enabled: modelData[2]
            selected: enabled && ctl.laptopJack === modelData[1]
            text: texts.t("settings.source." + modelData[0])
            subtitle: texts.t("settings.source." + modelData[0] + ".hint")
            disabledReason: texts.t("settings.later.short") + " " + texts.t("settings.source." + modelData[0] + ".hint")
            onClicked: ctl.changeInstrument(ctl.strings, ctl.fretless, modelData[1])
        }
    }
}
