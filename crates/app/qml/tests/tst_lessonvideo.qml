import QtQuick
import QtTest

// The course video player (LH-LRN-14, Runde 14) with Qt Multimedia: a WebM and an MP4 play,
// and the subtitle of the moment shows as text (P7). Run by scripts/check-video.sh.
Item {
    id: root
    width: 640
    height: 480

    // LessonVideo reads its texts from the application window; keys stand for the texts.
    readonly property QtObject texts: QtObject {
        function t(key) { return key }
    }
    function applicationWindow() { return root }

    Loader {
        id: loader
        width: parent.width
        active: false
        source: Qt.resolvedUrl("../LessonVideo.qml")
    }

    TestCase {
        name: "LessonVideo"
        when: windowShown

        function cleanup() {
            loader.active = false
        }

        function test_plays_with_subtitle_data() {
            return [
                { tag: "webm", file: "clip.webm" },
                { tag: "mp4", file: "clip.mp4" },
            ]
        }

        function test_plays_with_subtitle(data) {
            loader.active = true
            compare(loader.status, Loader.Ready, "LessonVideo.qml loads (Qt Multimedia present)")
            const video = loader.item
            const url = Qt.resolvedUrl("../../../../testdata/video/" + data.file)
            video.cues = [{ startMs: 300, endMs: 1900, text: "Zeigefinger" }]
            video.source = function() { return url }

            // The cue starts after 300 ms: seeing it means the clip really plays.
            compare(video.cue, "")
            video.toggle()
            tryCompare(video, "cue", "Zeigefinger", 5000)
            compare(video.failure, "")
        }
    }
}
