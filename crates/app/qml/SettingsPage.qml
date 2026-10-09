import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Settings (LH-EIN, LH-ALL-01/02): input, output, instrument, practice, language, style.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var inputs: JSON.parse(ctl.inputDevicesJson)
    readonly property var outputs: JSON.parse(ctl.outputDevicesJson)
    readonly property var styles: ["kante", "kante-light", "system"]

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("nav.settings")
    Component.onCompleted: ctl.refreshDevices()

    function deviceIndex(list, id) {
        for (var i = 0; i < list.length; i++) {
            if (list[i].id === id) {
                return i + 1
            }
        }
        return 0
    }

    ColumnLayout {
        spacing: KanteStyle.unit(4)

        KanteHeading { text: texts.t("nav.settings"); pageTitle: true }

        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.input")
            QQC2.ComboBox {
                Layout.fillWidth: true
                model: ["—"].concat(page.inputs.map(function(d) { return d.name }))
                currentIndex: page.deviceIndex(page.inputs, ctl.inputDevice)
                onActivated: function(index) { ctl.chooseInput(index === 0 ? "" : page.inputs[index - 1].id) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.output")
            QQC2.ComboBox {
                Layout.fillWidth: true
                model: ["—"].concat(page.outputs.map(function(d) { return d.name }))
                currentIndex: page.deviceIndex(page.outputs, ctl.outputDevice)
                onActivated: function(index) { ctl.chooseOutput(index === 0 ? "" : page.outputs[index - 1].id) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.connection")
            hint: ctl.laptopJack ? texts.t("hint.laptop") : ""
            KanteSegmented {
                model: [texts.t("settings.source.line"), texts.t("settings.source.laptop")]
                currentIndex: ctl.laptopJack ? 1 : 0
                onActivated: function(index) { ctl.changeInstrument(ctl.strings, ctl.fretless, index === 1) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.strings")
            KanteSegmented {
                model: ["4", "5", "6"]
                currentIndex: ctl.strings - 4
                onActivated: function(index) { ctl.changeInstrument(index + 4, ctl.fretless, ctl.laptopJack) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.fretless")
            QQC2.Switch {
                checked: ctl.fretless
                onToggled: ctl.changeInstrument(ctl.strings, checked, ctl.laptopJack)
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("transport.metronome")
            QQC2.Switch {
                checked: ctl.metronome
                onToggled: ctl.changeOptions(checked, ctl.reference, ctl.lowStringOnTop)
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("transport.reference")
            QQC2.Switch {
                checked: ctl.reference
                onToggled: ctl.changeOptions(ctl.metronome, checked, ctl.lowStringOnTop)
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.latency")
            hint: texts.t("settings.latency.hint")
            QQC2.SpinBox {
                from: 0
                to: 500
                stepSize: 5
                value: Math.round(ctl.latencyMs)
                onValueModified: ctl.changeLatency(value)
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.language")
            KanteSegmented {
                model: ["Deutsch", "English"]
                currentIndex: texts.t("app.name") !== "" && ctl.language === "en" ? 1 : 0
                onActivated: function(index) { ctl.changeLanguage(index === 1 ? "en" : "de") }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.style")
            KanteSegmented {
                model: ["Kante", "Kante Light", "System"]
                currentIndex: Math.max(0, page.styles.indexOf(ctl.style))
                onActivated: function(index) { ctl.changeStyle(page.styles[index]) }
            }
        }
    }
}
