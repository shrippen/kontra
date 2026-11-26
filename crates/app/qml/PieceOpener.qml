import QtQuick

// Opens a piece and goes where it leads: to practice, or to the library when the bass track must
// be chosen (the library shows that question). Main.qml owns one and forwards to it.
QtObject {
    id: opener

    required property var ctl
    required property string current
    signal showRequested(string name)

    // `prepare` runs once the piece is open, before the page changes (mode, loop); `after` once
    // practice shows (start a probe). Opening from the practice page itself (the run summary)
    // keeps that page. Returns whether the piece opened.
    function openPiece(path, prepare, after) {
        if (!ctl.openFile(path)) {
            if (current !== "library" && trackToChoose()) {
                showRequested("library")
            }
            return false
        }
        if (prepare) {
            prepare()
        }
        if (current !== "practice") {
            showRequested("practice")
        }
        if (after) {
            after()
        }
        return true
    }

    // The next piece of the active setlist; like `openPiece`, a bass track to choose leads to the
    // library. The practice page stays when it opens.
    function openNext() {
        if (!ctl.openNext() && trackToChoose()) {
            showRequested("library")
        }
    }

    function trackToChoose() {
        return JSON.parse(ctl.trackChoicesJson).length > 0
    }
}
