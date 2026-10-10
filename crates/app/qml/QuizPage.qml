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
    property var question: null
    property string heard: ""
    // A note counts once it was heard the same for a few tuner updates (no passing noise).
    property int steadyMidi: -1
    property int steadyCount: 0
    readonly property var stringNames: JSON.parse(ctl.tuningJson).map(function(s) { return s.name })

    function start() {
        question = JSON.parse(ctl.quizStart(kinds[kind]))
        heard = ""
    }
    function next() {
        question = JSON.parse(ctl.quizNext())
        heard = ""
        steadyMidi = -1
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("quiz.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: { ctl.startTuner(); start() }
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

        KanteFretboard {
            Layout.fillWidth: true
            Layout.preferredHeight: KanteStyle.unit(160)
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
