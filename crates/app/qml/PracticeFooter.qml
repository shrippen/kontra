import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Under the notes on the practice page: fretboard, level and its advice, hand position, signs.
RowLayout {
    id: footer

    /// The practice page: its next note, hand position, signs and room.
    required property var practice
    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts

    // Only as high as its content: a nested layout fills by default, and in the keyboard
    // view (no fretboard) it took the notes' room (QA 13.10.).
    Layout.fillHeight: false
    // Level and its advice stay at any height, the notes shrink (QA 27.10., S12: P7 at
    // 150 % found neither); a low window leaves out the fretboard only. Its height does not
    // follow play, level or hand: the notes stay put (owner 12.11.).
    visible: !practice.presenting
    spacing: KanteStyle.unit(16)
    ColumnLayout {
        Layout.fillWidth: true
        // The keyboard view has its own keys; a fretboard beside them would only distract.
        visible: practice.fretboardRoom && ctl.fretboardShown
        spacing: KanteStyle.unit(4)
        KanteFretboard {
            Layout.fillWidth: true
            Layout.preferredHeight: KanteStyle.unit(120)
            strings: practice.stringNames.length
            stringNames: practice.stringNames
            frets: 15
            lowStringOnTop: ctl.lowStringOnTop
            leftHanded: ctl.leftHanded
            // The finger the position suggests is written in the marker (1 index … 4 pinky).
            markers: practice.nextNote ? [{ string: practice.nextNote.string, fret: practice.nextNote.fret, role: "next",
                    label: practice.nextNote.finger > 0 ? String(practice.nextNote.finger) : "" }] : []
            accessibleName: texts.t("fretboard.name")
            markerText: texts.t("fretboard.marker")
        }
    }
    GridLayout {
        columns: practice.fretboardRoom && !ctl.fretboardShown ? 5 : 1; columnSpacing: KanteStyle.unit(16)
        Layout.preferredWidth: meter.implicitWidth
        // Beside the fretboard a column; alone in the row (keys view, slim or low window) the
        // whole width; with the fretboard hidden one row of the whole width (owner 12.11.).
        Layout.fillWidth: !practice.fretboardRoom || !ctl.fretboardShown
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
            brief: practice.compact
            fit: LevelAdvice.Fit.Steady
            onUseLaptopJack: ctl.changeConnection("laptop")
        }
        // Room for two rows of pills at any time; the shift and the finger come and go.
        Flow {
            Layout.fillWidth: true
            Layout.preferredHeight: positionPill.implicitHeight * 2 + spacing
            clip: true
            spacing: KanteStyle.unit(8)
            visible: practice.fretboardRoom
            KantePill {
                id: positionPill
                visible: practice.handPosition > 0
                status: KantePill.State.Off
                text: texts.t("fretboard.position", { position: practice.handPosition })
                Accessible.name: text
            }
            KantePill {
                id: shiftPill
                visible: practice.shift !== null
                status: KantePill.State.Locked
                text: practice.shift ? texts.t("fretboard.shift", { position: practice.shift.position, bar: practice.shift.bar }) : ""
                Accessible.name: text
                onTextChanged: if (text !== "") Accessible.announce(text)
            }
            QQC2.Label {
                visible: practice.nextNote !== null && practice.nextNote.finger > 0
                text: practice.nextNote ? texts.t("fretboard.finger", { finger: practice.nextNote.finger }) : ""
                color: KanteStyle.mutedTextColor
            }
        }
        // The signs of this piece, each with its "?" (LH-LRN-09): lane and notes show them
        // while playing, when a click would come too late.
        Flow {
            Layout.fillWidth: true
            visible: practice.fretboardRoom && (practice.signs.length > 0 || practice.hasChordSymbols)
            spacing: KanteStyle.unit(16)
            Repeater {
                model: practice.signs
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
                visible: practice.hasChordSymbols && chordTip.entry !== null
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
        // Hides the fretboard: the notes take its room (owner 12.11.); remembered.
        KanteButton {
            visible: practice.fretboardRoom
            emphasis: KanteButton.Emphasis.Quiet
            text: ctl.fretboardShown ? texts.t("fretboard.hide") : texts.t("fretboard.show")
            onClicked: ctl.changeFretboard(!ctl.fretboardShown)
        }
    }
}
