import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// First-start assistant (LH-EIN-01, journey J-A1): instrument, connection, level and tuning,
// optional latency, then straight into the first exercise. Every step can be skipped.
Kirigami.ScrollablePage {
    id: page

    // Tab never leaves the focus below the edge (Kante 1.49, QA 27.10. S2).
    readonly property KanteFocusFollow focusFollow: KanteFocusFollow { flickable: page.flickable }

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var inputs: JSON.parse(ctl.inputDevicesJson)
    readonly property var steps: ["instrument", "connection", "level", "latency", "done"]
    property int step: 0
    // Steps left out with "skip": a dash, not the done check (Kante 1.49, QA 27.10. L4).
    property var skippedSteps: []
    // The player's answer on the last step (LH-EIN-13): "new", "player" or "songs".
    property string level: ""
    readonly property var levels: ["new", "player", "songs"]

    // Where the setup leads (QA 27.10., L5: everybody landed in "Groove Viertel").
    //   new     first lesson, simple practice page
    //   player  the exercises, the basics level open
    //   songs   the library
    function finish() {
        ctl.finishSetup()
        var app = applicationWindow()
        if (level === "new") {
            ctl.changeSimpleView(true)
            var next = JSON.parse(ctl.learnJson()).next
            if (!next) {
                app.show("learn")
                return
            }
            app.lessonId = next.id
            app.lessonPage = 0
            app.show("lesson")
            return
        }
        if (level === "player") {
            app.exerciseLevel = 1
            app.show("exercises")
            return
        }
        app.show("library")
    }

    title: texts.t("setup.title")
    background: Rectangle { color: KanteStyle.backgroundColor }

    Component.onCompleted: ctl.refreshDevices()
    onStepChanged: {
        // Each step starts at its top (QA 27.10.: at 150 % the next step opened scrolled down).
        flickable.contentY = 0
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

    function next() {
        skippedSteps = skippedSteps.filter(function(i) { return i !== step })
        step = Math.min(step + 1, steps.length - 1)
    }

    function skip() {
        if (skippedSteps.indexOf(step) < 0) {
            skippedSteps = skippedSteps.concat([step])
        }
        step = Math.min(step + 1, steps.length - 1)
    }

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("setup.title"); pageTitle: true }
        KanteSteps {
            Layout.fillWidth: true
            model: page.steps.map(function(s) { return texts.t("setup.step." + s) })
            current: page.step
            skipped: page.skippedSteps
            stateNames: ({ done: texts.t("setup.state.done"), current: texts.t("setup.state.current"),
                           upcoming: texts.t("setup.state.upcoming"), skipped: texts.t("setup.state.skipped") })
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.15
            text: texts.t("setup." + page.steps[page.step] + ".hint")
            color: KanteStyle.textColor
        }

        // 1 Instrument
        InstrumentChoice {
            visible: page.steps[page.step] === "instrument"
            Layout.fillWidth: true
        }

        // 2 Connection
        ColumnLayout {
            visible: page.steps[page.step] === "connection"
            Layout.fillWidth: true
            RowLayout {
                KanteSectionLabel { text: texts.t("settings.connection") }
                KanteInfoTip { term: "Hi-Z"; info: texts.t("help.hiz"); accessibleName: texts.t("help.about", { term: "Hi-Z" }) }
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
                    // The setting row sizes its control by the control, not by Layout (names were cut).
                    width: KanteStyle.unit(280)
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
                listening: page.steps[page.step] === "level"
                onUseLaptopJack: ctl.changeConnection("laptop")
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
                KanteInfoTip { term: texts.t("settings.latency"); info: texts.t("help.latency"); accessibleName: texts.t("help.about", { term: texts.t("settings.latency") }) }
            }
            RowLayout {
                KanteButton {
                    text: texts.t("calibration.start")
                    busy: ctl.calibrating
                    enabled: !ctl.calibrating
                    onClicked: ctl.startCalibration()
                }
                // "0 ms" before any measurement read like a result (QA 14.10.).
                QQC2.Label {
                    text: ctl.latencyMs > 0 ? Math.round(ctl.latencyMs) + " ms" : texts.t("calibration.none")
                    color: ctl.latencyMs > 0 ? KanteStyle.textColor : KanteStyle.mutedTextColor
                }
            }
            CalibrationBeat { Layout.fillWidth: true }
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                visible: ctl.calibrationText !== ""
                text: ctl.calibrationText
                color: KanteStyle.mutedTextColor
            }
        }

        // 5 Done: where to go first (LH-EIN-13).
        ColumnLayout {
            visible: page.steps[page.step] === "done"
            Layout.fillWidth: true
            spacing: 0
            Accessible.role: Accessible.List
            Accessible.name: texts.t("setup.done.question")
            KanteSectionLabel { text: texts.t("setup.done.question") }
            Repeater {
                model: page.levels
                delegate: KanteListRow {
                    required property string modelData
                    Layout.fillWidth: true
                    selected: page.level === modelData
                    text: texts.t("setup.done." + modelData)
                    subtitle: texts.t("setup.done." + modelData + ".hint")
                    onClicked: page.level = modelData
                }
            }
        }
    }

    // Back, skip and next stay in one place on every step (QA 27.10.: they moved with each step
    // and with the level advice; a click aimed at "next" hit empty space).
    footer: QQC2.ToolBar {
        background: Rectangle { color: KanteStyle.backgroundColor }
        // Aligned with the page content.
        leftPadding: page.leftPadding
        rightPadding: page.rightPadding
        topPadding: KanteStyle.unit(12)
        bottomPadding: KanteStyle.unit(12)
        contentItem: RowLayout {
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
                onClicked: page.skip()
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
                    applicationWindow().show("home")
                }
            }
            KanteButton {
                text: texts.t("setup.go")
                emphasis: KanteButton.Emphasis.Primary
                visible: page.step === page.steps.length - 1
                enabled: page.level !== ""
                onClicked: page.finish()
            }
        }
    }
}
