import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Ear training (LH-LRN-03): Kontra plays, the player answers on the instrument. Three kinds:
// play a note or short line back, the second note of an interval, a chord's root. Every played
// note gets a mark (check or cross, not only colour); a solved question names what it was and
// shows where it lies, then the next one plays by itself.
Kirigami.Page {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var kinds: ["echo", "interval", "chord"]
    property int kind: 0
    readonly property var round: ctl.earState !== "" ? JSON.parse(ctl.earState) : null
    readonly property bool solved: round !== null && round.solved
    readonly property var stringNames: JSON.parse(ctl.tuningJson).map(function(s) { return s.name })

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("ear.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: ctl.earStart(kinds[kind])
    Component.onDestruction: ctl.earStop()

    onSolvedChanged: {
        if (!solved)
            return
        prompt.Accessible.announce(round.reveal)
        advance.restart()
    }
    Timer { id: advance; interval: 2500; onTriggered: ctl.earNext() }

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("ear.title"); pageTitle: true }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            color: KanteStyle.textColor
            text: texts.t("ear.hint")
        }
        KanteSegmented {
            Accessible.name: texts.t("ear.title")
            model: page.kinds.map(function(k) { return texts.t("ear.kind." + k) })
            currentIndex: page.kind
            onActivated: function(index) { page.kind = index; advance.stop(); ctl.earStart(page.kinds[index]) }
        }

        KanteCallout {
            Layout.fillWidth: true
            visible: page.round !== null && page.round.silent
            kind: KanteCallout.Kind.Warn
            autoHide: false
            title: texts.t("ear.silent")
        }

        KanteCallout {
            id: prompt
            Layout.fillWidth: true
            kind: page.solved ? KanteCallout.Kind.Ok : KanteCallout.Kind.Info
            autoHide: false
            title: page.round ? page.round.prompt : ""
            text: !page.round ? ""
                : page.solved ? page.round.reveal
                : page.round.prompting ? texts.t("ear.prompting")
                : page.round.total > 1 ? texts.t("ear.progress", { done: page.round.progress, total: page.round.total })
                : texts.t("ear.your.turn")
            Accessible.name: title
            RowLayout {
                spacing: KanteStyle.unit(8)
                KanteButton {
                    text: texts.t("ear.again")
                    Accessible.name: texts.t("ear.again")
                    enabled: page.round !== null && !page.round.prompting
                    onClicked: ctl.earPlay()
                }
                KanteButton {
                    text: texts.t("quiz.next")
                    Accessible.name: texts.t("quiz.next")
                    onClicked: { advance.stop(); ctl.earNext() }
                }
            }
        }

        // The notes played for this question, each with its mark and name.
        Flow {
            Layout.fillWidth: true
            spacing: KanteStyle.unit(12)
            visible: page.round !== null && page.round.played.length > 0
            Accessible.name: texts.t("ear.played")
            Repeater {
                model: page.round ? page.round.played : []
                delegate: Row {
                    required property var modelData
                    spacing: KanteStyle.unit(4)
                    KanteNoteMark {
                        anchors.verticalCenter: parent.verticalCenter
                        noteState: modelData.right ? "hit" : "wrong"
                    }
                    // A wrong note says which way to go: "C2 too high".
                    QQC2.Label {
                        readonly property string direction: modelData.right ? "" : modelData.direction
                        color: KanteStyle.textColor
                        text: direction === "" ? modelData.note
                            : texts.t("ear.too." + direction, { note: modelData.note })
                        Accessible.name: direction !== "" ? text
                            : texts.t(modelData.right ? "ear.played.right" : "ear.played.wrong", { note: modelData.note })
                    }
                }
            }
        }

        // Prompt volume (round 8: its own slider, also in the mixer).
        KanteSettingRow {
            Layout.fillWidth: true
            title: texts.t("mixer.prompt")
            RowLayout {
                spacing: KanteStyle.unit(8)
                QQC2.Slider {
                    id: promptSlider
                    Layout.preferredWidth: KanteStyle.unit(220)
                    from: 0
                    to: 1
                    stepSize: 0.05
                    value: ctl.promptVolume
                    focusPolicy: Qt.StrongFocus
                    Accessible.name: texts.t("mixer.prompt")
                    onMoved: ctl.changeVolume("prompt", value)
                    KanteSliderSkin { control: promptSlider }
                }
                QQC2.Label {
                    color: KanteStyle.textColor
                    text: texts.t("mixer.percent", { percent: Math.round(promptSlider.value * 100) })
                }
            }
        }

        KanteFretboard {
            Layout.fillWidth: true
            Layout.preferredHeight: KanteStyle.unit(160)
            strings: page.stringNames.length
            stringNames: page.stringNames
            frets: 15
            lowStringOnTop: ctl.lowStringOnTop
            markers: page.round ? page.round.positions : []
            accessibleName: texts.t("fretboard.name")
            markerText: texts.t("fretboard.marker")
        }

        QQC2.Label {
            color: KanteStyle.strongTextColor
            font.pointSize: KanteStyle.defaultFont.pointSize * 1.3
            text: page.round ? page.round.score : ""
            Accessible.name: text
        }

        Item { Layout.fillHeight: true }
    }
}
