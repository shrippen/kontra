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
    readonly property var views: ["lane", "tab", "notation"]

    // Notes with their live state; judged states are collected and applied ten times a second,
    // because the views re-read the whole list on every assignment.
    property var notes: []
    property var bars: JSON.parse(ctl.barsJson)
    property var pendingStates: ({})
    property bool dirty: false

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: ctl.title
    padding: KanteStyle.unit(12)

    function reset() {
        notes = JSON.parse(ctl.notesJson)
        bars = JSON.parse(ctl.barsJson)
        pendingStates = {}
        dirty = false
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

    Connections {
        target: ctl
        function onArrangementLoaded() { page.reset() }
        function onNoteJudged(index, state, offsetMs, cents) {
            page.pendingStates[index] = state
            page.dirty = true
        }
        function onFinished() { summary.open() }
    }

    Timer {
        interval: 100
        repeat: true
        running: page.dirty
        onTriggered: {
            var next = page.notes.slice()
            for (var key in page.pendingStates) {
                var i = Number(key)
                if (next[i]) {
                    next[i] = Object.assign({}, next[i], { state: page.pendingStates[key] })
                }
            }
            page.pendingStates = {}
            page.dirty = false
            page.notes = next
        }
    }

    readonly property var times: bars.map(function(b) { return b.time })
    readonly property var stringNames: JSON.parse(ctl.stringNamesJson)
    readonly property int nextIndex: ctl.holdingIndex >= 0 ? ctl.holdingIndex : lane.nextIndex
    readonly property var nextNote: nextIndex >= 0 && nextIndex < notes.length ? notes[nextIndex] : null

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(10)

        RowLayout {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(12)
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                KanteHeading { text: ctl.title; pageTitle: true; Layout.fillWidth: true; elide: Text.ElideRight }
                QQC2.Label { text: ctl.artist; visible: ctl.artist !== ""; color: KanteStyle.mutedTextColor }
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
            KanteButton {
                visible: ctl.backingName !== ""
                text: texts.t("backing.remove")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: ctl.chooseBacking("")
            }
        }

        // Wait mode: what to play next, in words (LH-MOD-03).
        KanteBanner {
            Layout.fillWidth: true
            visible: ctl.holdingIndex >= 0
            title: ctl.holdingText
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
                bars: page.times
                position: ctl.position
                // Wide enough that eighths at 160 BPM do not overlap: about two note heads per beat.
                pixelsPerSecond: Math.max(KanteStyle.unit(320), height * 1.6) * Math.max(0.5, ctl.speed)
                accessibleName: texts.t("lane.name")
                loopStart: ctl.loopStart
                loopEnd: ctl.loopEnd
            }
            KanteTabStaff {
                anchors.fill: parent
                visible: ctl.view === "tab"
                strings: page.stringNames.length
                stringNames: page.stringNames
                lowStringOnTop: ctl.lowStringOnTop
                notes: visible ? page.notes : []
                bars: page.bars
                position: ctl.position
            }
            KanteBassStaff {
                anchors.fill: parent
                visible: ctl.view === "notation"
                notes: visible ? page.notes : []
                bars: page.bars
                position: ctl.position
            }
        }

        RowLayout {
            Layout.fillWidth: true
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
            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                visible: meter.zone !== "good"
                text: texts.t("level." + meter.zone + ".hint")
                color: KanteStyle.mutedTextColor
            }
            }
        }

        KanteTransportBar {
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
        nameFilters: ["Audio (*.wav *.flac *.ogg *.mp3)"]
        onAccepted: ctl.chooseBacking(selectedFile.toString())
    }

    SummarySheet {
        id: summary
        anchors.centerIn: parent
        width: Math.min(page.width - KanteStyle.unit(40), KanteStyle.unit(640))
        height: contentHeight
        z: 5
        onPracticeBar: function(bar) {
            ctl.changeMode("workshop")
            ctl.loopBars(bar, bar)
            summary.close()
        }
    }
}
