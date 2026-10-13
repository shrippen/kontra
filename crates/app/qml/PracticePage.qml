import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami
import Kante

// Practice (LH-MOD-01..05, LH-ANZ-01..05): mode, view, the running notes, transport.
Kirigami.Page {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var modes: ["play", "song", "wait", "workshop"]
    readonly property var views: ["lane", "tab", "notation", "combined", "keys"]

    // The notes as loaded; live states go to the views one by one (setNoteState) and are kept
    // here too, so a view that gets its notes later (view switch) can catch up.
    property var notes: []
    property var bars: JSON.parse(ctl.barsJson)
    // The notation view: the same notes as written for bass, rests, key signature.
    property var staff: JSON.parse(ctl.staffJson)
    property var states: ({})
    // Technique results by note (LH-BEW-06), kept like `states` for views that load later.
    property var techniques: ({})
    // Play mode marks quietly or not at all (setting); the other modes mark fully.
    readonly property int marks: ctl.mode !== "play" || ctl.playMarks === "full" ? KanteTabLane.Marks.Full
                               : ctl.playMarks === "off" ? KanteTabLane.Marks.Off : KanteTabLane.Marks.Quiet
    readonly property var noteViews: [lane, tabStaff, bassStaff, combinedBass, combinedTab, keyLane]

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: ctl.title
    padding: KanteStyle.unit(12)

    property bool summaryReady: false

    // A new layout of a run that goes on (transposed, LH-WIE-02): the states stay and the views
    // catch up with them; otherwise a new piece or run starts clean.
    function reset() {
        summaryReady = false
        notes = JSON.parse(ctl.notesJson)
        bars = JSON.parse(ctl.barsJson)
        staff = JSON.parse(ctl.staffJson)
        if (!ctl.runKept) {
            states = {}
            techniques = {}
        }
    }

    // Re-applies the live states after `view` got a new notes list (which clears them).
    // Called through a fresh closure: Qt.callLater merges calls of the same function.
    function catchUp(view) {
        for (var key in states) {
            view.setNoteState(Number(key), states[key])
        }
        page.showTechniques(view)
    }

    // Technique results onto a view that draws technique signs (tab lane, tab, notation).
    function showTechniques(view) {
        if (typeof view.setTechniqueResult !== "function") {
            return
        }
        for (var key in techniques) {
            view.setTechniqueResult(Number(key), techniques[key])
        }
    }

    function barAt(time) {
        var index = 0
        for (var i = 0; i < bars.length; i++) {
            if (bars[i].time <= time) {
                index = i
            }
        }
        return index
    }

    Component.onCompleted: {
        reset()
        if (ctl.startPresenting) {
            present(true)
        }
    }

    // Keyboard (LH-NF-07, QA P7/P2): the transport keys work on the whole page, not only with
    // focus in the transport bar; modes 1–4, views Alt+1–4. Not while typing in a field.
    readonly property bool keysFree: {
        var item = applicationWindow().activeFocusItem
        return !(item && item.hasOwnProperty("cursorPosition"))
    }
    // Foot switches emulate a keyboard and mostly send PageDown/PageUp: play and rewind.
    Shortcut { sequences: ["K", "PgDown"]; enabled: page.keysFree; onActivated: ctl.togglePlay() }
    Shortcut { sequence: "N"; enabled: page.keysFree && ctl.nextTitle !== ""; onActivated: ctl.openNext() }
    Shortcut { sequences: ["Home", "PgUp"]; enabled: page.keysFree; onActivated: ctl.rewind() }
    // Loop start and end at the current bar; "," and "." for layouts where brackets need AltGr.
    Shortcut { sequences: ["[", ","]; enabled: page.keysFree; onActivated: ctl.loopEdge("start") }
    Shortcut { sequences: ["]", "."]; enabled: page.keysFree; onActivated: ctl.loopEdge("end") }
    Shortcut { sequence: "L"; enabled: page.keysFree; onActivated: transport.loopToggled() }
    Shortcut { sequence: "M"; enabled: page.keysFree; onActivated: ctl.changeOptions(!ctl.metronome, ctl.reference, ctl.lowStringOnTop) }
    Shortcut { sequence: "C"; enabled: page.keysFree; onActivated: ctl.changeCountIn(!ctl.countIn) }
    Shortcut { sequence: "X"; enabled: page.keysFree; onActivated: mixerButton.toggle() }
    Shortcut { sequences: ["+", "="]; enabled: page.keysFree; onActivated: ctl.changeSpeed(Math.min(1.5, ctl.speed + 0.05)) }
    Shortcut { sequence: "-"; enabled: page.keysFree; onActivated: ctl.changeSpeed(Math.max(0.25, ctl.speed - 0.05)) }
    Repeater {
        model: 4
        delegate: Item {
            Shortcut { sequence: String(index + 1); enabled: page.keysFree; onActivated: ctl.changeMode(page.modes[index]) }
            Shortcut { sequence: "Alt+" + (index + 1); onActivated: ctl.changeView(page.views[index]) }
        }
    }

    Connections {
        target: ctl
        function onArrangementLoaded() { page.reset() }
        function onNoteJudged(index, state, offsetMs, cents) {
            page.states[index] = state
            // Cents go along in full marks (LH-BEW-05); views that take no third argument ignore it.
            var shownCents = page.marks === KanteTabLane.Marks.Full && (state === "hit" || state === "offpitch") ? cents : undefined
            for (var i = 0; i < page.noteViews.length; i++) {
                page.noteViews[i].setNoteState(index, state, shownCents)
            }
        }
        function onNoteTechnique(index, technique, result) {
            page.techniques[index] = result
            for (var i = 0; i < page.noteViews.length; i++) {
                var view = page.noteViews[i]
                if (typeof view.setTechniqueResult === "function") {
                    view.setTechniqueResult(index, result)
                }
            }
        }
        // Play mode just plays (LH-MOD-01, round 6): the summary only on request.
        function onFinished() {
            if (ctl.mode === "play") {
                page.summaryReady = true
                summaryCallout.Accessible.announce(texts.t("summary.ready"))
            } else {
                summary.open()
            }
        }
        function onPlayingChanged() { if (ctl.playing) page.summaryReady = false }
        function onModeChanged() { page.summaryReady = false }
    }

    readonly property var times: bars.map(function(b) { return b.time })
    // Most onsets in one bar: tablature gives dense bars room (sixteenths with ghost notes ran
    // together, QA 13.10.). Chords count once.
    readonly property int densestBar: {
        var most = 0, bar = 0, count = 0, last = -1
        for (var i = 0; i < notes.length; i++) {
            var t = notes[i].time
            while (bar + 1 < times.length && times[bar + 1] <= t) {
                bar++
                count = 0
            }
            if (t === last) {
                continue
            }
            last = t
            count++
            most = Math.max(most, count)
        }
        return most
    }
    // Room per onset in the tablature, ghost note brackets included.
    readonly property real tabNoteWidth: KanteStyle.unit(26) * presentScale
    readonly property var stringNames: JSON.parse(ctl.stringNamesJson)
    // Keyboard view (J-E1): the instrument's range and the note names in the user's language.
    readonly property var keys: JSON.parse(ctl.keyRangeJson)
    // Backing track without bass (LH-BIB-09): {available, state, progress, text, installText}.
    readonly property var stems: ctl.stemsJson !== "" ? JSON.parse(ctl.stemsJson) : ({ state: "none" })
    readonly property int nextIndex: ctl.holdingIndex >= 0 ? ctl.holdingIndex : lane.nextIndex
    readonly property var nextNote: nextIndex >= 0 && nextIndex < notes.length ? notes[nextIndex] : null
    // Hand position (LH-ANZ-05) and the next change of it, shown at least one bar ahead.
    readonly property int handPosition: nextNote ? nextNote.position : 0
    readonly property var shift: positionShift(nextIndex, ctl.position)

    // The first note from `from` on whose position differs, if it starts within a bar from `now`.
    function positionShift(from, now) {
        if (from < 0 || from >= notes.length || bars.length === 0) {
            return null
        }
        var bar = barAt(now)
        var length = bar + 1 < bars.length ? bars[bar + 1].time - bars[bar].time : bars[bar].time - (bar > 0 ? bars[bar - 1].time : 0)
        var horizon = Math.max(now, notes[from].time) + Math.max(length, 0.5)
        for (var i = from + 1; i < notes.length && notes[i].time <= horizon; i++) {
            if (notes[i].position !== notes[from].position) {
                return { position: notes[i].position, bar: bars[barAt(notes[i].time)].number }
            }
        }
        return null
    }

    // Small screens and beamers (QA P4: 1280×720 at 150 %): the title gets its own row and the
    // fretboard and level make room for the notes.
    readonly property bool narrow: width < KanteStyle.unit(1000)
    readonly property bool compact: height < KanteStyle.unit(560)

    // Presentation (LH-ANZ-09, P4 at the beamer): full screen, only title, notes and transport;
    // staff lines half as large again, the lane grows with the room. F5 toggles, Esc ends.
    readonly property bool presenting: applicationWindow().presenting
    readonly property real presentScale: presenting ? 1.5 : 1
    function present(on) { applicationWindow().presenting = on }
    Shortcut { sequence: "F5"; onActivated: page.present(!page.presenting) }
    Shortcut { sequence: "Escape"; enabled: page.presenting && !summary.visible; onActivated: page.present(false) }
    // The summary closes like a dialog (QA 13.10.: only a click closed it).
    Shortcut { sequence: "Escape"; enabled: summary.visible; onActivated: summary.close() }
    Component.onDestruction: present(false)

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(10)

        GridLayout {
            Layout.fillWidth: true
            columns: page.narrow ? 2 : 3
            columnSpacing: KanteStyle.unit(12)
            rowSpacing: KanteStyle.unit(8)
            ColumnLayout {
                Layout.fillWidth: true
                Layout.columnSpan: page.narrow ? 2 : 1
                spacing: 0
                KanteHeading { text: ctl.title; pageTitle: true; Layout.fillWidth: true; elide: Text.ElideRight }
                QQC2.Label { text: ctl.artist; visible: ctl.artist !== ""; color: KanteStyle.mutedTextColor }
                // Kontra picks the bass track itself; another can be chosen (QA P2 R9).
                RowLayout {
                    readonly property var tracks: JSON.parse(ctl.tracksJson)
                    visible: tracks.names.length > 1 && !page.presenting
                    QQC2.Label { text: texts.t("track.label"); color: KanteStyle.mutedTextColor }
                    KanteComboBox {
                        Layout.preferredWidth: KanteStyle.unit(220)
                        Accessible.name: texts.t("track.label")
                        model: parent.tracks.names
                        currentIndex: parent.tracks.current
                        onActivated: function(index) { ctl.switchTrack(index) }
                    }
                }
            }
            KanteSegmented {
                visible: !page.presenting
                model: page.modes.map(function(m) { return texts.t("mode." + m) })
                tooltips: page.modes.map(function(m) { return texts.t("mode." + m + ".hint") })
                currentIndex: Math.max(0, page.modes.indexOf(ctl.mode))
                onActivated: function(index) { ctl.changeMode(page.modes[index]) }
            }
            KanteSegmented {
                visible: !page.presenting && !page.narrow
                model: page.views.map(function(v) { return texts.t("view." + v) })
                currentIndex: Math.max(0, page.views.indexOf(ctl.view))
                onActivated: function(index) { ctl.changeView(page.views[index]) }
            }
            // Narrow windows (P4 at 1280×720, 150 %): five views do not fit next to the modes.
            KanteComboBox {
                visible: !page.presenting && page.narrow
                Layout.preferredWidth: KanteStyle.unit(200)
                Accessible.name: texts.t("view.title")
                model: page.views.map(function(v) { return texts.t("view." + v) })
                currentIndex: Math.max(0, page.views.indexOf(ctl.view))
                onActivated: function(index) { ctl.changeView(page.views[index]) }
            }
        }

        // Backing track (LH-WIE-01): chosen per piece, stretched with the speed.
        RowLayout {
            Layout.fillWidth: true
            visible: !page.presenting
            spacing: KanteStyle.unit(8)
            KanteHud { fullText: texts.t("backing.title"); color: KanteStyle.mutedTextColor }
            QQC2.Label {
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                text: ctl.backingName !== "" ? ctl.backingName : texts.t("backing.none")
                color: ctl.backingName !== "" ? KanteStyle.textColor : KanteStyle.mutedTextColor
            }
            KanteButton {
                text: texts.t("backing.choose")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: backingDialog.open()
            }
            // Setlist (LH-BIB-04, J-B1: next song in at most two clicks; also the N key).
            KanteButton {
                visible: ctl.nextTitle !== ""
                text: texts.t("setlist.next", { title: ctl.nextTitle })
                emphasis: KanteButton.Emphasis.Primary
                onClicked: ctl.openNext()
            }
            KanteButton {
                visible: ctl.backingName !== ""
                text: texts.t("backing.align")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: alignSheet.open()
            }
            KanteButton {
                visible: ctl.backingName !== ""
                text: texts.t("backing.remove")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: ctl.chooseBacking("")
            }

            // Transpose (LH-WIE-02): piece and backing move together; the notes are laid out
            // again. In the backing's row, so the notes keep the height (QA 13.10.).
            KanteHud {
                visible: !page.narrow
                Layout.leftMargin: KanteStyle.unit(16)
                fullText: texts.t("transpose.title")
                color: KanteStyle.mutedTextColor
            }
            KanteSpinBox {
                from: -ctl.transposeMax
                to: ctl.transposeMax
                value: ctl.transpose
                Accessible.name: texts.t("transpose.title")
                textFromValue: function(value) {
                    return value === 0 ? texts.t("transpose.none")
                        : texts.t("transpose.semitones", { n: (value > 0 ? "+" : "") + value })
                }
                onValueModified: ctl.changeTranspose(value)
            }
            KanteInfoTip {
                term: texts.t("transpose.title")
                info: texts.t("transpose.hint")
                accessibleName: texts.t("help.about", { term: texts.t("transpose.title") })
            }
        }

        // Without the recording's bass (LH-BIB-09, round 9): Demucs does it once, if installed;
        // without it the switch is greyed out and the tip says how to install it.
        RowLayout {
            Layout.fillWidth: true
            visible: !page.presenting && page.stems.state !== "none"
            spacing: KanteStyle.unit(8)
            readonly property bool running: page.stems.state === "running"
            KanteHud { fullText: texts.t("stems.title"); color: KanteStyle.mutedTextColor }
            KanteSwitch {
                text: texts.t("stems.switch")
                enabled: page.stems.available && !parent.running
                checked: page.stems.state === "on" || parent.running
                // A click replaces the binding; the state comes from Kontra (failed, cancelled).
                onToggled: {
                    ctl.removeBass(checked)
                    checked = Qt.binding(function() { return page.stems.state === "on" || parent.running })
                }
            }
            KanteProgressBar {
                visible: parent.running
                Layout.preferredWidth: KanteStyle.unit(120)
                value: page.stems.progress || 0
            }
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: page.stems.text || ""
                color: page.stems.state === "failed" ? KanteStyle.negativeTextColor : KanteStyle.mutedTextColor
            }
            KanteInfoTip {
                visible: !page.stems.available
                term: texts.t("stems.install.title")
                info: page.stems.installText || ""
                accessibleName: texts.t("stems.install.title")
            }
            KanteButton {
                visible: !page.stems.available
                text: texts.t("stems.retry")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: ctl.refreshStems()
            }
            KanteButton {
                visible: parent.running
                text: texts.t("stems.cancel")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: ctl.cancelStems()
            }
        }

        // Workshop (LH-MOD-04): which bars, and how the tempo follows the hit rate (target band).
        RowLayout {
            id: workshopRow
            Layout.fillWidth: true
            visible: ctl.mode === "workshop" && !page.presenting
            spacing: KanteStyle.unit(8)
            readonly property int fromBar: ctl.loopStart >= 0 ? page.barAt(ctl.loopStart) + 1 : 0
            readonly property int toBar: ctl.loopEnd >= 0 ? page.barAt(ctl.loopEnd - 0.001) + 1 : 0
            property string lastChange: ""

            KanteHud { fullText: texts.t("workshop.bars"); color: KanteStyle.mutedTextColor }
            KanteSpinBox {
                id: fromSpin
                from: 1
                to: Math.max(1, page.bars.length)
                value: Math.max(1, workshopRow.fromBar)
                editable: true
                Accessible.name: texts.t("workshop.from")
                onValueModified: ctl.loopBars(value - 1, Math.max(value, toSpin.value) - 1)
            }
            QQC2.Label { text: texts.t("workshop.to"); color: KanteStyle.textColor }
            KanteSpinBox {
                id: toSpin
                from: 1
                to: Math.max(1, page.bars.length)
                value: Math.max(1, workshopRow.toBar > 0 ? workshopRow.toBar : fromSpin.value)
                editable: true
                Accessible.name: texts.t("workshop.until")
                onValueModified: ctl.loopBars(Math.min(fromSpin.value, value) - 1, value - 1)
            }
            QQC2.Label {
                Layout.fillWidth: true
                // As wide as the text, so the info button sits right after it.
                Layout.maximumWidth: implicitWidth
                wrapMode: Text.WordWrap
                color: KanteStyle.textColor
                text: {
                    var band = texts.t("workshop.band", { low: Math.round(ctl.bandLow * 100), high: Math.round(ctl.bandHigh * 100) })
                    var speed = texts.t("workshop.speed", { percent: Math.round(ctl.speed * 100) })
                    if (ctl.loopStart < 0)
                        return texts.t("workshop.choose") + " · " + band
                    if (ctl.passRate < 0)
                        return band + " · " + speed
                    return texts.t("workshop.pass", { n: ctl.passes, percent: Math.round(ctl.passRate * 100) })
                        + " · " + band + " · " + speed
                        + (workshopRow.lastChange !== "" ? " · " + workshopRow.lastChange : "")
                }
            }
            KanteInfoTip {
                term: texts.t("help.band.term")
                accessibleName: texts.t("help.about", { term: texts.t("help.band.term") })
                info: texts.t("help.band", { low: Math.round(ctl.bandLow * 100), high: Math.round(ctl.bandHigh * 100) })
            }
            Item { Layout.fillWidth: true }
            Connections {
                target: ctl
                function onPassComplete(clean, hitRate) {
                    workshopRow.lastChange = hitRate > ctl.bandHigh ? texts.t("workshop.above")
                        : hitRate < ctl.bandLow ? texts.t("workshop.below") : texts.t("workshop.inside")
                }
                function onModeChanged() { workshopRow.lastChange = "" }
            }
        }

        // Note density (LH-MOD-06): fewer notes while a part is new; per section, fixable.
        RowLayout {
            Layout.fillWidth: true
            visible: ctl.mode === "song" || ctl.mode === "workshop"
            spacing: KanteStyle.unit(8)
            KanteHud { fullText: texts.t("density.title"); color: KanteStyle.mutedTextColor }
            KanteSegmented {
                Accessible.name: texts.t("density.title")
                model: [texts.t("density.sparse"), texts.t("density.beats"), texts.t("density.full")]
                tooltips: [texts.t("density.hint"), texts.t("density.hint"), texts.t("density.hint")]
                currentIndex: ctl.density - 1
                onActivated: function(index) { ctl.changeDensity(index + 1, ctl.densityFixed ? "fixed" : "free") }
            }
            KanteSwitch {
                checked: ctl.densityFixed
                Accessible.name: texts.t("density.fixed")
                onToggled: ctl.changeDensity(ctl.density, checked ? "fixed" : "free")
            }
            QQC2.Label { text: texts.t("density.fixed"); color: KanteStyle.textColor }
            Item { Layout.fillWidth: true }
        }

        // Between runs only: Kontra suggests, the player decides (round 5).
        KanteCallout {
            id: densityCallout
            Layout.fillWidth: true
            visible: ctl.densitySuggestion > 0 && !ctl.playing
            kind: KanteCallout.Kind.Info
            autoHide: false
            title: ctl.densityText
            Accessible.name: ctl.densityText
            onVisibleChanged: if (visible) Accessible.announce(ctl.densityText)
            RowLayout {
                KanteButton {
                    text: texts.t("density.accept")
                    emphasis: KanteButton.Emphasis.Primary
                    onClicked: ctl.changeDensity(ctl.densitySuggestion, "free")
                }
                KanteButton {
                    text: texts.t("density.decline")
                    emphasis: KanteButton.Emphasis.Quiet
                    onClicked: ctl.declineDensity()
                }
            }
        }

        // Wait mode: what to play next, in words (LH-MOD-03). An instruction, not a warning (QA
        // P1: the hazard banner read as an alarm); read out by screen readers when it changes (P7).
        KanteCallout {
            id: waitCallout
            Layout.fillWidth: true
            visible: ctl.holdingIndex >= 0
            kind: KanteCallout.Kind.Info
            autoHide: false
            title: ctl.holdingText
        }
        Connections {
            target: ctl
            function onHoldingTextChanged() {
                if (ctl.holdingIndex >= 0 && ctl.holdingText !== "")
                    waitCallout.Accessible.announce(ctl.holdingText)
            }
        }

        // Play mode: the run is over, the summary waits behind a button.
        KanteCallout {
            id: summaryCallout
            Layout.fillWidth: true
            visible: page.summaryReady && !summary.visible
            kind: KanteCallout.Kind.Info
            autoHide: false
            title: texts.t("summary.ready")
            Accessible.name: texts.t("summary.ready")
            KanteButton {
                text: texts.t("summary.show")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: summary.open()
            }
        }

        // Clipped: on small screens a staff taller than its room drew over the transport (QA 13.10.).
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            KanteTabLane {
                id: lane
                anchors.fill: parent
                visible: ctl.view === "lane"
                strings: page.stringNames.length
                stringNames: page.stringNames
                lowStringOnTop: ctl.lowStringOnTop
                notes: page.notes
                marks: page.marks
                onNotesChanged: Qt.callLater(function() { page.catchUp(lane) })
                bars: page.times
                position: ctl.position
                // Wide enough that eighths at 160 BPM do not overlap: about two note heads per beat.
                pixelsPerSecond: Math.max(KanteStyle.unit(320), height * 1.6) * Math.max(0.5, ctl.speed)
                accessibleName: texts.t("lane.name")
                loopStart: ctl.loopStart
                loopEnd: ctl.loopEnd
            }
            KanteTabStaff {
                id: tabStaff
                anchors.fill: parent
                // As many lines as fit (QA P2 R6: one line left the next bars hidden).
                systems: Math.max(1, Math.floor(height / (KanteStyle.unit(170) * page.presentScale)))
                lineSpacing: KanteStyle.unit(14) * page.presentScale
                minBarWidth: Math.max(KanteStyle.unit(200), page.densestBar * page.tabNoteWidth)
                visible: ctl.view === "tab"
                strings: page.stringNames.length
                stringNames: page.stringNames
                lowStringOnTop: ctl.lowStringOnTop
                notes: visible ? page.notes : []
                marks: page.marks
                onNotesChanged: Qt.callLater(function() { page.catchUp(tabStaff) })
                bars: page.bars
                position: ctl.position
            }
            KanteBassStaff {
                id: bassStaff
                anchors.fill: parent
                // Smaller where one line does not fit (beamer at 150 %): cut off, its beams ran
                // over the transport (G18). A KanteBassStaff line is 14.5 staff spaces, 16.5 with
                // chord symbols.
                staffSpace: Math.min(KanteStyle.unit(10) * page.presentScale, height / (hasChords ? 16.5 : 14.5))
                // As many lines as fit, as in the tablature (QA 13.10.: one line, the rest empty).
                systems: Math.max(1, Math.floor(height / systemHeight))
                visible: ctl.view === "notation"
                notes: visible ? page.staff.notes : []
                rests: page.staff.rests
                keyFifths: page.staff.keyFifths
                marks: page.marks
                onNotesChanged: Qt.callLater(function() { page.catchUp(bassStaff) })
                bars: page.bars
                position: ctl.position
            }
            // Notation above tablature (LH-ANZ-04).
            ColumnLayout {
                id: combined
                anchors.fill: parent
                visible: ctl.view === "combined"
                spacing: KanteStyle.unit(8)
                KanteBassStaff {
                    id: combinedBass
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    // Half the room is the staff's (G18); from the column, not its own height,
                    // which follows its size.
                    staffSpace: Math.min(KanteStyle.unit(10) * page.presentScale,
                                         (combined.height - combined.spacing) / 2 / (hasChords ? 16.5 : 14.5))
                    // The same bars as the tablature below, so each bar sits over its tab.
                    barsPerSystem: combinedTab.perSystem
                    notes: combined.visible ? page.staff.notes : []
                    rests: page.staff.rests
                    keyFifths: page.staff.keyFifths
                    marks: page.marks
                    onNotesChanged: Qt.callLater(function() { page.catchUp(combinedBass) })
                    bars: page.bars
                    position: ctl.position
                }
                KanteTabStaff {
                    id: combinedTab
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    lineSpacing: KanteStyle.unit(14) * page.presentScale
                    minBarWidth: Math.max(KanteStyle.unit(200), page.densestBar * page.tabNoteWidth)
                    strings: page.stringNames.length
                    stringNames: page.stringNames
                    lowStringOnTop: ctl.lowStringOnTop
                    notes: combined.visible ? page.notes : []
                    marks: page.marks
                    onNotesChanged: Qt.callLater(function() { page.catchUp(combinedTab) })
                    bars: page.bars
                    position: ctl.position
                }
            }
            // Bass over MIDI (J-E1, P5): the notes fall onto the keys to play.
            ColumnLayout {
                id: keysView
                anchors.fill: parent
                visible: ctl.view === "keys"
                spacing: 0
                KanteKeyLane {
                    id: keyLane
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    low: page.keys.low
                    high: page.keys.high
                    noteNames: page.keys.names
                    notes: keysView.visible ? page.notes : []
                    marks: page.marks
                    onNotesChanged: Qt.callLater(function() { page.catchUp(keyLane) })
                    bars: page.bars
                    position: ctl.position
                    pixelsPerSecond: Math.max(KanteStyle.unit(240), height * 1.2) * Math.max(0.5, ctl.speed)
                    accessibleName: texts.t("keyboard.lane")
                    loopStart: ctl.loopStart
                    loopEnd: ctl.loopEnd
                }
                KanteKeyboard {
                    Layout.fillWidth: true
                    Layout.preferredHeight: KanteStyle.unit(90)
                    low: page.keys.low
                    high: page.keys.high
                    noteNames: page.keys.names
                    markers: page.nextNote ? [{ midi: page.nextNote.midi, role: "next" }] : []
                    accessibleName: texts.t("keyboard.name")
                    roleNames: ({ current: texts.t("keyboard.current"), next: texts.t("keyboard.next"),
                                  root: texts.t("keyboard.root"), scale: texts.t("keyboard.scale") })
                    markerText: texts.t("keyboard.marker")
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !page.compact && !page.presenting
            spacing: KanteStyle.unit(16)
            ColumnLayout {
            Layout.fillWidth: true
            // The keyboard view has its own keys; a fretboard beside them would only distract.
            visible: ctl.view !== "keys"
            spacing: KanteStyle.unit(4)
            KanteFretboard {
                Layout.fillWidth: true
                Layout.preferredHeight: KanteStyle.unit(120)
                strings: page.stringNames.length
                stringNames: page.stringNames
                frets: 15
                lowStringOnTop: ctl.lowStringOnTop
                leftHanded: ctl.leftHanded
                // The finger the position suggests is written in the marker (1 index … 4 pinky).
                markers: page.nextNote ? [{ string: page.nextNote.string, fret: page.nextNote.fret, role: "next",
                                            label: page.nextNote.finger > 0 ? String(page.nextNote.finger) : "" }] : []
                accessibleName: texts.t("fretboard.name")
                markerText: texts.t("fretboard.marker")
            }
            RowLayout {
                spacing: KanteStyle.unit(8)
                visible: page.handPosition > 0
                KantePill {
                    status: KantePill.State.Off
                    text: texts.t("fretboard.position", { position: page.handPosition })
                    Accessible.name: text
                }
                KantePill {
                    id: shiftPill
                    visible: page.shift !== null
                    status: KantePill.State.Locked
                    text: page.shift ? texts.t("fretboard.shift", { position: page.shift.position, bar: page.shift.bar }) : ""
                    Accessible.name: text
                    onTextChanged: if (text !== "") Accessible.announce(text)
                }
                QQC2.Label {
                    visible: page.nextNote !== null && page.nextNote.finger > 0
                    text: page.nextNote ? texts.t("fretboard.finger", { finger: page.nextNote.finger }) : ""
                    color: KanteStyle.mutedTextColor
                }
            }
            }
            ColumnLayout {
            Layout.preferredWidth: meter.implicitWidth
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
                listening: ctl.playing
                onUseLaptopJack: ctl.changeConnection("laptop")
            }
            }
        }

        // The mixer opens above the transport (in the page, so screenshots show it).
        KanteCard {
            Layout.fillWidth: true
            Layout.preferredHeight: pageMixer.implicitHeight + KanteStyle.unit(16)
            visible: mixerButton.checked
            barColor: KanteStyle.accentColor
            borderColor: KanteStyle.frameColor
            Mixer {
                id: pageMixer
                x: KanteStyle.unit(12)
                y: KanteStyle.unit(8)
                width: parent.width - KanteStyle.unit(24)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(8)
            KanteTransportBar {
                id: transport
                Layout.fillWidth: true
                playing: ctl.playing
                looping: ctl.loopStart >= 0
                metronome: ctl.metronome
                countIn: ctl.countIn
                speed: ctl.speed
                rewindText: texts.t("transport.rewind")
                playText: texts.t("transport.play")
                pauseText: texts.t("transport.pause")
                loopText: texts.t("transport.loop")
                speedText: texts.t("transport.speed")
                metronomeText: texts.t("transport.metronome")
                countInText: texts.t("transport.countin")
                positionName: texts.t("transport.position")
                accessibleName: texts.t("transport.name")
                onCountInToggled: ctl.changeCountIn(!ctl.countIn)
                minSpeed: 0.25
                maxSpeed: 1.5
                position: ctl.position
                duration: ctl.duration
                onPlayToggled: ctl.togglePlay()
                onRewind: ctl.rewind()
                onSpeedChangeRequested: function(speed) { ctl.changeSpeed(speed) }
                onMetronomeToggled: ctl.changeOptions(!ctl.metronome, ctl.reference, ctl.lowStringOnTop)
                onLoopToggled: {
                    if (ctl.loopStart >= 0) {
                        ctl.clearLoop()
                        return
                    }
                    var bar = page.barAt(ctl.position)
                    ctl.loopBars(bar, Math.min(bar + 1, page.bars.length - 1))
                }
            }
            KanteToolButton {
                id: presentButton
                Layout.alignment: Qt.AlignTop
                Layout.preferredHeight: KanteStyle.heightMedium
                text: page.presenting ? texts.t("present.end") : texts.t("present.start")
                display: QQC2.AbstractButton.TextOnly
                implicitWidth: Math.ceil(leftPadding + presentWords.advanceWidth + rightPadding + KanteStyle.unit(2))
                focusPolicy: Qt.StrongFocus
                Accessible.name: text
                QQC2.ToolTip.text: texts.t("present.hint")
                QQC2.ToolTip.visible: hovered
                onClicked: page.present(!page.presenting)
                TextMetrics { id: presentWords; font: presentButton.font; text: presentButton.text }
            }
            KanteToolButton {
                id: mixerButton
                visible: !page.presenting
                Layout.alignment: Qt.AlignTop
                Layout.preferredHeight: KanteStyle.heightMedium
                text: texts.t("mixer.title")
                display: QQC2.AbstractButton.TextOnly
                implicitWidth: Math.ceil(leftPadding + mixerWords.advanceWidth + rightPadding + KanteStyle.unit(2))
                checkable: true
                checked: ctl.showMixer
                focusPolicy: Qt.StrongFocus
                Accessible.name: text
                QQC2.ToolTip.text: texts.t("keys.mixer")
                QQC2.ToolTip.visible: hovered
                TextMetrics { id: mixerWords; font: mixerButton.font; text: mixerButton.text }
            }
        }
    }

    BackingAlign {
        id: alignSheet
        anchors.centerIn: parent
        width: Math.min(page.width - KanteStyle.unit(40), KanteStyle.unit(640))
        z: 5
        Component.onCompleted: if (ctl.showAlign) open()
    }

    FileDialog {
        id: backingDialog
        nameFilters: [texts.t("backing.filter") + " (*.wav *.flac *.ogg *.mp3)"]
        onAccepted: ctl.chooseBacking(selectedFile.toString())
    }

    // The summary covers the notes completely (QA P1/P4: the page showed through the card).
    Rectangle {
        anchors.fill: parent
        anchors.margins: -page.padding
        visible: summary.visible
        z: 4
        color: Qt.rgba(KanteStyle.backgroundColor.r, KanteStyle.backgroundColor.g, KanteStyle.backgroundColor.b, 0.92)
        MouseArea { anchors.fill: parent; onClicked: summary.close() }
    }

    SummarySheet {
        id: summary
        anchors.centerIn: parent
        width: Math.min(page.width - KanteStyle.unit(40), KanteStyle.unit(640))
        height: Math.min(contentHeight, page.height - KanteStyle.unit(24))
        z: 5
        onPracticeBar: function(bar) {
            ctl.changeMode("workshop")
            ctl.loopBars(bar, bar)
            summary.close()
        }
    }
}
