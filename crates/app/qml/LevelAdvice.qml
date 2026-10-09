import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Pegel-Assistent (LH-EIN-03/04): reads the input level over the last seconds, not the instant
// peak, so pauses between notes do not flip to "too quiet" (QA P1 #8). Tells "no signal" apart
// from "too quiet", explains gain, and after a few quiet seconds on the line input offers the
// laptop-jack profile (QA P1 #14: the hint came only when the user picked the jack).
ColumnLayout {
    id: advice

    property real peakDb: -120
    property bool clipped: false
    /// Only judge while sound is expected (playing, or the setup's level step).
    property bool listening: true
    property bool laptopJack: false
    signal useLaptopJack()

    readonly property var texts: applicationWindow().texts
    // Highest peak of the last `holdMs`; clipping counts for the same time.
    property real heldDb: -120
    property bool heldClip: false
    property int holdMs: 3000
    property real listenedMs: 0
    property real quietMs: 0
    property var history: []

    readonly property string state: !listening ? "" : heldClip ? "clip"
        : heldDb < -60 ? (listenedMs > 1500 ? "silent" : "")
        : heldDb < -36 ? "quiet" : heldDb > -6 ? "loud" : "good"
    readonly property bool suggestLaptop: listening && !laptopJack && state === "quiet" && quietMs > 4000

    spacing: KanteStyle.unit(6)

    Timer {
        interval: 100
        repeat: true
        running: advice.listening
        onRunningChanged: if (!running) { advice.history = []; advice.heldDb = -120; advice.heldClip = false; advice.listenedMs = 0; advice.quietMs = 0 }
        onTriggered: {
            var now = Date.now()
            var h = advice.history.filter(function(e) { return now - e.t < advice.holdMs })
            h.push({ t: now, db: advice.peakDb, clip: advice.clipped })
            advice.history = h
            advice.heldDb = h.reduce(function(m, e) { return Math.max(m, e.db) }, -120)
            advice.heldClip = h.some(function(e) { return e.clip })
            advice.listenedMs += interval
            advice.quietMs = advice.state === "quiet" ? advice.quietMs + interval : 0
        }
    }

    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        visible: advice.state !== "" && advice.state !== "good"
        text: advice.state === "" ? "" : advice.texts.t("advice." + advice.state)
        color: KanteStyle.mutedTextColor
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }
    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        visible: advice.state === "quiet" || advice.state === "loud" || advice.state === "clip"
        text: advice.texts.t("advice.gain")
        color: KanteStyle.mutedTextColor
        font: KanteStyle.smallFont
    }
    RowLayout {
        visible: advice.suggestLaptop
        Layout.fillWidth: true
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: advice.texts.t("advice.laptop")
            color: KanteStyle.textColor
        }
        KanteButton {
            text: advice.texts.t("advice.laptop.use")
            onClicked: advice.useLaptopJack()
        }
    }
}
