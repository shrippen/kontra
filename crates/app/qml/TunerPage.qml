import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Tuner (LH-TUN-01/03): note, cents and what to do, in words and symbol.
Kirigami.Page {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property bool hearing: ctl.tunerNote !== ""
    readonly property string noteName: hearing ? ctl.tunerNote.replace(/-?[0-9]+$/, "") : ""
    readonly property int octave: hearing ? Number(ctl.tunerNote.match(/-?[0-9]+$/)) : 0

    // Guided tuning (LH-TUN-03, J-A2): string by string, lowest first, in words and symbol.
    property bool guided: true
    readonly property var strings: JSON.parse(ctl.tuningJson)
    property int target: 0
    property var done: []
    property real inTuneMs: 0
    readonly property bool allDone: strings.length > 0 && done.length === strings.length
    readonly property var targetString: target < strings.length ? strings[target] : null
    // Cents from the target string's open note; far off means another string is ringing.
    readonly property real targetCents: hearing && targetString ? (ctl.tunerMidi - targetString.midi) * 100 : 0
    readonly property bool wrongString: hearing && Math.abs(targetCents) > 250
    // The string that is ringing (nearest open string), for the chromatic view (LH-TUN-01).
    // By index: drop D has two strings named D (LH-TUN-02).
    readonly property int ringingIndex: {
        if (!hearing || strings.length === 0) return -1
        var best = 0
        for (var i = 1; i < strings.length; i++)
            if (Math.abs(strings[i].midi - ctl.tunerMidi) < Math.abs(strings[best].midi - ctl.tunerMidi)) best = i
        return Math.abs(strings[best].midi - ctl.tunerMidi) < 1.5 ? best : -1
    }
    readonly property string ringing: ringingIndex >= 0 ? strings[ringingIndex].name : ""

    function restart() { target = 0; done = []; inTuneMs = 0 }
    onStringsChanged: restart()

    Timer {
        interval: 100
        repeat: true
        running: page.guided && !page.allDone
        onTriggered: {
            // A beginner plucks whatever string: follow it if it is one still to tune.
            if (page.wrongString && page.ringingIndex >= 0 && page.done.indexOf(page.ringingIndex) < 0) {
                page.target = page.ringingIndex
                page.inTuneMs = 0
                return
            }
            if (page.hearing && !page.wrongString && Math.abs(page.targetCents) <= 3) {
                page.inTuneMs += interval
                if (page.inTuneMs >= 1000) {
                    page.done = page.done.concat([page.target])
                    page.inTuneMs = 0
                    if (page.target + 1 < page.strings.length) page.target += 1
                }
            } else {
                page.inTuneMs = 0
            }
        }
    }

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("tuner.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: ctl.startTuner()
    Component.onDestruction: ctl.stopTuner()

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(16)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: texts.t("tuner.title"); pageTitle: true; Layout.fillWidth: true }
            KanteSegmented {
                Accessible.name: texts.t("tuner.mode")
                model: [texts.t("tuner.guided"), texts.t("tuner.chromatic")]
                currentIndex: page.guided ? 0 : 1
                onActivated: function(index) { page.guided = index === 0; page.restart() }
            }
        }

        // The strings in order; done ones carry a check, the current one a frame (not colour only).
        RowLayout {
            Layout.fillWidth: true
            visible: page.guided
            spacing: KanteStyle.unit(8)
            Repeater {
                model: page.strings
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    readonly property bool isDone: page.done.indexOf(index) >= 0
                    readonly property bool isTarget: index === page.target && !page.allDone
                    Layout.preferredWidth: KanteStyle.unit(72)
                    Layout.preferredHeight: KanteStyle.unit(56)
                    color: isTarget ? KanteStyle.cardColor : "transparent"
                    border.width: isTarget ? 2 : 1
                    border.color: isTarget ? KanteStyle.accentColor : KanteStyle.frameColor
                    Accessible.role: Accessible.Button
                    Accessible.name: texts.t("tuner.string", { name: modelData.name })
                        + (isDone ? ", " + texts.t("tuner.ok") : "")
                    Row {
                        anchors.centerIn: parent
                        spacing: KanteStyle.unit(6)
                        QQC2.Label {
                            text: modelData.name
                            font: KanteStyle.monoFont(KanteStyle.defaultFont.pointSize * 1.6, true)
                            color: KanteStyle.strongTextColor
                        }
                        KanteNoteMark { visible: isDone; noteState: "hit"; width: KanteStyle.unit(18); height: width; anchors.verticalCenter: parent.verticalCenter }
                    }
                    MouseArea { anchors.fill: parent; onClicked: { page.target = index; page.inTuneMs = 0 } }
                }
            }
            Item { Layout.fillWidth: true }
            KanteButton {
                visible: page.done.length > 0
                text: texts.t("tuner.again")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: page.restart()
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            visible: page.guided
            wrapMode: Text.WordWrap
            font: KanteStyle.defaultFont
            color: KanteStyle.strongTextColor
            text: page.allDone ? texts.t("tuner.alldone")
                : !page.targetString ? ""
                : !page.hearing ? texts.t("tuner.pluck", { name: page.targetString.name })
                : page.wrongString ? texts.t("tuner.wrongstring", { name: page.targetString.name })
                : Math.abs(page.targetCents) <= 3 ? texts.t("tuner.hold")
                : page.targetCents < 0 ? texts.t("tuner.up.string", { name: page.targetString.name })
                : texts.t("tuner.down.string", { name: page.targetString.name })
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }

        KanteTunerGauge {
            Layout.fillWidth: true
            Layout.fillHeight: true
            noteName: page.guided && page.targetString && page.hearing && !page.wrongString ? page.targetString.name : page.noteName
            octave: page.guided && page.targetString && page.hearing && !page.wrongString ? Math.floor(page.targetString.midi / 12) - 1 : page.octave
            cents: page.guided && page.hearing && !page.wrongString ? Math.max(-50, Math.min(50, page.targetCents)) : ctl.tunerCents
            active: page.hearing
            readonly property real shownCents: page.guided && !page.wrongString ? page.targetCents : ctl.tunerCents
            hint: !page.hearing ? texts.t("tuner.silent")
                : shownCents < -3 ? texts.t("tuner.up")
                : shownCents > 3 ? texts.t("tuner.down")
                : texts.t("tuner.ok") + (!page.guided && page.ringing !== "" ? " · " + texts.t("tuner.string", { name: page.ringing }) : "")
            inTuneText: texts.t("tuner.ok")
            accessibleName: texts.t("tuner.name")
        }

        KanteLevelMeter {
            id: meter
            Layout.fillWidth: true
            peakDb: ctl.peakDb
            rmsDb: ctl.rmsDb
            clipped: ctl.clipped
            quietText: texts.t("level.quiet")
            goodText: texts.t("level.good")
            loudText: texts.t("level.loud")
            clipText: texts.t("level.clip")
            title: texts.t("level.title")
            accessibleName: texts.t("level.name")
        }
        LevelAdvice {
            Layout.fillWidth: true
            listening: ctl.tunerActive
            onUseLaptopJack: ctl.changeConnection("laptop")
        }
    }
}
