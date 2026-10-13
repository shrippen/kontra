import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Mixer (LH-WIE-03, LH-MOD-05): volumes of backing track, metronome, reference tone and drums
// (LH-LRN-01), and the bass played through Kontra (off by default: the interface's direct
// monitoring has no delay). While it plays, three tone bands and a compressor shape it
// and warmth (LH-EIN-08); "Neutral" resets them.
// Changes apply at once, also while playing. On the practice page and in the settings.
ColumnLayout {
    id: mixer

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    // On the practice page: without the ear training's prompt, which plays only there (QA 13.10.).
    property bool practice: false

    spacing: 0

    // One volume: title, slider and the value in percent (not only the slider's position).
    component Volume: KanteSettingRow {
        id: row
        property string bus
        property real value
        Layout.fillWidth: true
        RowLayout {
            spacing: KanteStyle.unit(8)
            QQC2.Slider {
                id: slider
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(220)
                from: 0
                to: 1
                stepSize: 0.05
                value: row.value
                focusPolicy: Qt.StrongFocus
                Accessible.name: row.title
                onMoved: mixer.ctl.changeVolume(row.bus, value)
                KanteSliderSkin { control: slider }
            }
            QQC2.Label {
                Layout.preferredWidth: KanteStyle.unit(48)
                horizontalAlignment: Text.AlignRight
                color: KanteStyle.textColor
                text: mixer.texts.t("mixer.percent", { percent: Math.round(slider.value * 100) })
            }
        }
    }

    Volume {
        title: mixer.texts.t("mixer.backing")
        bus: "backing"
        value: mixer.ctl.backingVolume
    }
    Volume {
        title: mixer.texts.t("mixer.metronome")
        bus: "metronome"
        value: mixer.ctl.metronomeVolume
    }
    Volume {
        title: mixer.texts.t("mixer.tone")
        bus: "tone"
        value: mixer.ctl.toneVolume
    }
    // One tone band of the monitored bass in whole dB, or compressor and warmth in percent.
    component Shaping: KanteSettingRow {
        id: shapingRow
        property string band
        property real value
        readonly property bool isAmount: band === "compress" || band === "warmth"
        readonly property real range: mixer.ctl.shapingRangeDb
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        RowLayout {
            spacing: KanteStyle.unit(8)
            QQC2.Slider {
                id: shapingSlider
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(220)
                from: shapingRow.isAmount ? 0 : -shapingRow.range
                to: shapingRow.isAmount ? 1 : shapingRow.range
                stepSize: shapingRow.isAmount ? 0.05 : 1
                value: shapingRow.value
                focusPolicy: Qt.StrongFocus
                Accessible.name: shapingRow.title
                onMoved: mixer.ctl.changeShaping(shapingRow.band, value)
                KanteSliderSkin { control: shapingSlider }
            }
            QQC2.Label {
                Layout.preferredWidth: KanteStyle.unit(48)
                horizontalAlignment: Text.AlignRight
                color: KanteStyle.textColor
                text: shapingRow.isAmount
                    ? (shapingSlider.value === 0 ? mixer.texts.t("mixer.shaping.off")
                        : mixer.texts.t("mixer.percent", { percent: Math.round(shapingSlider.value * 100) }))
                    : mixer.texts.t("mixer.shaping.db", { db: (shapingSlider.value > 0 ? "+" : "") + Math.round(shapingSlider.value) })
            }
        }
    }

    Volume {
        title: mixer.texts.t("mixer.drums")
        bus: "drums"
        value: mixer.ctl.drumsVolume
    }
    Volume {
        visible: !mixer.practice
        title: mixer.texts.t("mixer.prompt")
        bus: "prompt"
        value: mixer.ctl.promptVolume
    }
    KanteSettingRow {
        Layout.fillWidth: true
        title: mixer.texts.t("mixer.monitor")
        hint: mixer.texts.t("mixer.monitor.hint")
        KanteSwitch {
            Accessible.name: mixer.texts.t("mixer.monitor")
            Accessible.description: mixer.texts.t("mixer.monitor.hint")
            focusPolicy: Qt.StrongFocus
            checked: mixer.ctl.monitor
            onToggled: mixer.ctl.changeMonitor(checked)
        }
    }
    Volume {
        visible: mixer.ctl.monitor
        title: mixer.texts.t("mixer.monitor.volume")
        bus: "monitor"
        value: mixer.ctl.monitorVolume
    }
    Shaping {
        section: mixer.texts.t("mixer.shaping")
        title: mixer.texts.t("mixer.shaping.low")
        hint: mixer.texts.t("mixer.shaping.hint")
        band: "low"
        value: mixer.ctl.monitorLowDb
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.mid")
        band: "mid"
        value: mixer.ctl.monitorMidDb
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.high")
        band: "high"
        value: mixer.ctl.monitorHighDb
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.compress")
        hint: mixer.texts.t("mixer.shaping.compress.hint")
        band: "compress"
        value: mixer.ctl.monitorCompress
    }
    Shaping {
        title: mixer.texts.t("mixer.shaping.warmth")
        hint: mixer.texts.t("mixer.shaping.warmth.hint")
        band: "warmth"
        value: mixer.ctl.monitorWarmth
    }
    KanteSettingRow {
        Layout.fillWidth: true
        visible: mixer.ctl.monitor
        title: mixer.texts.t("mixer.shaping.neutral.title")
        KanteButton {
            text: mixer.texts.t("mixer.shaping.neutral")
            Accessible.name: mixer.texts.t("mixer.shaping.neutral.title")
            enabled: mixer.ctl.monitorLowDb !== 0 || mixer.ctl.monitorMidDb !== 0
                || mixer.ctl.monitorHighDb !== 0 || mixer.ctl.monitorCompress !== 0
                || mixer.ctl.monitorWarmth !== 0
            onClicked: mixer.ctl.resetShaping()
        }
    }
}
