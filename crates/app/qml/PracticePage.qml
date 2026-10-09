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
    readonly property var views: ["lane", "tab", "notation", "combined"]

    // The notes as loaded; live states go to the views one by one (setNoteState) and are kept
    // here too, so a view that gets its notes later (view switch) can catch up.
    property var notes: []
    property var bars: JSON.parse(ctl.barsJson)
    // The notation view: the same notes as written for bass, rests, key signature.
    property var staff: JSON.parse(ctl.staffJson)
    property var states: ({})
    // Play mode marks quietly or not at all (setting); the other modes mark fully.
    readonly property int marks: ctl.mode !== "play" || ctl.playMarks === "full" ? KanteTabLane.Marks.Full
                               : ctl.playMarks === "off" ? KanteTabLane.Marks.Off : KanteTabLane.Marks.Quiet
    readonly property var noteViews: [lane, tabStaff, bassStaff, combinedBass, combinedTab]

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: ctl.title
    padding: KanteStyle.unit(12)

    property bool summaryReady: false

    function reset() {
        summaryReady = false
        notes = JSON.parse(ctl.notesJson)
        bars = JSON.parse(ctl.barsJson)
        staff = JSON.parse(ctl.staffJson)
        states = {}
    }

    // Re-applies the live states after `view` got a new notes list (which clears them).
    // Called through a fresh closure: Qt.callLater merges calls of the same function.
    function catchUp(view) {
        for (var key in states) {
            view.setNoteState(Number(key), states[key])
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

    Component.onCompleted: reset()

    // Keyboard (LH-NF-07, QA P7/P2): the transport keys work on the whole page, not only with
    // focus in the transport bar; modes 1–4, views Alt+1–4. Not while typing in a field.
    readonly property bool keysFree: {
        var item = applicationWindow().activeFocusItem
        return !(item && item.hasOwnProperty("cursorPosition"))
    }
    Shortcut { sequence: "K"; enabled: page.keysFree; onActivated: ctl.togglePlay() }
    Shortcut { sequence: "N"; enabled: page.keysFree && ctl.nextTitle !== ""; onActivated: ctl.openNext() }
    Shortcut { sequence: "Home"; enabled: page.keysFree; onActivated: ctl.rewind() }
    Shortcut { sequence: "L"; enabled: page.keysFree; onActivated: transport.loopToggled() }
    Shortcut { sequence: "M"; enabled: page.keysFree; onActivated: ctl.changeOptions(!ctl.metronome, ctl.reference, ctl.lowStringOnTop) }
    Shortcut { sequence: "C"; enabled: page.keysFree; onActivated: ctl.changeCountIn(!ctl.countIn) }
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
            for (var i = 0; i < page.noteViews.length; i++) {
                page.noteViews[i].setNoteState(index, state)
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
    readonly property var stringNames: JSON.parse(ctl.stringNamesJson)
    readonly property int nextIndex: ctl.holdingIndex >= 0 ? ctl.holdingIndex : lane.nextIndex
    readonly property var nextNote: nextIndex >= 0 && nextIndex < notes.length ? notes[nextIndex] : null

    // Small screens and beamers (QA P4: 1280×720 at 150 %): the title gets its own row and the
    // fretboard and level make room for the notes.
    readonly property bool narrow: width < KanteStyle.unit(1000)
    readonly property bool compact: height < KanteStyle.unit(560)

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
                    visible: tracks.names.length > 1
                    QQC2.Label { text: texts.t("track.label"); color: KanteStyle.mutedTextColor }
                    QQC2.ComboBox {
                        Layout.preferredWidth: KanteStyle.unit(220)
                        Accessible.name: texts.t("track.label")
                        model: parent.tracks.names
                        currentIndex: parent.tracks.current
                        onActivated: function(index) { ctl.switchTrack(index) }
                    }
                }
            }
            KanteSegmented {
                model: page.modes.map(function(m) { return texts.t("mode." + m) })
                tooltips: page.modes.map(function(m) { return texts.t("mode." + m + ".hint") })
                currentIndex: Math.max(0, page.modes.indexOf(ctl.mode))
                onActivated: function(index) { ctl.changeMode(page.modes[index]) }
            }
            KanteSegmented {
                model: page.views.map(function(v) { return texts.t("view." + v) })
                currentIndex: Math.max(0, page.views.indexOf(ctl.view))
                onActivated: function(index) { ctl.changeView(page.views[index]) }
            }
        }

        // Backing track (LH-WIE-01): chosen per piece, stretched with the speed.
        RowLayout {
            Layout.fillWidth: true
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
                text: texts.t("backing.remove")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: ctl.chooseBacking("")
            }
        }

        // Workshop (LH-MOD-04): which bars, and how the tempo follows the hit rate (target band).
        RowLayout {
            id: workshopRow
            Layout.fillWidth: true
            visible: ctl.mode === "workshop"
            spacing: KanteStyle.unit(8)
            readonly property int fromBar: ctl.loopStart >= 0 ? page.barAt(ctl.loopStart) + 1 : 0
            readonly property int toBar: ctl.loopEnd >= 0 ? page.barAt(ctl.loopEnd - 0.001) + 1 : 0
            property string lastChange: ""

            KanteHud { fullText: texts.t("workshop.bars"); color: KanteStyle.mutedTextColor }
            QQC2.SpinBox {
                id: fromSpin
                from: 1
                to: Math.max(1, page.bars.length)
                value: Math.max(1, workshopRow.fromBar)
                editable: true
                Accessible.name: texts.t("workshop.from")
                onValueModified: ctl.loopBars(value - 1, Math.max(value, toSpin.value) - 1)
            }
            QQC2.Label { text: texts.t("workshop.to"); color: KanteStyle.textColor }
            QQC2.SpinBox {
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
            Connections {
                target: ctl
                function onPassComplete(clean, hitRate) {
                    workshopRow.lastChange = hitRate > ctl.bandHigh ? texts.t("workshop.above")
                        : hitRate < ctl.bandLow ? texts.t("workshop.below") : texts.t("workshop.inside")
                }
                function onModeChanged() { workshopRow.lastChange = "" }
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

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

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
                systems: Math.max(1, Math.floor(height / KanteStyle.unit(170)))
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
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !page.compact
            spacing: KanteStyle.unit(16)
            KanteFretboard {
                Layout.fillWidth: true
                Layout.preferredHeight: KanteStyle.unit(120)
                strings: page.stringNames.length
                stringNames: page.stringNames
                frets: 15
                lowStringOnTop: ctl.lowStringOnTop
                markers: page.nextNote ? [{ string: page.nextNote.string, fret: page.nextNote.fret, role: "next" }] : []
                accessibleName: texts.t("fretboard.name")
                markerText: texts.t("fretboard.marker")
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
                peakDb: ctl.peakDb
                clipped: ctl.clipped
                listening: ctl.playing
                laptopJack: ctl.laptopJack
                onUseLaptopJack: ctl.changeInstrument(ctl.strings, ctl.fretless, true)
            }
            }
        }

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
