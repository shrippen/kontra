import QtQuick
import QtQuick.Layouts
import Kante

// How the bass reaches Kontra (LH-EIN-01, LH-EIN-11, LH-EIN-12): cable through an interface,
// the laptop jack, a MIDI keyboard (P5) or a microphone (P6, wider tolerances, said here).
ColumnLayout {
    id: choice

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    // [key, available]
    readonly property var entries: [["line", true], ["laptop", true], ["midi", true], ["mic", true]]

    spacing: 0
    Accessible.role: Accessible.List
    Accessible.name: texts.t("settings.connection")

    Repeater {
        model: choice.entries
        delegate: KanteListRow {
            required property var modelData
            Layout.fillWidth: true
            enabled: modelData[1]
            selected: enabled && ctl.connection === modelData[0]
            text: texts.t("settings.source." + modelData[0])
            // MIDI says which keyboard it found, or that none is connected.
            subtitle: modelData[0] !== "midi" ? texts.t("settings.source." + modelData[0] + ".hint")
                : ctl.midiPortName !== "" ? texts.t("settings.source.midi.found", { name: ctl.midiPortName })
                : texts.t("settings.source.midi.none")
            disabledReason: texts.t("settings.later.short") + " " + texts.t("settings.source." + modelData[0] + ".hint")
            onClicked: ctl.changeConnection(modelData[0])
        }
    }
}
