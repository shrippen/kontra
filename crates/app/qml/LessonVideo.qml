import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import QtMultimedia
import Kante

// A course video (LH-LRN-14) in a lesson: nothing loads until "play" (a stream starts on a click,
// never on opening the page); the subtitle of the moment stands under the picture as text (P7).
// Loaded by LessonBlock; without Qt Multimedia that one falls back to a link.
ColumnLayout {
    id: video

    /** Asked for on the first "play": the address to stream or the unpacked file. */
    property var source: function() { return "" }
    /** `[{startMs, endMs, text}]` from the course's WebVTT file. */
    property var cues: []
    /** Streamed from the web: a failure means no connection. */
    property bool remote: false
    readonly property var texts: applicationWindow().texts
    property string failure: ""
    readonly property string cue: {
        const at = player.position
        const hit = video.cues.find(function(c) { return c.startMs <= at && at < c.endMs })
        return hit ? hit.text : ""
    }

    function clock(ms) {
        const s = Math.floor(ms / 1000)
        return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0")
    }
    function toggle() {
        if (player.playing) {
            player.pause()
            return
        }
        if (player.source.toString() === "") {
            const url = video.source()
            if (url === "") {
                video.failure = texts.t("learn.video.error")
                return
            }
            video.failure = ""
            player.source = url
        }
        player.play()
    }

    spacing: KanteStyle.unit(8)

    MediaPlayer {
        id: player
        videoOutput: output
        audioOutput: AudioOutput {}
        onErrorOccurred: function(error, message) {
            video.failure = texts.t(video.remote ? "learn.video.offline" : "learn.video.error")
        }
    }

    // The picture: 16:9 at most as wide as the text column, shown once something plays.
    VideoOutput {
        id: output
        Layout.fillWidth: true
        Layout.maximumWidth: KanteStyle.unit(640)
        Layout.preferredHeight: width * 9 / 16
        visible: player.source.toString() !== ""
    }
    QQC2.Label {
        Layout.fillWidth: true
        Layout.maximumWidth: KanteStyle.unit(640)
        visible: video.cue !== ""
        wrapMode: Text.WordWrap
        horizontalAlignment: Text.AlignHCenter
        color: KanteStyle.textColor
        text: video.cue
        Accessible.role: Accessible.StaticText
        Accessible.name: video.cue
    }

    RowLayout {
        spacing: KanteStyle.unit(8)
        KanteButton {
            text: player.playing ? texts.t("learn.video.pause")
                : player.source.toString() === "" ? texts.t("learn.video") : texts.t("learn.video.play")
            onClicked: video.toggle()
        }
        QQC2.Label {
            visible: player.duration > 0
            color: KanteStyle.textColor
            text: video.clock(player.position) + " / " + video.clock(player.duration)
        }
    }
    KanteCallout {
        Layout.fillWidth: true
        Layout.maximumWidth: KanteStyle.unit(640)
        visible: video.failure !== ""
        kind: KanteCallout.Kind.Warn
        autoHide: false
        text: video.failure
    }
}
