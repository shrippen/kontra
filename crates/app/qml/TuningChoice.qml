import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Tuning of the profile (LH-TUN-02): presets and a custom tuning with one note per string.
// Used by the tuner and for the tab of files without their own tuning.
ColumnLayout {
    id: choice

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var values: ["standard", "drop", "half-down", "whole-down", "custom"]
    readonly property var strings: JSON.parse(ctl.profileTuningJson)
    readonly property var notes: JSON.parse(ctl.noteChoicesJson)

    spacing: KanteStyle.unit(8)

    function noteIndex(midi) {
        for (var i = 0; i < notes.length; i++) {
            if (notes[i].midi === midi) {
                return i
            }
        }
        return -1
    }

    KanteSegmented {
        Accessible.name: texts.t("settings.tuning")
        // Drop D on four strings; on five and six the low B drops to A.
        model: [texts.t("settings.tuning.standard"),
                texts.t(ctl.strings === 4 ? "settings.tuning.dropd" : "settings.tuning.dropa"),
                texts.t("settings.tuning.half"), texts.t("settings.tuning.whole"),
                texts.t("settings.tuning.custom")]
        currentIndex: Math.max(0, choice.values.indexOf(ctl.tuning))
        onActivated: function(index) { ctl.changeTuning(choice.values[index]) }
    }

    // The open strings, lowest first; editable in the custom tuning.
    RowLayout {
        spacing: KanteStyle.unit(8)
        Repeater {
            model: choice.strings
            delegate: QQC2.ComboBox {
                required property var modelData
                required property int index
                Accessible.name: texts.t("settings.tuning.string", { n: choice.strings.length - index })
                Layout.preferredWidth: KanteStyle.unit(80)
                enabled: ctl.tuning === "custom"
                model: choice.notes.map(function(n) { return n.name })
                currentIndex: choice.noteIndex(modelData.midi)
                displayText: modelData.full
                onActivated: function(i) { ctl.changeCustomString(index, choice.notes[i].midi) }
            }
        }
    }
}
