import QtQuick
import QtQuick.Layouts
import Kante

// How the bass reaches Kontra (LH-EIN-01, LH-EIN-11, LH-EIN-12): cable through an interface,
// the laptop jack, a MIDI keyboard (P5) or a microphone (P6, wider tolerances, said here).
ColumnLayout {
    id: choice

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    // [key, available], by instrument (LH-EIN-09): MIDI comes with "bass over MIDI", a double
    // bass suggests the microphone first, its pickup goes through the interface.
    readonly property var entries: ctl.instrument === "midi" ? [["midi", true]]
        : ctl.instrument === "double" ? [["mic", true], ["line", true], ["laptop", true]]
        : [["line", true], ["laptop", true], ["mic", true]]
    // MIDI devices (LH-EIN-12, round 9): {ports: [{name, chosen}], missingText}.
    readonly property var midi: ctl.midiPortsJson !== "" ? JSON.parse(ctl.midiPortsJson) : ({ ports: [], missingText: "" })

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
            subtitle: modelData[0] === "line" && ctl.instrument === "double" ? texts.t("settings.source.line.double.hint")
                : modelData[0] !== "midi" ? texts.t("settings.source." + modelData[0] + ".hint")
                : ctl.midiPortName !== "" ? texts.t("settings.source.midi.found", { name: ctl.midiPortName })
                : texts.t("settings.source.midi.none")
            disabledReason: texts.t("settings.later.short") + " " + texts.t("settings.source." + modelData[0] + ".hint")
            onClicked: ctl.changeConnection(modelData[0])
        }
    }

    // The microphone's looser judging in numbers: too long for its row, which cut it off at
    // half a laptop screen (QA 14.10.).
    KanteCallout {
        Layout.fillWidth: true
        visible: ctl.connection === "mic"
        kind: KanteCallout.Kind.Info
        text: texts.t("settings.source.mic.detail")
        autoHide: false
    }

    // Which keyboard, when several are connected; a chosen one that is missing is said here.
    KanteSettingRow {
        visible: ctl.connection === "midi" && (choice.midi.ports.length > 1 || choice.midi.missingText !== "")
        Layout.fillWidth: true
        title: texts.t("settings.midi.device")
        hint: choice.midi.missingText
        KanteComboBox {
            visible: choice.midi.ports.length > 0
            Accessible.name: texts.t("settings.midi.device")
            model: choice.midi.ports.map(function(p) { return p.name })
            currentIndex: choice.midi.ports.findIndex(function(p) { return p.chosen })
            onActivated: function(index) { ctl.chooseMidi(choice.midi.ports[index].name) }
        }
    }
}
