import QtQuick
import QtCore
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami
import Kante

// Settings (LH-EIN, LH-ALL-01/02): input, output, instrument, practice, language, style.
// A sticky, shrinking header (title + a scrollspy tab per category, KanteStickyHeader) sits
// over the scrolling content; each category is a KanteSettingGroup tile (design A, Kontra
// round 21, owner 13.11.2026).
Kirigami.Page {
    id: page

    // Kirigami.Page already has a `flickable` property (for its own keyboard navigation);
    // this is ours, not auto-managed (unlike Kirigami.ScrollablePage's).
    flickable: flick

    // Tab never leaves the focus below the sticky header (Kante 1.49, QA 27.10. S2).
    readonly property KanteFocusFollow focusFollow: KanteFocusFollow {
        flickable: page.flickable
        topInset: header.height
    }

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var licences: JSON.parse(ctl.licencesJson())
    // Kontra, Qt and KDE Frameworks: the first entries of the licence list.
    readonly property int mainLicences: 3
    readonly property var inputs: JSON.parse(ctl.inputDevicesJson)
    readonly property var outputs: JSON.parse(ctl.outputDevicesJson)
    readonly property var styles: ["kante", "kante-light", "system"]
    // LH-FOR-03: how a loaded backup is taken over.
    property string dataHow: "merge"
    property var updateState: JSON.parse(ctl.updateNotice("poll"))
    // Round 16 (P3, P4): prepared backing tracks and separated songs, by song.
    property var caches: JSON.parse(ctl.cachesJson())
    readonly property var cacheGroups: [
        { id: "backing", title: texts.t("cache.backing"), hint: "cache.backing.hint" },
        { id: "stems", title: texts.t("cache.stems"), hint: "cache.stems.hint" }
    ]
    function refreshCaches() { caches = JSON.parse(ctl.cachesJson()) }
    // Preparing or separating ends while the page is open: the list follows.
    Connections {
        target: ctl
        function onBackingLoadingChanged() { page.refreshCaches() }
        function onStemsJsonChanged() { page.refreshCaches() }
    }

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("nav.settings")
    leftPadding: applicationWindow().readingPad(width)
    rightPadding: leftPadding

    // The Flickable (and its scrollbar) stay full-bleed, like Kirigami.ScrollablePage's own
    // ScrollView: leftPadding/rightPadding (the shared reading width) margin the content
    // column instead, not the Flickable itself, so the scrollbar sits at the page's true right
    // edge, never over a tile or the header (review round 22: it sat inside the reading width,
    // over the tiles' border and the "Starten" button).
    contentItem: Item {
        anchors.fill: parent
    }

    Component.onCompleted: {
        ctl.refreshDevices()
        // QA `--mixer`: scroll to the mixer once the layout is done.
        if (ctl.showMixer) {
            Qt.callLater(function() { header.scrollTo(page.sectionIndex("mixer")) })
        }
        sectionTimer.running = ctl.startSection !== ""
    }

    // LH-EIN-02: what "automatic" means right now (the input that fits the connection: an
    // interface, or the built-in card for the laptop jack).
    readonly property string autoInput: {
        for (var i = 0; i < inputs.length; i++) {
            if (inputs[i].preferred) {
                return texts.t("settings.device.auto", { name: inputs[i].name })
            }
        }
        return texts.t("settings.device.default")
    }

    // The categories of this page, in order (the sticky header's tabs).
    readonly property var sections: [
        { key: "connection", title: texts.t("settings.section.connection") },
        { key: "instrument", title: texts.t("settings.section.instrument") },
        { key: "display", title: texts.t("settings.section.display") },
        { key: "scoring", title: texts.t("settings.section.scoring") },
        { key: "practice", title: texts.t("settings.section.practice") },
        { key: "mixer", title: texts.t("mixer.title") },
        { key: "language", title: texts.t("settings.section.language") },
        { key: "data", title: texts.t("data.title") },
        { key: "cache", title: texts.t("cache.title") },
        { key: "about", title: texts.t("about.title") },
        { key: "keys", title: texts.t("keys.title") }
    ]
    function sectionIndex(key) {
        for (var i = 0; i < sections.length; i++) {
            if (sections[i].key === key) {
                return i
            }
        }
        return -1
    }

    function deviceIndex(list, id) {
        for (var i = 0; i < list.length; i++) {
            if (list[i].id === id) {
                return i + 1
            }
        }
        return 0
    }

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: col.implicitHeight + KanteStyle.unit(24)
        flickableDirection: Flickable.VerticalFlick
        boundsBehavior: Flickable.StopAtBounds
        // Qt's default deceleration is too high (Kirigami.ScrollablePage works around the same
        // bug, QTBUG-121500); keep its feel now that we build the Flickable ourselves.
        flickDeceleration: 1500
        maximumFlickVelocity: 5000

        ColumnLayout {
            id: col
            x: page.leftPadding
            y: KanteStyle.unit(12)
            width: flick.width - page.leftPadding - page.rightPadding
            spacing: KanteStyle.unit(16)

            // Room for the sticky header above the content.
            Item { Layout.preferredHeight: header.expandedHeight }

            // The setup assistant lives here (and on the start page), not in a category.
            KanteSettingRow {
                Layout.fillWidth: true
                title: texts.t("settings.setup")
                hint: texts.t("settings.setup.hint")
                KanteButton {
                    text: texts.t("settings.setup.start")
                    onClicked: applicationWindow().show("setup")
                }
            }

            KanteSettingGroup {
                id: connGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("connection")].title
                countText: texts.t(count === 1 ? "settings.group.count.one" : "settings.group.count", { n: count })

                // The audio input only matters while the bass comes as audio (not over MIDI).
                KanteSettingRow {
                    Layout.fillWidth: true
                    id: connRow
                    visible: ctl.connection !== "midi"
                    title: texts.t("settings.input")
                    hint: page.inputs.length === 0 ? texts.t("settings.input.none") : ""
                    KanteComboBox {
                        Accessible.name: texts.t("settings.input")
                        // The setting row sizes its control by the control, not by Layout (names were cut).
                        width: KanteStyle.unit(280)
                        model: [page.autoInput].concat(page.inputs.map(function(d) { return d.name }))
                        currentIndex: page.deviceIndex(page.inputs, ctl.inputDevice)
                        onActivated: function(index) { ctl.chooseInput(index === 0 ? "" : page.inputs[index - 1].id) }
                    }
                }
                // LH-EIN-02: what hangs on a USB input without output; an answer holds at once.
                Repeater {
                    model: ctl.connection !== "midi" ? page.inputs.filter(function(d) { return d.usbRole !== "" }) : []
                    delegate: KanteSettingRow {
                        required property var modelData
                        Layout.fillWidth: true
                        title: texts.t("settings.usbrole", { name: modelData.name })
                        KanteSegmented {
                            Accessible.name: texts.t("settings.usbrole", { name: modelData.name })
                            model: [texts.t("usbask.instrument"), texts.t("usbask.microphone")]
                            // "open": no answer yet, neither side chosen.
                            currentIndex: modelData.usbRole === "instrument" ? 0 : modelData.usbRole === "microphone" ? 1 : -1
                            onActivated: function(index) { ctl.setUsbRole(modelData.id, index === 0 ? "instrument" : "microphone") }
                        }
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.output")
                    KanteComboBox {
                        Accessible.name: texts.t("settings.output")
                        // The setting row sizes its control by the control, not by Layout (names were cut).
                        width: KanteStyle.unit(280)
                        model: [texts.t("settings.device.default")].concat(page.outputs.map(function(d) { return d.name }))
                        currentIndex: page.deviceIndex(page.outputs, ctl.outputDevice)
                        onActivated: function(index) { ctl.chooseOutput(index === 0 ? "" : page.outputs[index - 1].id) }
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.connection")
                    hint: ctl.laptopJack ? texts.t("hint.laptop") : ""
                    KanteInfoTip { term: "Hi-Z"; info: texts.t("help.hiz"); accessibleName: texts.t("help.about", { term: "Hi-Z" }) }
                }
                // LH-EIN-10: inputs that come later are entries of the choice, not left out (QA P5, P6).
                ConnectionChoice { Layout.fillWidth: true }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.latency")
                    hint: texts.t("settings.latency.hint")
                    RowLayout {
                        KanteSpinBox {
                            Accessible.name: texts.t("settings.latency")
                            from: 0
                            to: 500
                            stepSize: 5
                            value: Math.round(ctl.latencyMs)
                            // With its unit: a bare "0" said nothing (QA 14.10.).
                            textFromValue: function(v) { return v + " ms" }
                            valueFromText: function(t) { return parseInt(t) }
                            onValueModified: ctl.changeLatency(value)
                        }
                        // No cable fits the laptop's combo jack (P1); MIDI sends no sound (review #202).
                        KanteButton {
                            visible: !ctl.laptopJack && ctl.connection !== "midi"
                            text: texts.t("calibration.loopback.start")
                            busy: ctl.calibrating && ctl.cableMeasuring
                            enabled: !ctl.calibrating
                            onClicked: ctl.startLoopback()
                        }
                        KanteButton {
                            text: texts.t("calibration.start")
                            busy: ctl.calibrating && !ctl.cableMeasuring
                            enabled: !ctl.calibrating
                            onClicked: ctl.startCalibration()
                        }
                        KanteInfoTip { term: texts.t("settings.latency"); info: texts.t("help.latency"); accessibleName: texts.t("help.about", { term: texts.t("settings.latency") }) }
                        // How to measure: with a cable, or on the click (laptop jack and MIDI have no cable).
                        HintTip {
                            title: texts.t("calibration.start")
                            body: (ctl.laptopJack ? texts.t("calibration.loopback.jack")
                                   : ctl.connection === "midi" ? texts.t("calibration.loopback.midi")
                                   : texts.t("calibration.loopback.hint")) + " " + texts.t("calibration.hint")
                        }
                    }
                }
                // Where the value comes from and the player's lean, in words (R4, LH-EIN-05).
                QQC2.Label {
                    Layout.fillWidth: true
                    visible: ctl.latencyNote !== ""
                    wrapMode: Text.WordWrap
                    color: KanteStyle.textColor
                    text: ctl.latencyNote
                    Accessible.role: Accessible.StaticText
                    Accessible.name: texts.t("settings.latency") + ": " + Math.round(ctl.latencyMs) + " ms, " + text
                }
                CalibrationBeat { Layout.fillWidth: true }
                // The result of a measurement stays; how to measure sits behind the "?" (D4).
                QQC2.Label {
                    Layout.fillWidth: true
                    visible: ctl.calibrationText !== ""
                    wrapMode: Text.WordWrap
                    color: KanteStyle.mutedTextColor
                    text: ctl.calibrationText
                }
            }

            KanteSettingGroup {
                id: instrGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("instrument")].title
                countText: texts.t(count === 1 ? "settings.group.count.one" : "settings.group.count", { n: count })

                InstrumentChoice { Layout.fillWidth: true }
                KanteSettingRow {
                    Layout.fillWidth: true
                    visible: ctl.instrument !== "midi"
                    title: texts.t("settings.reference")
                    RowLayout {
                        KanteSpinBox {
                            Accessible.name: texts.t("settings.reference")
                            from: 430
                            to: 450
                            value: Math.round(ctl.a4Hz)
                            editable: true
                            textFromValue: function(value) { return value + " Hz" }
                            valueFromText: function(text) { return parseInt(text) }
                            onValueModified: ctl.changeReference(value)
                        }
                        HintTip { title: texts.t("settings.reference"); key: "settings.reference.hint" }
                    }
                }
            }

            KanteSettingGroup {
                id: displayGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("display")].title
                countText: texts.t(count === 1 ? "settings.group.count.one" : "settings.group.count", { n: count })

                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.lowtop")
                    hint: texts.t("settings.lowtop.hint")
                    KanteSwitch {
                        Accessible.name: texts.t("settings.lowtop")
                        checked: ctl.lowStringOnTop
                        onToggled: ctl.changeOptions(ctl.metronome, ctl.reference, checked)
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.lefthanded")
                    hint: texts.t("settings.lefthanded.hint")
                    KanteSwitch {
                        Accessible.name: texts.t("settings.lefthanded")
                        checked: ctl.leftHanded
                        onToggled: ctl.changeLeftHanded(checked)
                    }
                }
                KanteSettingRow {
                    id: colorsRow
                    Layout.fillWidth: true
                    title: texts.t("settings.colors")
                    RowLayout {
                        KanteSegmented {
                            Accessible.name: texts.t("settings.colors")
                            readonly property var values: ["kante", "mono", "bright"]
                            model: [texts.t("settings.colors.kante"), texts.t("settings.colors.mono"),
                                    texts.t("settings.colors.bright")]
                            currentIndex: Math.max(0, values.indexOf(ctl.stringColors))
                            onActivated: function(index) { ctl.changeStringColors(values[index]) }
                        }
                        HintTip { title: texts.t("settings.colors"); key: "settings.colors.hint" }
                    }
                }
                // Preview of the string colours with a hit, a wrong and a waiting note.
                KanteTabLane {
                    Layout.fillWidth: true
                    Layout.preferredHeight: KanteStyle.unit(24) * Math.max(4, ctl.strings)
                    strings: ctl.strings
                    stringNames: JSON.parse(ctl.profileTuningJson).map(function(s) { return s.name })
                    lowStringOnTop: ctl.lowStringOnTop
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
                    RowLayout {
                        KanteSegmented {
                            Accessible.name: texts.t("settings.playmarks")
                            readonly property var values: ["quiet", "off", "full"]
                            model: [texts.t("settings.playmarks.quiet"), texts.t("settings.playmarks.off"),
                                    texts.t("settings.playmarks.full")]
                            currentIndex: Math.max(0, values.indexOf(ctl.playMarks))
                            onActivated: function(index) { ctl.changePlayMarks(values[index]) }
                        }
                        HintTip { title: texts.t("settings.playmarks"); key: "settings.playmarks.hint" }
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
            }

            KanteSettingGroup {
                id: scoringGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("scoring")].title
                countText: texts.t(count === 1 ? "settings.group.count.one" : "settings.group.count", { n: count })

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
                        KanteSpinBox {
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
            }

            KanteSettingGroup {
                id: practiceGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("practice")].title
                countText: texts.t(count === 1 ? "settings.group.count.one" : "settings.group.count", { n: count })

                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("transport.metronome")
                    KanteSwitch {
                        Accessible.name: texts.t("transport.metronome")
                        checked: ctl.metronome
                        onToggled: ctl.changeOptions(checked, ctl.reference, ctl.lowStringOnTop)
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("transport.reference")
                    KanteSwitch {
                        Accessible.name: texts.t("transport.reference")
                        checked: ctl.reference
                        onToggled: ctl.changeOptions(ctl.metronome, checked, ctl.lowStringOnTop)
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.adaptive")
                    hint: texts.t("settings.adaptive.hint")
                    KanteSwitch {
                        Accessible.name: texts.t("settings.adaptive")
                        checked: ctl.adaptive
                        onToggled: ctl.changeAdaptive(checked)
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.band")
                    RowLayout {
                        KanteSwitch {
                            Accessible.name: texts.t("settings.band")
                            checked: ctl.reactingBand
                            onToggled: ctl.changeReactingBand(checked)
                        }
                        HintTip { title: texts.t("settings.band"); key: "settings.band.hint" }
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.simple")
                    RowLayout {
                        KanteSwitch {
                            Accessible.name: texts.t("settings.simple")
                            checked: ctl.simpleView
                            onToggled: ctl.changeSimpleView(checked)
                        }
                        HintTip { title: texts.t("settings.simple"); key: "settings.simple.hint" }
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    title: texts.t("settings.streak")
                    RowLayout {
                        KanteSwitch {
                            Accessible.name: texts.t("settings.streak")
                            checked: ctl.streak
                            onToggled: ctl.changeStreak(checked)
                        }
                        HintTip { title: texts.t("settings.streak"); key: "settings.streak.hint" }
                    }
                }
                KanteSettingRow {
                    Layout.fillWidth: true
                    // Not where a store announces updates (Flatpak).
                    visible: updateState.offered
                    title: texts.t("settings.update")
                    RowLayout {
                        KanteSwitch {
                            Accessible.name: texts.t("settings.update")
                            checked: updateState.enabled
                            onToggled: updateState = JSON.parse(ctl.updateNotice(checked ? "on" : "off"))
                        }
                        HintTip { title: texts.t("settings.update"); key: "settings.update.hint" }
                    }
                }
            }

            KanteSettingGroup {
                id: mixerGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("mixer")].title
                countText: ""
                Mixer { Layout.fillWidth: true }
            }

            KanteSettingGroup {
                id: languageGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("language")].title
                countText: texts.t(count === 1 ? "settings.group.count.one" : "settings.group.count", { n: count })

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
            }

            // LH-FOR-03: all user data as one open JSON file (backup, second computer).
            KanteSettingGroup {
                id: dataGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("data")].title
                countText: ""

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
            }

            // Round 16: each cache with its size, each entry by its song; delete one or all.
            KanteSettingGroup {
                id: cacheGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("cache")].title
                countText: ""

                Repeater {
                    id: cacheGroupsView
                    model: page.cacheGroups
                    delegate: ColumnLayout {
                        id: group
                        required property var modelData
                        readonly property var cache: page.caches[modelData.id]
                        Layout.fillWidth: true
                        spacing: 0
                        KanteSettingRow {
                            Layout.fillWidth: true
                            title: group.modelData.title
                            hint: texts.t(group.modelData.hint, { size: group.cache.total })
                            KanteButton {
                                text: texts.t("cache.clear")
                                // Also when only half-written files are left.
                                enabled: group.cache.bytes > 0
                                Accessible.name: texts.t("cache.clear.name", { cache: group.modelData.title })
                                onClicked: {
                                    ctl.clearCache(group.modelData.id)
                                    page.refreshCaches()
                                }
                            }
                        }
                        QQC2.Label {
                            visible: group.cache.entries.length === 0
                            text: texts.t("cache.empty")
                            color: KanteStyle.mutedTextColor
                        }
                        Repeater {
                            model: group.cache.entries
                            delegate: KanteListRow {
                                id: entry
                                required property var modelData
                                Layout.fillWidth: true
                                text: modelData.name
                                meta: modelData.size
                                focusOnClick: false
                                trailing: KanteButton {
                                    text: texts.t("cache.delete")
                                    emphasis: KanteButton.Emphasis.Quiet
                                    size: KanteButton.Size.Small
                                    Accessible.name: texts.t("cache.delete.name", { name: entry.modelData.name })
                                    onClicked: {
                                        ctl.forgetCached(group.modelData.id, entry.modelData.key)
                                        page.refreshCaches()
                                    }
                                }
                            }
                        }
                    }
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    color: KanteStyle.mutedTextColor
                    text: texts.t("cache.hint")
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
            }

            // LH-NF-11: Kontra's licence and every library inside it.
            KanteSettingGroup {
                id: aboutGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("about")].title
                countText: ""

                QQC2.Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    color: KanteStyle.textColor
                    text: texts.t("about.text")
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    color: KanteStyle.textColor
                    text: texts.t("about.vst")
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: KanteStyle.unit(16)
                    rowSpacing: KanteStyle.unit(2)
                    Accessible.name: texts.t("about.title")
                    Repeater {
                        // Kontra, Qt and KDE Frameworks first; the libraries on request (about 70 lines
                        // pushed the shortcuts out of reach, QA 13.10.).
                        model: allLicences.checked ? page.licences : page.licences.slice(0, page.mainLicences)
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
                KanteButton {
                    id: allLicences
                    checkable: true
                    emphasis: KanteButton.Emphasis.Quiet
                    text: texts.t("about.all", { n: page.licences.length - page.mainLicences })
                }
            }

            // All keyboard shortcuts in one place (LH-NF-07).
            KanteSettingGroup {
                id: keysGroup
                Layout.fillWidth: true
                title: page.sections[page.sectionIndex("keys")].title
                countText: ""

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
                            ["N", "keys.next"], ["1–4", "keys.mode"], ["Alt+1–5", "keys.view"], ["Ctrl+0–9", "keys.pages"], ["F5 / Esc", "keys.present"]
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
        }
    }

    // Runs below the sticky header (full-bleed, in page.overlay).
    HeaderScrollBar {
        flickable: flick
        headerHeight: header.height
        policy: flick.contentHeight > flick.height ? QQC2.ScrollBar.AlwaysOn : QQC2.ScrollBar.AsNeeded
    }

    // The page's own padding insets the scrolling content (reading width, shared with every
    // other page); the header stays full-bleed, so it goes into the page's overlay instead of
    // the padded content area.
    KanteStickyHeader {
        id: header
        parent: page.overlay
        anchors { left: parent.left; right: parent.right; top: parent.top }
        flickable: flick
        title: texts.t("nav.settings")
        tabsName: texts.t("settings.jump")
        sideMargin: page.leftPadding
        model: page.sections.map(function(s) { return s.title })
        targets: [connGroup, instrGroup, displayGroup, scoringGroup, practiceGroup, mixerGroup,
                  languageGroup, dataGroup, cacheGroup, aboutGroup, keysGroup]
    }

    // QA `--section data|about`: screenshots of the lower sections, once the layout settled.
    Timer {
        id: sectionTimer
        interval: 500
        onTriggered: {
            var key = ({ data: "data", about: "about" })[ctl.startSection]
            if (key) {
                header.scrollTo(page.sectionIndex(key))
            }
        }
    }

    FileDialog {

        // QA: Qt's dialog in the window, never the desktop portal.

        options: ctl.qaRun ? FileDialog.DontUseNativeDialog : 0
        id: exportDialog
        title: texts.t("dialog.export")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "json"
        selectedFile: StandardPaths.writableLocation(StandardPaths.DocumentsLocation) + "/" + "kontra-daten.json"
        nameFilters: [texts.t("data.files") + " (*.json)"]
        onAccepted: ctl.exportData(selectedFile.toString())
    }
    FileDialog {
        // QA: Qt's dialog in the window, never the desktop portal.
        options: ctl.qaRun ? FileDialog.DontUseNativeDialog : 0
        id: importDialog
        title: texts.t("dialog.import")
        nameFilters: [texts.t("data.files") + " (*.json)"]
        onAccepted: ctl.importData(selectedFile.toString(), page.dataHow)
    }
}
