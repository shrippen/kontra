import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Mixer (LH-WIE-03, LH-MOD-05): volumes of backing track, metronome and reference tone, and the
// bass played through Kontra (off by default: the interface's direct monitoring has no delay).
// Changes apply at once, also while playing. On the practice page and in the settings.
ColumnLayout {
    id: mixer

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts

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
    KanteSettingRow {
        Layout.fillWidth: true
        title: mixer.texts.t("mixer.monitor")
        hint: mixer.texts.t("mixer.monitor.hint")
        QQC2.Switch {
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
}
