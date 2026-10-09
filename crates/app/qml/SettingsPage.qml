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
    Component.onCompleted: {
        ctl.refreshDevices()
        // QA `--mixer`: scroll to the mixer once the layout is done.
        if (ctl.showMixer) {
            Qt.callLater(function() { page.flickable.contentY = mixerLabel.y })
        }
    }

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
            hint: page.inputs.length === 0 ? texts.t("settings.input.none") : ""
            QQC2.ComboBox {
                Accessible.name: texts.t("settings.input")
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(280)
                model: [texts.t("settings.device.default")].concat(page.inputs.map(function(d) { return d.name }))
                currentIndex: page.deviceIndex(page.inputs, ctl.inputDevice)
                onActivated: function(index) { ctl.chooseInput(index === 0 ? "" : page.inputs[index - 1].id) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.output")
            QQC2.ComboBox {
                Accessible.name: texts.t("settings.output")
                Layout.fillWidth: true
                Layout.preferredWidth: KanteStyle.unit(280)
                model: [texts.t("settings.device.default")].concat(page.outputs.map(function(d) { return d.name }))
                currentIndex: page.deviceIndex(page.outputs, ctl.outputDevice)
                onActivated: function(index) { ctl.chooseOutput(index === 0 ? "" : page.outputs[index - 1].id) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.connection")
            hint: ctl.laptopJack ? texts.t("hint.laptop") : ""
            KanteSegmented {
                Accessible.name: texts.t("settings.connection")
                model: [texts.t("settings.source.line"), texts.t("settings.source.laptop")]
                currentIndex: ctl.laptopJack ? 1 : 0
                onActivated: function(index) { ctl.changeInstrument(ctl.strings, ctl.fretless, index === 1) }
            }
        }
        // LH-EIN-10: inputs that come later are named, not left out (QA P5 Aylin, P6 Karl).
        KanteCallout {
            Layout.fillWidth: true
            kind: KanteCallout.Kind.Info
            autoHide: false
            text: texts.t("settings.later")
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.strings")
            KanteSegmented {
                Accessible.name: texts.t("settings.strings")
                model: ["4", "5", "6"]
                currentIndex: ctl.strings - 4
                onActivated: function(index) { ctl.changeInstrument(index + 4, ctl.fretless, ctl.laptopJack) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.fretless")
            QQC2.Switch {
                Accessible.name: texts.t("settings.fretless")
                checked: ctl.fretless
                onToggled: ctl.changeInstrument(ctl.strings, checked, ctl.laptopJack)
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.lowtop")
            hint: texts.t("settings.lowtop.hint")
            QQC2.Switch {
                Accessible.name: texts.t("settings.lowtop")
                checked: ctl.lowStringOnTop
                onToggled: ctl.changeOptions(ctl.metronome, ctl.reference, checked)
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.playmarks")
            hint: texts.t("settings.playmarks.hint")
            KanteSegmented {
                Accessible.name: texts.t("settings.playmarks")
                readonly property var values: ["quiet", "off", "full"]
                model: [texts.t("settings.playmarks.quiet"), texts.t("settings.playmarks.off"),
                        texts.t("settings.playmarks.full")]
                currentIndex: Math.max(0, values.indexOf(ctl.playMarks))
                onActivated: function(index) { ctl.changePlayMarks(values[index]) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.window")
            hint: texts.t("settings.window.hint")
            KanteSegmented {
                Accessible.name: texts.t("settings.window")
                readonly property var values: ["auto", "loose", "normal", "strict"]
                model: [texts.t("settings.auto"), texts.t("settings.window.loose"),
                        texts.t("settings.window.normal"), texts.t("settings.window.strict")]
                currentIndex: Math.max(0, values.indexOf(ctl.strictness))
                onActivated: function(index) { ctl.changeScoring(values[index], ctl.pitchCents) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.cents")
            hint: texts.t("settings.cents.hint")
            KanteSegmented {
                Accessible.name: texts.t("settings.cents")
                readonly property var values: [0, 10, 20, 30, 50]
                model: [texts.t("settings.auto"), "10 ct", "20 ct", "30 ct", "50 ct"]
                currentIndex: Math.max(0, values.indexOf(ctl.pitchCents))
                onActivated: function(index) { ctl.changeScoring(ctl.strictness, values[index]) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("transport.metronome")
            QQC2.Switch {
                Accessible.name: texts.t("transport.metronome")
                checked: ctl.metronome
                onToggled: ctl.changeOptions(checked, ctl.reference, ctl.lowStringOnTop)
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("transport.reference")
            QQC2.Switch {
                Accessible.name: texts.t("transport.reference")
                checked: ctl.reference
                onToggled: ctl.changeOptions(ctl.metronome, checked, ctl.lowStringOnTop)
            }
        }
        KanteSectionLabel { id: mixerLabel; text: texts.t("mixer.title"); Layout.topMargin: KanteStyle.unit(12) }
        Mixer { Layout.fillWidth: true }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.latency")
            hint: texts.t("settings.latency.hint")
            RowLayout {
                QQC2.SpinBox {
                    Accessible.name: texts.t("settings.latency")
                    from: 0
                    to: 500
                    stepSize: 5
                    value: Math.round(ctl.latencyMs)
                    onValueModified: ctl.changeLatency(value)
                }
                KanteButton {
                    text: texts.t("calibration.start")
                    busy: ctl.calibrating
                    enabled: !ctl.calibrating
                    onClicked: ctl.startCalibration()
                }
            }
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.mutedTextColor
            text: ctl.calibrationText !== "" ? ctl.calibrationText : texts.t("calibration.hint")
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.language")
            KanteSegmented {
                Accessible.name: texts.t("settings.language")
                model: ["Deutsch", "English"]
                currentIndex: texts.t("app.name") !== "" && ctl.language === "en" ? 1 : 0
                onActivated: function(index) { ctl.changeLanguage(index === 1 ? "en" : "de") }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.style")
            KanteSegmented {
                Accessible.name: texts.t("settings.style")
                model: ["Kante", "Kante Light", "System"]
                currentIndex: Math.max(0, page.styles.indexOf(ctl.style))
                onActivated: function(index) { ctl.changeStyle(page.styles[index]) }
            }
        }

        // All keyboard shortcuts in one place (LH-NF-07).
        KanteSectionLabel { text: texts.t("keys.title"); Layout.topMargin: KanteStyle.unit(12) }
        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: KanteStyle.unit(16)
            rowSpacing: KanteStyle.unit(4)
            Repeater {
                model: [
                    ["Space / K", "keys.play"], ["Pos1 / Home", "keys.rewind"], ["L", "keys.loop"],
                    ["M", "keys.metronome"], ["C", "keys.countin"], ["X", "keys.mixer"], ["+ / −", "keys.speed"],
                    ["N", "keys.next"], ["1–4", "keys.mode"], ["Alt+1–4", "keys.view"], ["Ctrl+1–6", "keys.pages"]
                ]
                delegate: RowLayout {
                    Layout.columnSpan: 2
                    spacing: KanteStyle.unit(16)
                    QQC2.Label {
                        Layout.preferredWidth: KanteStyle.unit(110)
                        text: modelData[0]
                        font: KanteStyle.monoFont(KanteStyle.defaultFont.pointSize, false)
                        color: KanteStyle.strongTextColor
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        text: texts.t(modelData[1])
                        color: KanteStyle.textColor
                    }
                }
            }
        }
    }
}
