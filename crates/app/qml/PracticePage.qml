import QtQuick
import QtCore
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
    // The technique signs in this piece, once each (the key under the fretboard).
    readonly property var signs: {
        var found = []
        for (var i = 0; i < notes.length; i++) {
            var t = notes[i].technique
            if (t && found.indexOf(t) < 0) {
                found.push(t)
            }
        }
        return found
    }
    // Chord symbols stand over the notation only.
    readonly property bool hasChordSymbols: (ctl.view === "notation" || ctl.view === "combined")
        && bars.some(function(b) { return b.chord })
    property var bars: JSON.parse(ctl.barsJson)
    // The notation view: the same notes as written for bass, rests, key signature.
    property var staff: JSON.parse(ctl.staffJson)
    property var states: ({})
    // A blind probe (DIDAKTIK I1) shows bars and cursor only; the marks come back after it.
    readonly property var shown: ctl.probing ? [] : notes
    readonly property var shownStaff: ctl.probing ? [] : staff.notes
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
    // focus in the transport bar; modes 1–4, views Alt+1–5. Not while typing in a field.
    readonly property bool keysFree: {
        var item = applicationWindow().activeFocusItem
        return !(item && item.hasOwnProperty("cursorPosition"))
    }
    // A bar typed into a workshop field ends with Return (taken) or Escape (dropped): the keys go
    // back to the page; before, K and the foot switch stayed dead (QA 27.10., P2 Jens).
    function endField() { keySink.forceActiveFocus() }
    function cancelField(spin) {
        // The typed text goes before the field lets go (it takes its text then); a binding
        // again after, so the field follows the loop when it moves later.
        spin.contentItem.text = spin.textFromValue(spin.value, spin.locale)
        keySink.forceActiveFocus()
        spin.contentItem.text = Qt.binding(function() { return spin.displayText })
    }
    // The page is a focus scope: focused itself, it hands the focus back to the field.
    Item { id: keySink }

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
        model: page.modes.length
        delegate: Item {
            Shortcut { sequence: String(index + 1); enabled: page.keysFree; onActivated: ctl.changeMode(page.modes[index]) }
        }
    }
    // Every view, the keyboard (Alt+5) included (QA 27.10.: it had no key).
    Repeater {
        model: page.views.length
        delegate: Item {
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
        // A loop pass ended: the next one starts clean. Kept, the notes ahead showed the last
        // pass's marks before they were played (QA 14.10., workshop).
        function onPassComplete(clean, hitRate) {
            page.states = {}
            page.techniques = {}
            for (var i = 0; i < page.noteViews.length; i++) {
                page.noteViews[i].resetStates()
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
    readonly property var nextNote: nextIndex >= 0 && nextIndex < shown.length ? shown[nextIndex] : null
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
    // Half a laptop screen (beside a video or a tab): one column, no fretboard (QA 13.10.).
    readonly property bool slim: width < KanteStyle.unit(700)
    readonly property bool compact: height < KanteStyle.unit(560)
    // Five views as buttons need about 470 px: below this width a list, so modes, view and
    // transposing share one row (QA 14.10., 1280 px with the sidebar).
    readonly property bool viewsAsList: width < KanteStyle.unit(1300)

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

    // Nothing open yet (QA 14.10.: an empty lane with live buttons): the ways to a piece.
    ColumnLayout {
        anchors.fill: parent
        visible: !ctl.loaded
        spacing: KanteStyle.unit(16)
        KanteHeading { text: texts.t("nav.practice"); pageTitle: true }
        KanteEmptyState {
            Layout.fillWidth: true
            title: texts.t("practice.empty.title")
            text: texts.t("practice.empty.text")
            Row {
                spacing: KanteStyle.unit(8)
                KanteButton {
                    text: texts.t("practice.empty.exercises")
                    emphasis: KanteButton.Emphasis.Primary
                    onClicked: applicationWindow().show("exercises")
                }
                KanteButton {
                    text: texts.t("practice.empty.library")
                    onClicked: applicationWindow().show("library")
                }
            }
        }
        Item { Layout.fillHeight: true }
    }

    ColumnLayout {
        anchors.fill: parent
        visible: ctl.loaded
        // Under the summary nothing takes the focus (QA 27.10., P7: Tab went through 18 hidden
        // controls before "close").
        enabled: !summary.visible
        spacing: KanteStyle.unit(10)

        // The piece's title on a row of its own (QA 14.10.: beside modes and views it was cut
        // to "Groove Viertel auf einem Akkord in…"); the page's own actions beside it.
        RowLayout {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(8)
            ColumnLayout {
                Layout.fillWidth: true
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
            // Setlist (LH-BIB-04, J-B1: next song in at most two clicks; also the N key).
            KanteButton {
                visible: ctl.nextTitle !== "" && !page.presenting
                Layout.alignment: Qt.AlignTop
                text: texts.t("setlist.next", { title: ctl.nextTitle })
                emphasis: KanteButton.Emphasis.Primary
                onClicked: ctl.openNext()
            }
            // A backing track is for some (P2); without one its row stays away (QA 14.10.).
            KanteToolButton {
                id: backingButton
                visible: ctl.backingName === "" && !page.presenting
                Layout.alignment: Qt.AlignTop
                Layout.preferredHeight: KanteStyle.heightMedium
                text: texts.t("backing.choose")
                display: QQC2.AbstractButton.TextOnly
                implicitWidth: Math.ceil(leftPadding + backingWords.advanceWidth + rightPadding + KanteStyle.unit(2))
                focusPolicy: Qt.StrongFocus
                Accessible.name: text
                QQC2.ToolTip.text: texts.t("backing.hint")
                QQC2.ToolTip.visible: hovered
                onClicked: backingDialog.open()
                TextMetrics { id: backingWords; font: backingButton.font; text: backingButton.text }
            }
            KanteToolButton {
                id: presentButton
                // Only the narrowest pages leave it out (F5 still works); at a beamer with 853 px it
                // is what the teacher looks for (QA 27.10., P4).
                visible: page.width >= KanteStyle.unit(560) || page.presenting
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

        // How to play and what to see: modes, then views (a list where five do not fit).
        Flow {
            Layout.fillWidth: true
            visible: !page.presenting
            spacing: KanteStyle.unit(12)
            KanteSegmented {
                Accessible.name: texts.t("mode.title")
                model: page.modes.map(function(m) { return texts.t("mode." + m) })
                tooltips: page.modes.map(function(m) { return texts.t("mode." + m + ".hint") })
                currentIndex: Math.max(0, page.modes.indexOf(ctl.mode))
                onActivated: function(index) { ctl.changeMode(page.modes[index]) }
            }
            KanteSegmented {
                visible: !page.viewsAsList
                Accessible.name: texts.t("view.title")
                model: page.views.map(function(v) { return texts.t("view." + v) })
                currentIndex: Math.max(0, page.views.indexOf(ctl.view))
                onActivated: function(index) { ctl.changeView(page.views[index]) }
            }
            // Narrow windows (P4 at 1280×720, 150 %): five views do not fit next to the modes.
            KanteComboBox {
                visible: page.viewsAsList
                width: KanteStyle.unit(200)
                Accessible.name: texts.t("view.title")
                model: page.views.map(function(v) { return texts.t("view." + v) })
                currentIndex: Math.max(0, page.views.indexOf(ctl.view))
                onActivated: function(index) { ctl.changeView(page.views[index]) }
            }
            // Transpose (LH-WIE-02): piece and backing move together; the notes are laid out
            // again. Beside modes and views, so the notes keep the height (QA 13.10.).
            Row {
            spacing: KanteStyle.unit(8)
            height: KanteStyle.heightMedium
            KanteHud {
                anchors.verticalCenter: parent.verticalCenter
                fullText: texts.t("transpose.title")
                color: KanteStyle.mutedTextColor
            }
            KanteSpinBox {
                anchors.verticalCenter: parent.verticalCenter
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
                anchors.verticalCenter: parent.verticalCenter
                term: texts.t("transpose.title")
                info: texts.t("transpose.hint")
                accessibleName: texts.t("help.about", { term: texts.t("transpose.title") })
            }
            }
            // Aim against the beat (LH-BEW-08, DIDAKTIK I5): the summary says if the run held it.
            Row {
                readonly property var pockets: ["free", "onbeat", "laidback", "pushed"]
                spacing: KanteStyle.unit(8)
                height: KanteStyle.heightMedium
                KanteHud {
                    anchors.verticalCenter: parent.verticalCenter
                    fullText: texts.t("pocket.title")
                    color: KanteStyle.mutedTextColor
                }
                KanteComboBox {
                    anchors.verticalCenter: parent.verticalCenter
                    width: KanteStyle.unit(170)
                    Accessible.name: texts.t("pocket.title")
                    model: parent.pockets.map(function(p) { return texts.t("pocket." + p) })
                    currentIndex: Math.max(0, parent.pockets.indexOf(ctl.pocket))
                    onActivated: function(index) { ctl.changePocket(parent.pockets[index]) }
                }
                KanteInfoTip {
                    anchors.verticalCenter: parent.verticalCenter
                    term: texts.t("pocket.title")
                    info: texts.t("pocket.hint")
                    accessibleName: texts.t("help.about", { term: texts.t("pocket.title") })
                }
            }
        }

        // Backing track (LH-WIE-01): chosen per piece, stretched with the speed.
        RowLayout {
            Layout.fillWidth: true
            visible: !page.presenting && ctl.backingName !== ""
            spacing: KanteStyle.unit(8)
            KanteHud { fullText: texts.t("backing.title"); color: KanteStyle.mutedTextColor }
            QQC2.Label {
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                text: ctl.backingName !== "" ? ctl.backingName : texts.t("backing.none")
                color: ctl.backingName !== "" ? KanteStyle.textColor : KanteStyle.mutedTextColor
            }
            KanteButton {
                text: texts.t("backing.other")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: backingDialog.open()
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

        }

        // Without the recording's bass (LH-BIB-09, round 9): Demucs does it once, if installed;
        // without it "without bass" is greyed out and the tip says how to install it. While it
        // runs: progress, time left, cancel; once separated: switch at once, or delete the file.
        RowLayout {
            id: stemsRow
            Layout.fillWidth: true
            visible: !page.presenting && page.stems.state !== "none"
            spacing: KanteStyle.unit(8)
            readonly property bool running: page.stems.state === "running"
            readonly property int chosen: page.stems.state === "on" || running ? 1 : 0
            KanteHud { fullText: texts.t("stems.title"); color: KanteStyle.mutedTextColor }
            KanteSegmented {
                id: stemsChoice
                Accessible.name: texts.t("stems.title")
                model: [texts.t("stems.original"), texts.t("stems.without")]
                tooltips: [texts.t("stems.original.hint"),
                    page.stems.available ? texts.t("stems.without.hint") : texts.t("stems.without.missing")]
                // A song separated before plays without bass even without Demucs.
                disabledIndexes: page.stems.available || page.stems.cached ? [] : [1]
                currentIndex: stemsRow.chosen
                // A click replaces the binding; the state comes from Kontra (failed, busy).
                onActivated: function(index) {
                    if (stemsRow.running && index === 0)
                        ctl.cancelStems()
                    else
                        ctl.removeBass(index === 1)
                    currentIndex = Qt.binding(function() { return stemsRow.chosen })
                }
            }
            KanteProgressBar {
                visible: stemsRow.running
                Layout.preferredWidth: KanteStyle.unit(120)
                value: page.stems.progress || 0
                // Before the first progress Demucs loads its model.
                indeterminate: value <= 0
                Accessible.role: Accessible.ProgressBar
                Accessible.name: texts.t("stems.progress")
            }
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: page.stems.text || ""
                color: page.stems.state === "failed" ? KanteStyle.negativeTextColor : KanteStyle.mutedTextColor
            }
            KanteButton {
                visible: stemsRow.running
                text: texts.t("stems.cancel")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: ctl.cancelStems()
            }
            KanteButton {
                visible: page.stems.cached === true && !stemsRow.running
                text: texts.t("stems.delete")
                emphasis: KanteButton.Emphasis.Quiet
                Accessible.description: page.stems.deleteText || ""
                QQC2.ToolTip.text: page.stems.deleteText || ""
                QQC2.ToolTip.visible: hovered
                QQC2.ToolTip.delay: 500
                onClicked: ctl.forgetStems()
            }
            KanteButton {
                visible: !page.stems.available
                text: texts.t("stems.retry")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: ctl.refreshStems()
            }
            // Failed: Demucs' words; without Demucs: how to install it; else where the files go.
            KanteInfoTip {
                readonly property bool failed: page.stems.state === "failed"
                term: failed ? texts.t("stems.why")
                    : page.stems.available ? texts.t("stems.where.title") : texts.t("stems.install.title")
                info: failed ? page.stems.detail || ""
                    : page.stems.available ? page.stems.cacheText || "" : page.stems.installText || ""
                accessibleName: term
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
            // The workshop always works on bars: without a chosen part it starts with the two at
            // the play position (QA 14.10.: "bars 1 to 1" showed, yet play ran the whole piece).
            function chooseLoop() {
                if (ctl.mode !== "workshop" || ctl.loopStart >= 0 || page.bars.length === 0)
                    return
                var bar = page.barAt(ctl.position)
                ctl.loopBars(bar, Math.min(bar + 1, page.bars.length - 1))
            }
            Component.onCompleted: chooseLoop()

            KanteHud { fullText: texts.t("workshop.bars"); color: KanteStyle.mutedTextColor }
            KanteSpinBox {
                id: fromSpin
                from: 1
                to: Math.max(1, page.bars.length)
                value: Math.max(1, workshopRow.fromBar)
                editable: true
                Accessible.name: texts.t("workshop.from")
                onValueModified: ctl.loopBars(value - 1, Math.max(value, toSpin.value) - 1)
                // The field passes Return on after taking the value; also when it was not valid.
                Keys.onReturnPressed: page.endField()
                Keys.onEnterPressed: page.endField()
                Keys.onEscapePressed: page.cancelField(fromSpin)
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
                // The field passes Return on after taking the value; also when it was not valid.
                Keys.onReturnPressed: page.endField()
                Keys.onEnterPressed: page.endField()
                Keys.onEscapePressed: page.cancelField(toSpin)
            }
            QQC2.Label {
                // Fills the row: tied to its own width it wrapped "Tempo 100 / %" after the
                // text grew (QA 14.10.).
                Layout.fillWidth: true
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
            // Blind probe (DIDAKTIK I1): the bars once without notes; the summary compares.
            KanteButton {
                visible: ctl.loopStart >= 0
                enabled: !ctl.playing
                text: texts.t("probe.start")
                emphasis: KanteButton.Emphasis.Quiet
                Accessible.description: texts.t("probe.start.hint")
                onClicked: ctl.startProbe()
            }
            KanteInfoTip {
                visible: ctl.loopStart >= 0
                term: texts.t("probe.start")
                accessibleName: texts.t("help.about", { term: texts.t("probe.start") })
                info: texts.t("probe.start.hint")
            }
            Item { Layout.fillWidth: true }
            Connections {
                target: ctl
                function onPassComplete(clean, hitRate) {
                    workshopRow.lastChange = hitRate > ctl.bandHigh ? texts.t("workshop.above")
                        : hitRate < ctl.bandLow ? texts.t("workshop.below") : texts.t("workshop.inside")
                }
                function onModeChanged() {
                    workshopRow.lastChange = ""
                    workshopRow.chooseLoop()
                }
                function onArrangementLoaded() { workshopRow.chooseLoop() }
            }
        }

        // Building blocks (DIDAKTIK I6, LH-MOD-08): the bars in blocks, joined step by step.
        Flow {
            id: chainRow
            Layout.fillWidth: true
            visible: ctl.mode === "workshop" && !page.presenting
            spacing: KanteStyle.unit(8)
            readonly property var chain: JSON.parse(ctl.chainJson)
            // Bars per block or phrases (cut at rests), as ctl.startChain takes them.
            readonly property var sizes: ["off", "2", "4", "phrases"]
            // Blocks running now; choosing them again keeps the step reached.
            property string size: "off"
            readonly property string order: chainBackward.checked ? "backward" : "forward"
            function start(size) {
                chainRow.size = size
                ctl.startChain(size, chainRow.order)
            }
            Row {
                spacing: KanteStyle.unit(8)
                height: KanteStyle.heightMedium
                KanteHud {
                    anchors.verticalCenter: parent.verticalCenter
                    fullText: texts.t("chain.title")
                    color: KanteStyle.mutedTextColor
                }
                KanteComboBox {
                    id: chainSize
                    anchors.verticalCenter: parent.verticalCenter
                    width: KanteStyle.unit(160)
                    Accessible.name: texts.t("chain.title")
                    model: [texts.t("chain.off"), texts.t("chain.two"), texts.t("chain.four"), texts.t("chain.phrases")]
                    onActivated: function(index) {
                        if (chainRow.sizes[index] === chainRow.size) {
                            return
                        }
                        chainRow.start(chainRow.sizes[index])
                    }
                }
                KanteSwitch {
                    id: chainBackward
                    anchors.verticalCenter: parent.verticalCenter
                    Accessible.name: texts.t("chain.backward")
                    onToggled: { if (chainRow.chain !== null) { chainRow.start(chainRow.sizes[chainSize.currentIndex]) } }
                }
                QQC2.Label { anchors.verticalCenter: parent.verticalCenter; text: texts.t("chain.backward"); color: KanteStyle.textColor }
                KanteInfoTip {
                    anchors.verticalCenter: parent.verticalCenter
                    term: texts.t("chain.title")
                    info: texts.t("chain.hint")
                    accessibleName: texts.t("help.about", { term: texts.t("chain.title") })
                }
            }
            QQC2.Label {
                visible: chainRow.chain !== null
                height: KanteStyle.heightMedium
                verticalAlignment: Text.AlignVCenter
                text: chainRow.chain !== null ? chainRow.chain.text : ""
                color: KanteStyle.textColor
            }
            // Bars chosen by hand end the blocks: the list says so.
            Connections {
                target: ctl
                function onChainJsonChanged() {
                    if (JSON.parse(ctl.chainJson) === null) {
                        chainSize.currentIndex = 0
                        chainRow.size = "off"
                    }
                }
            }
        }

        // Note density (LH-MOD-06): fewer notes while a part is new; per section, fixable. A Flow:
        // as one row its width pushed the whole page past a narrow window (QA 14.10., 700 px).
        Flow {
            id: densityRow
            Layout.fillWidth: true
            visible: (ctl.mode === "song" || ctl.mode === "workshop") && !page.presenting
            spacing: KanteStyle.unit(8)
            // In a run-through the part is the whole piece (QA 14.10.: "this section" without one).
            readonly property string fixedText: ctl.mode === "workshop" ? texts.t("density.fixed") : texts.t("density.fixed.piece")
            Row {
                spacing: KanteStyle.unit(8)
                KanteHud { anchors.verticalCenter: parent.verticalCenter; fullText: texts.t("density.title"); color: KanteStyle.mutedTextColor }
                KanteSegmented {
                    Accessible.name: texts.t("density.title")
                    model: [texts.t("density.sparse"), texts.t("density.beats"), texts.t("density.full")]
                    tooltips: [texts.t("density.hint"), texts.t("density.hint"), texts.t("density.hint")]
                    currentIndex: ctl.density - 1
                    onActivated: function(index) { ctl.changeDensity(index + 1, ctl.densityFixed ? "fixed" : "free") }
                }
            }
            Row {
                spacing: KanteStyle.unit(8)
                height: KanteStyle.heightMedium
                KanteSwitch {
                    anchors.verticalCenter: parent.verticalCenter
                    checked: ctl.densityFixed
                    Accessible.name: densityRow.fixedText
                    onToggled: ctl.changeDensity(ctl.density, checked ? "fixed" : "free")
                }
                QQC2.Label { anchors.verticalCenter: parent.verticalCenter; text: densityRow.fixedText; color: KanteStyle.textColor }
            }
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

        // Blind probe running: why the notes are gone (P1 would think they broke).
        KanteCallout {
            Layout.fillWidth: true
            visible: ctl.probing
            kind: KanteCallout.Kind.Info
            autoHide: false
            title: texts.t("probe.banner", { from: workshopRow.fromBar, to: workshopRow.toBar })
            onVisibleChanged: { if (visible) { Accessible.announce(title) } }
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
                notes: page.shown
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
                // A short piece (an exercise of 8 bars) needs fewer lines than fit: the staff grows
                // up to half again instead of filling the top half only (QA 14.10., P1 at 1280 px).
                readonly property real grow: {
                    var system = KanteStyle.unit(170) * page.presentScale
                    var perLine = Math.max(1, Math.floor((width - KanteStyle.unit(60)) / Math.max(1, minBarWidth)))
                    var lines = Math.max(1, Math.ceil(page.bars.length / perLine))
                    return Math.max(1, Math.min(1.5, height / (lines * system)))
                }
                // As many lines as fit (QA P2 R6: one line left the next bars hidden).
                systems: Math.max(1, Math.floor(height / (KanteStyle.unit(170) * page.presentScale * grow)))
                lineSpacing: KanteStyle.unit(14) * page.presentScale * grow
                minBarWidth: Math.max(KanteStyle.unit(200), page.densestBar * page.tabNoteWidth)
                visible: ctl.view === "tab"
                strings: page.stringNames.length
                stringNames: page.stringNames
                lowStringOnTop: ctl.lowStringOnTop
                notes: visible ? page.shown : []
                marks: page.marks
                onNotesChanged: Qt.callLater(function() { page.catchUp(tabStaff) })
                bars: page.bars
                position: ctl.position
                loopStart: ctl.loopStart
                loopEnd: ctl.loopEnd
            }
            KanteBassStaff {
                id: bassStaff
                anchors.fill: parent
                // Smaller where one line does not fit (beamer at 150 %): cut off, its beams ran
                // over the transport (G18). A KanteBassStaff line is 14.5 staff spaces, 16.5 with
                // chord symbols.
                // Two lines rather than one with room left under it, as long as their staff space
                // keeps three quarters of the wanted one (QA 14.10., presentation at the beamer:
                // two bars and a third of the screen empty).
                // A short piece grows up to half again while all its lines still fit (QA 14.10.);
                // a bar is 18 staff spaces wide, the clef and key about 8.
                staffSpace: {
                    var wanted = KanteStyle.unit(10) * page.presentScale
                    var lineSpaces = hasChords ? 16.5 : 14.5
                    for (var grown = wanted * 1.5; grown > wanted; grown -= wanted * 0.05) {
                        var perLine = Math.max(1, Math.floor((width - grown * 8) / (grown * 18)))
                        if (Math.ceil(page.bars.length / perLine) * lineSpaces * grown <= height) {
                            return grown
                        }
                    }
                    var line = height / lineSpaces
                    return Math.min(wanted, line / 2 >= wanted * 0.75 ? line / 2 : line)
                }
                // As many lines as fit, as in the tablature (QA 13.10.: one line, the rest empty).
                systems: Math.max(1, Math.floor(height / systemHeight))
                visible: ctl.view === "notation"
                notes: visible ? page.shownStaff : []
                rests: page.staff.rests
                keyFifths: page.staff.keyFifths
                marks: page.marks
                onNotesChanged: Qt.callLater(function() { page.catchUp(bassStaff) })
                bars: page.bars
                position: ctl.position
                loopStart: ctl.loopStart
                loopEnd: ctl.loopEnd
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
                    notes: combined.visible ? page.shownStaff : []
                    rests: page.staff.rests
                    keyFifths: page.staff.keyFifths
                    marks: page.marks
                    onNotesChanged: Qt.callLater(function() { page.catchUp(combinedBass) })
                    bars: page.bars
                    position: ctl.position
                    loopStart: ctl.loopStart
                    loopEnd: ctl.loopEnd
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
                    notes: combined.visible ? page.shown : []
                    marks: page.marks
                    onNotesChanged: Qt.callLater(function() { page.catchUp(combinedTab) })
                    bars: page.bars
                    position: ctl.position
                    loopStart: ctl.loopStart
                    loopEnd: ctl.loopEnd
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
                    notes: keysView.visible ? page.shown : []
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
            // Only as high as its content: a nested layout fills by default, and in the keyboard
            // view (no fretboard) it took the notes' room (QA 13.10.).
            Layout.fillHeight: false
            visible: !page.compact && !page.presenting
            spacing: KanteStyle.unit(16)
            ColumnLayout {
            Layout.fillWidth: true
            // The keyboard view has its own keys; a fretboard beside them would only distract.
            visible: ctl.view !== "keys" && !page.slim
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
            // The signs of this piece, each with its "?" (LH-LRN-09): lane and notes show them
            // while playing, when a click would come too late.
            Flow {
                Layout.fillWidth: true
                visible: page.signs.length > 0 || page.hasChordSymbols
                spacing: KanteStyle.unit(16)
                Repeater {
                    model: page.signs
                    delegate: Row {
                        required property string modelData
                        visible: signTip.entry !== null
                        spacing: KanteStyle.unit(4)
                        KanteTechniqueMark {
                            anchors.verticalCenter: parent.verticalCenter
                            height: KanteStyle.unit(18)
                            width: implicitWidth
                            technique: modelData
                        }
                        QQC2.Label {
                            anchors.verticalCenter: parent.verticalCenter
                            text: signTip.term
                            color: KanteStyle.mutedTextColor
                        }
                        TermTip {
                            id: signTip
                            anchors.verticalCenter: parent.verticalCenter
                            topic: "technique." + modelData
                        }
                    }
                }
                Row {
                    visible: page.hasChordSymbols && chordTip.entry !== null
                    spacing: KanteStyle.unit(4)
                    QQC2.Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: chordTip.term
                        color: KanteStyle.mutedTextColor
                    }
                    TermTip {
                        id: chordTip
                        anchors.verticalCenter: parent.verticalCenter
                        topic: "lane.chord"
                    }
                }
            }
            }
            ColumnLayout {
            Layout.preferredWidth: meter.implicitWidth
            // Alone in the row (keyboard view, slim window): the whole width.
            Layout.fillWidth: ctl.view === "keys" || page.slim
            // Kontra listens only while playing; before, "−∞ dB · too quiet" read as a fault
            // (QA 14.10.). The meter's place keeps a line saying when it shows.
            // A MIDI keyboard has no level (P5): a line instead of "−∞ dB · too quiet".
            readonly property bool midi: ctl.connection === "midi"
            QQC2.Label {
                visible: !ctl.playing || parent.midi
                Layout.fillWidth: true
                Layout.preferredHeight: meter.implicitHeight
                verticalAlignment: Text.AlignVCenter
                wrapMode: Text.WordWrap
                // No keyboard plugged in: say so instead of "Kontra reads the keys" (QA 27.10., P5).
                text: !parent.midi ? texts.t("level.idle")
                    : ctl.midiPortName === "" ? texts.t("level.midi.none") : texts.t("level.midi")
                color: KanteStyle.mutedTextColor
            }
            KanteLevelMeter {
                id: meter
                visible: ctl.playing && !parent.midi
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
                listening: ctl.playing && ctl.connection !== "midi"
                onUseLaptopJack: ctl.changeConnection("laptop")
            }
            }
        }

        // The mixer opens above the transport (in the page, so screenshots show it). At most
        // 40 % of the page, the rest scrolls: open with tone shaping it squeezed the notes to a
        // strip (QA 14.10.).
        KanteCard {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(pageMixer.implicitHeight + KanteStyle.unit(16), page.height * 0.4)
            visible: mixerButton.checked
            barColor: KanteStyle.accentColor
            borderColor: KanteStyle.frameColor
            Flickable {
                id: mixerFlick
                x: KanteStyle.unit(12)
                y: KanteStyle.unit(8)
                width: parent.width - KanteStyle.unit(24)
                height: parent.height - KanteStyle.unit(16)
                contentWidth: width
                contentHeight: pageMixer.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                QQC2.ScrollBar.vertical: QQC2.ScrollBar { id: mixerBar; policy: mixerFlick.contentHeight > mixerFlick.height ? QQC2.ScrollBar.AlwaysOn : QQC2.ScrollBar.AsNeeded }
                // Tab into a row below the fold scrolls the whole row into view (P7, keyboard only).
                Window.onActiveFocusItemChanged: {
                    let item = Window.activeFocusItem
                    while (item && item.parent !== pageMixer) {
                        item = item.parent
                    }
                    if (!item) {
                        return
                    }
                    const top = item.y
                    if (top < contentY) {
                        contentY = top
                    } else if (top + item.height > contentY + height) {
                        contentY = Math.min(top + item.height - height, contentHeight - height)
                    }
                }
                Mixer {
                    id: pageMixer
                    practice: true
                    width: mixerFlick.width - (mixerFlick.contentHeight > mixerFlick.height ? mixerBar.width + KanteStyle.unit(8) : 0)
                }
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
        title: texts.t("dialog.backing")
        // Recordings of songs usually live in Music (QA 14.10.: it opened in the working folder).
        currentFolder: StandardPaths.writableLocation(StandardPaths.MusicLocation)
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
