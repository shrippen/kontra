import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// First-start assistant (LH-EIN-01, journey J-A1): instrument, connection, level and tuning,
// optional latency, then straight into the first exercise. Every step can be skipped.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var inputs: JSON.parse(ctl.inputDevicesJson)
    readonly property var steps: ["instrument", "connection", "level", "latency", "done"]
    property int step: 0

    title: texts.t("setup.title")
    background: Rectangle { color: KanteStyle.backgroundColor }

    Component.onCompleted: ctl.refreshDevices()
    onStepChanged: {
        // Level and tuning listen to the input; other steps leave it alone.
        if (steps[step] === "level") {
            ctl.startTuner()
        } else if (ctl.tunerActive) {
            ctl.stopTuner()
        }
    }

    // LH-EIN-02: what "automatic" means right now (an interface before the built-in card).
    readonly property string autoInput: {
        for (var i = 0; i < inputs.length; i++) {
            if (inputs[i].preferred) {
                return texts.t("settings.device.auto", { name: inputs[i].name })
            }
        }
        return texts.t("settings.device.default")
    }

    function next() { step = Math.min(step + 1, steps.length - 1) }

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("setup.title"); pageTitle: true }
        KanteSteps {
            Layout.fillWidth: true
            model: page.steps.map(function(s) { return texts.t("setup.step." + s) })
            current: page.step
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.15
            text: texts.t("setup." + page.steps[page.step] + ".hint")
            color: KanteStyle.textColor
        }

        // 1 Instrument
        ColumnLayout {
            visible: page.steps[page.step] === "instrument"
            Layout.fillWidth: true
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
                title: texts.t("settings.tuning")
                hint: texts.t("settings.tuning.hint")
                TuningChoice {}
            }
            KanteSettingRow {
                Layout.fillWidth: true
                title: texts.t("settings.fretless")
                KanteSwitch {
                    Accessible.name: texts.t("settings.fretless")
                    checked: ctl.fretless
                    onToggled: ctl.changeInstrument(ctl.strings, checked, ctl.laptopJack)
                }
            }
        }

        // 2 Connection
        ColumnLayout {
            visible: page.steps[page.step] === "connection"
            Layout.fillWidth: true
            RowLayout {
                KanteSectionLabel { text: texts.t("settings.connection") }
                InfoTip { term: "Hi-Z"; info: texts.t("help.hiz") }
            }
            // LH-EIN-10: MIDI and microphone are entries marked as coming later.
            ConnectionChoice { Layout.fillWidth: true }
            KanteCallout {
                Layout.fillWidth: true
                visible: ctl.laptopJack
                kind: KanteCallout.Kind.Info
                text: texts.t("hint.laptop")
                autoHide: false
            }
            KanteSettingRow {
                Layout.fillWidth: true
                title: texts.t("settings.input")
                KanteComboBox {
                    Accessible.name: texts.t("settings.input")
                    Layout.fillWidth: true
                    Layout.preferredWidth: KanteStyle.unit(280)
                    model: [page.autoInput].concat(page.inputs.map(function(d) { return d.name }))
                    currentIndex: {
                        for (var i = 0; i < page.inputs.length; i++) {
                            if (page.inputs[i].id === ctl.inputDevice) {
                                return i + 1
                            }
                        }
                        return 0
                    }
                    onActivated: function(index) { ctl.chooseInput(index === 0 ? "" : page.inputs[index - 1].id) }
                }
            }
        }

        // 3 Level and tuning
        ColumnLayout {
            visible: page.steps[page.step] === "level"
            Layout.fillWidth: true
            KanteLevelMeter {
                id: meter
                Layout.fillWidth: true
                peakDb: ctl.peakDb
                rmsDb: ctl.rmsDb
                clipped: ctl.clipped
                title: texts.t("level.title")
                quietText: texts.t("level.quiet")
                goodText: texts.t("level.good")
                loudText: texts.t("level.loud")
                clipText: texts.t("level.clip")
                accessibleName: texts.t("level.name")
            }
            LevelAdvice {
                Layout.fillWidth: true
                peakDb: ctl.peakDb
                clipped: ctl.clipped
                listening: page.steps[page.step] === "level"
                laptopJack: ctl.laptopJack
                onUseLaptopJack: ctl.changeInstrument(ctl.strings, ctl.fretless, true)
            }
            KanteTunerGauge {
                Layout.fillWidth: true
                Layout.preferredHeight: KanteStyle.unit(260)
                noteName: ctl.tunerNote.replace(/-?[0-9]+$/, "")
                octave: ctl.tunerNote !== "" ? Number(ctl.tunerNote.match(/-?[0-9]+$/)) : 0
                cents: ctl.tunerCents
                active: ctl.tunerNote !== ""
                hint: ctl.tunerNote === "" ? texts.t("tuner.silent")
                    : ctl.tunerCents < -3 ? texts.t("tuner.up")
                    : ctl.tunerCents > 3 ? texts.t("tuner.down") : texts.t("tuner.ok")
                inTuneText: texts.t("tuner.ok")
                accessibleName: texts.t("tuner.name")
            }
        }

        // 4 Latency
        ColumnLayout {
            visible: page.steps[page.step] === "latency"
            Layout.fillWidth: true
            RowLayout {
                Layout.fillWidth: true
                QQC2.Label { text: texts.t("calibration.hint"); wrapMode: Text.WordWrap; Layout.fillWidth: true; color: KanteStyle.textColor }
                InfoTip { term: texts.t("settings.latency"); info: texts.t("help.latency") }
            }
            RowLayout {
                KanteButton {
                    text: texts.t("calibration.start")
                    busy: ctl.calibrating
                    enabled: !ctl.calibrating
                    onClicked: ctl.startCalibration()
                }
                QQC2.Label { text: Math.round(ctl.latencyMs) + " ms"; color: KanteStyle.textColor }
            }
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                visible: ctl.calibrationText !== ""
                text: ctl.calibrationText
                color: KanteStyle.mutedTextColor
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(10)
            KanteButton {
                text: texts.t("setup.back")
                emphasis: KanteButton.Emphasis.Quiet
                visible: page.step > 0
                onClicked: page.step -= 1
            }
            Item { Layout.fillWidth: true }
            KanteButton {
                text: texts.t("setup.skip")
                emphasis: KanteButton.Emphasis.Quiet
                visible: page.step < page.steps.length - 1
                onClicked: page.next()
            }
            KanteButton {
                text: texts.t("setup.next")
                emphasis: KanteButton.Emphasis.Primary
                visible: page.step < page.steps.length - 1
                onClicked: page.next()
            }
            KanteButton {
                text: texts.t("setup.later")
                visible: page.step === page.steps.length - 1
                onClicked: {
                    ctl.finishSetup()
                    applicationWindow().show("library")
                }
            }
            KanteButton {
                text: texts.t("setup.finish")
                emphasis: KanteButton.Emphasis.Primary
                visible: page.step === page.steps.length - 1
                onClicked: {
                    ctl.finishSetup()
                    if (ctl.openDemo()) {
                        applicationWindow().show("practice")
                    }
                }
            }
        }
    }
}
