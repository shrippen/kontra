import QtQuick
import QtCore
import QtQuick.Layouts
import QtQuick.Dialogs
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
    // LH-FOR-03: how a loaded backup is taken over.
    property string dataHow: "merge"

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("nav.settings")
    Component.onCompleted: {
        ctl.refreshDevices()
        // QA `--mixer`: scroll to the mixer once the layout is done.
        if (ctl.showMixer) {
            Qt.callLater(function() { page.flickable.contentY = mixerLabel.y })
        }
        sectionTimer.running = ctl.startSection !== ""
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
                model: [page.autoInput].concat(page.inputs.map(function(d) { return d.name }))
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
            InfoTip { term: "Hi-Z"; info: texts.t("help.hiz") }
        }
        // LH-EIN-10: inputs that come later are entries of the choice, not left out (QA P5, P6).
        ConnectionChoice { Layout.fillWidth: true }
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
            title: texts.t("settings.reference")
            hint: texts.t("settings.reference.hint")
            QQC2.SpinBox {
                Accessible.name: texts.t("settings.reference")
                from: 430
                to: 450
                value: Math.round(ctl.a4Hz)
                editable: true
                textFromValue: function(value) { return value + " Hz" }
                valueFromText: function(text) { return parseInt(text) }
                onValueModified: ctl.changeReference(value)
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
            title: texts.t("settings.lefthanded")
            hint: texts.t("settings.lefthanded.hint")
            QQC2.Switch {
                Accessible.name: texts.t("settings.lefthanded")
                checked: ctl.leftHanded
                onToggled: ctl.changeLeftHanded(checked)
            }
        }
        KanteSettingRow {
            id: colorsRow
            Layout.fillWidth: true
            title: texts.t("settings.colors")
            hint: texts.t("settings.colors.hint")
            KanteSegmented {
                Accessible.name: texts.t("settings.colors")
                readonly property var values: ["kante", "mono", "bright"]
                model: [texts.t("settings.colors.kante"), texts.t("settings.colors.mono"),
                        texts.t("settings.colors.bright")]
                currentIndex: Math.max(0, values.indexOf(ctl.stringColors))
                onActivated: function(index) { ctl.changeStringColors(values[index]) }
            }
        }
        // Preview of the string colours with a hit, a wrong and a waiting note.
        KanteTabLane {
            Layout.fillWidth: true
            Layout.preferredHeight: KanteStyle.unit(24) * Math.max(4, ctl.strings)
            strings: ctl.strings
            stringNames: JSON.parse(ctl.profileTuningJson).map(function(s) { return s.name })
            lowStringOnTop: ctl.lowStringOnTop
            stringColors: applicationWindow().stringColors
            barNumbers: false
            playLine: 0.05
            pixelsPerSecond: KanteStyle.unit(90)
            notes: [{ time: 0.5, duration: 0.4, string: 0, fret: 3, state: "hit" },
                    { time: 1.5, duration: 0.4, string: 1, fret: 5, state: "wrong" },
                    { time: 2.5, duration: 0.4, string: 2, fret: 2, state: "pending" },
                    { time: 3.5, duration: 0.4, string: 3, fret: 0, state: "pending" },
                    { time: 4.5, duration: 0.4, string: 4, fret: 7, state: "pending" },
                    { time: 5.5, duration: 0.4, string: 5, fret: 1, state: "pending" }]
                .filter(function(n) { return n.string < ctl.strings })
            accessibleName: texts.t("settings.colors.preview")
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
            title: texts.t("settings.motion")
            hint: texts.t("settings.motion.hint")
            KanteSegmented {
                Accessible.name: texts.t("settings.motion")
                readonly property var values: ["system", "reduce", "full"]
                model: [texts.t("settings.motion.system"), texts.t("settings.motion.reduce"),
                        texts.t("settings.motion.full")]
                currentIndex: Math.max(0, values.indexOf(ctl.motion))
                onActivated: function(index) { ctl.changeMotion(values[index]) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.window")
            hint: texts.t("settings.window.hint")
            ColumnLayout {
                KanteSegmented {
                    Accessible.name: texts.t("settings.window")
                    readonly property var values: ["auto", "loose", "normal", "strict", "custom"]
                    model: [texts.t("settings.auto"), texts.t("settings.window.loose"),
                            texts.t("settings.window.normal"), texts.t("settings.window.strict"),
                            texts.t("settings.window.custom")]
                    currentIndex: Math.max(0, values.indexOf(ctl.strictness))
                    onActivated: function(index) { ctl.changeScoring(values[index], ctl.pitchCents) }
                }
                // Free window in ms (LH-BEW-02), ± around the beat.
                QQC2.SpinBox {
                    visible: ctl.strictness === "custom"
                    Accessible.name: texts.t("settings.window.ms")
                    from: 20
                    to: 150
                    stepSize: 5
                    editable: true
                    value: Math.round(ctl.windowMs)
                    textFromValue: function(value) { return "± " + value + " ms" }
                    valueFromText: function(text) { return parseInt(text.replace(/[^0-9]/g, "")) }
                    onValueModified: ctl.changeWindow(value)
                }
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
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("settings.adaptive")
            hint: texts.t("settings.adaptive.hint")
            QQC2.Switch {
                Accessible.name: texts.t("settings.adaptive")
                checked: ctl.adaptive
                onToggled: ctl.changeAdaptive(checked)
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
                InfoTip { term: texts.t("settings.latency"); info: texts.t("help.latency") }
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

        // LH-FOR-03: all user data as one open JSON file (backup, second computer).
        KanteSectionLabel { id: dataLabel; text: texts.t("data.title"); Layout.topMargin: KanteStyle.unit(12) }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.mutedTextColor
            text: texts.t("data.hint")
        }
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("data.import.how")
            hint: page.dataHow === "replace" ? texts.t("data.replace.hint") : texts.t("data.merge.hint")
            KanteSegmented {
                Accessible.name: texts.t("data.import.how")
                readonly property var values: ["merge", "replace"]
                model: [texts.t("data.merge"), texts.t("data.replace")]
                currentIndex: Math.max(0, values.indexOf(page.dataHow))
                onActivated: function(index) { page.dataHow = values[index] }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            KanteButton {
                text: texts.t("data.export")
                Accessible.name: texts.t("data.export")
                onClicked: exportDialog.open()
            }
            KanteButton {
                text: texts.t("data.import")
                Accessible.name: texts.t("data.import")
                onClicked: importDialog.open()
            }
        }

        // LH-NF-04: audio dropouts since the start, for bug reports.
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.mutedTextColor
            font: KanteStyle.monoFont(KanteStyle.defaultFont.pointSize, false)
            text: texts.t("settings.xruns", { n: ctl.xruns })
            Accessible.name: text
        }

        // LH-NF-11: Kontra's licence and every library inside it.
        KanteSectionLabel { id: aboutLabel; text: texts.t("about.title"); Layout.topMargin: KanteStyle.unit(12) }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t("about.text")
        }
        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: KanteStyle.unit(16)
            rowSpacing: KanteStyle.unit(2)
            Accessible.name: texts.t("about.title")
            Repeater {
                model: JSON.parse(ctl.licencesJson())
                delegate: RowLayout {
                    required property var modelData
                    Layout.columnSpan: 2
                    spacing: KanteStyle.unit(16)
                    QQC2.Label {
                        Layout.preferredWidth: KanteStyle.unit(220)
                        elide: Text.ElideRight
                        text: modelData.name + (modelData.version !== "" ? " " + modelData.version : "")
                        font: KanteStyle.monoFont(KanteStyle.defaultFont.pointSize, false)
                        color: KanteStyle.strongTextColor
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        text: modelData.license
                        color: KanteStyle.textColor
                    }
                }
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
                    ["Space / K / PgDn", "keys.play"], ["Pos1 / Home / PgUp", "keys.rewind"], ["L", "keys.loop"],
                    ["[ / ,", "keys.loopstart"], ["] / .", "keys.loopend"],
                    ["M", "keys.metronome"], ["C", "keys.countin"], ["X", "keys.mixer"], ["+ / −", "keys.speed"],
                    ["N", "keys.next"], ["1–4", "keys.mode"], ["Alt+1–4", "keys.view"], ["Ctrl+1–6", "keys.pages"]
                ]
                delegate: RowLayout {
                    Layout.columnSpan: 2
                    spacing: KanteStyle.unit(16)
                    QQC2.Label {
                        Layout.preferredWidth: KanteStyle.unit(150)
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
        // Foot switch (LH-MOD-07): any pedal that types keys works, no driver needed.
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: texts.t("keys.footswitch")
            color: KanteStyle.mutedTextColor
        }
    }

    // QA `--section data|about`: screenshots of the lower sections, once the layout settled.
    Timer {
        id: sectionTimer
        interval: 500
        onTriggered: {
            var target = ({ data: dataLabel, about: aboutLabel })[ctl.startSection]
            if (target) {
                page.flickable.contentY = target.mapToItem(page.flickable.contentItem, 0, 0).y
            }
        }
    }

    FileDialog {
        id: exportDialog
        fileMode: FileDialog.SaveFile
        defaultSuffix: "json"
        currentFile: StandardPaths.writableLocation(StandardPaths.DocumentsLocation) + "/" + "kontra-daten.json"
        nameFilters: [texts.t("data.files") + " (*.json)"]
        onAccepted: ctl.exportData(selectedFile.toString())
    }
    FileDialog {
        id: importDialog
        nameFilters: [texts.t("data.files") + " (*.json)"]
        onAccepted: ctl.importData(selectedFile.toString(), page.dataHow)
    }
}
