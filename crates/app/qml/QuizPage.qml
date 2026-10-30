import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Fretboard quiz with the real instrument (LH-LRN-02): Kontra asks in words, the tuner hears the
// answer. A right note shows every place it lies; then the next question follows by itself.
Kirigami.Page {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var kinds: ["notes", "intervals"]
    property int kind: 0
    // Frets asked for notes (LH-LRN-18): presets 0–4, 0–7, 0–12, or an own range.
    readonly property var presets: [[0, 4], [0, 7], [0, 12]]
    readonly property int topFret: 12
    property int low: 0
    property int high: 4
    property bool custom: false
    readonly property int preset: custom ? presets.length : presets.findIndex(function(r) {
        return r[0] === low && r[1] === high })
    property var question: null
    property string heard: ""
    // A note counts once it was heard the same for a few tuner updates (no passing noise).
    property int steadyMidi: -1
    property int steadyCount: 0
    readonly property var stringNames: JSON.parse(ctl.tuningJson).map(function(s) { return s.name })

    function start() {
        question = JSON.parse(ctl.quizStart(kinds[kind], low, high))
        low = question.range.low
        high = question.range.high
        heard = ""
    }
    function chooseRange(index) {
        custom = index === presets.length
        if (!custom) {
            low = presets[index][0]
            high = presets[index][1]
        }
        start()
    }
    function next() {
        question = JSON.parse(ctl.quizNext())
        heard = ""
        steadyMidi = -1
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("quiz.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: {
        var stored = JSON.parse(ctl.quizJson()).range
        low = stored.low
        high = stored.high
        custom = preset < 0
        ctl.startTuner()
        start()
    }
    Component.onDestruction: ctl.stopTuner()

    Connections {
        target: ctl
        function onTunerMidiChanged() {
            if (ctl.tunerNote === "" || !page.question || page.question.solved)
                return
            var midi = Math.round(ctl.tunerMidi)
            page.heard = ctl.tunerNote
            if (midi === page.steadyMidi) {
                page.steadyCount++
            } else {
                page.steadyMidi = midi
                page.steadyCount = 1
            }
            if (page.steadyCount === 3 && ctl.quizAnswer(ctl.tunerMidi)) {
                page.question = JSON.parse(ctl.quizJson())
                prompt.Accessible.announce(texts.t("quiz.right"))
                advance.restart()
            }
        }
    }
    Timer { id: advance; interval: 2500; onTriggered: page.next() }

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("quiz.title"); pageTitle: true }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t("quiz.hint")
        }
        KanteSegmented {
            Accessible.name: texts.t("quiz.title")
            model: page.kinds.map(function(k) { return texts.t("quiz.kind." + k) })
            currentIndex: page.kind
            onActivated: function(index) { page.kind = index; page.start() }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.kinds[page.kind] === "notes"
            title: texts.t("quiz.range")
            hint: texts.t("quiz.range.hint")
            KanteSegmented {
                Accessible.name: texts.t("quiz.range")
                model: page.presets.map(function(r) { return texts.t("quiz.range.frets", { low: r[0], high: r[1] }) })
                    .concat([texts.t("quiz.range.custom")])
                currentIndex: page.preset
                onActivated: function(index) { page.chooseRange(index) }
            }
        }
        KanteSettingRow {
            Layout.fillWidth: true
            visible: page.kinds[page.kind] === "notes" && page.custom
            title: texts.t("quiz.range.own")
            RowLayout {
                spacing: KanteStyle.unit(8)
                QQC2.Label { color: KanteStyle.textColor; text: texts.t("quiz.range.from") }
                KanteSpinBox {
                    Accessible.name: texts.t("quiz.range.from")
                    from: 0
                    to: page.topFret
                    value: page.low
                    onValueModified: { page.low = value; page.high = Math.max(page.high, value); page.start() }
                }
                QQC2.Label { color: KanteStyle.textColor; text: texts.t("quiz.range.to") }
                KanteSpinBox {
                    Accessible.name: texts.t("quiz.range.to")
                    from: 0
                    to: page.topFret
                    value: page.high
                    onValueModified: { page.high = value; page.low = Math.min(page.low, value); page.start() }
                }
            }
        }

        KanteCallout {
            id: prompt
            Layout.fillWidth: true
            kind: page.question && page.question.solved ? KanteCallout.Kind.Ok : KanteCallout.Kind.Info
            autoHide: false
            title: page.question ? page.question.prompt : ""
            text: page.question && page.question.solved ? texts.t("quiz.right")
                : page.heard !== "" ? texts.t("quiz.played", { note: page.heard }) : texts.t("quiz.listening")
            Accessible.name: title
            KanteButton {
                text: texts.t("quiz.next")
                Accessible.name: texts.t("quiz.next")
                onClicked: { advance.stop(); page.next() }
            }
        }

        // The fretboard is the answer here: it grows with the width (a fifth of it, at most
        // 240), so the half page below it is not empty and the marks read from further away;
        // on a short screen (beamer) it shrinks, so the score below stays visible.
        KanteFretboard {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: KanteStyle.unit(96)
            Layout.preferredHeight: Layout.maximumHeight
            Layout.maximumHeight: Math.max(KanteStyle.unit(160), Math.min(KanteStyle.unit(240), width / 5))
            strings: page.stringNames.length
            stringNames: page.stringNames
            frets: 15
            lowStringOnTop: ctl.lowStringOnTop
            markers: page.question ? page.question.positions : []
            accessibleName: texts.t("fretboard.name")
            markerText: texts.t("fretboard.marker")
        }

        QQC2.Label {
            color: KanteStyle.strongTextColor
            font.pointSize: KanteStyle.defaultFont.pointSize * 1.3
            text: page.question ? page.question.score : ""
            Accessible.name: text
        }

        Item { Layout.fillHeight: true }
    }
}
