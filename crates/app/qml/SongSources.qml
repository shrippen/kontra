import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Songs from outside (LH-BIB-11, planung/QUELLEN-SONGS.md): "Search" asks Songsterr from Kontra (round 12),
// a result opens its page in the browser; both sites can also be searched in the browser. What the
// browser saves shows under "New in Downloads", one click opens it. Kontra downloads no tabs.
ColumnLayout {
    id: sources

    required property var ctl
    required property var texts
    // The download folder is read when the page shows or the window comes back, then every 15 s.
    property bool active: true
    property var downloads: ({ dir: "", files: [] })
    // Set when the browser could not be started: the address to copy instead.
    property string failedUrl: ""
    readonly property bool slim: width < KanteStyle.unit(560)
    // The Songsterr search: state ("idle", "running", "done", "error"), reason and hits.
    readonly property var songs: ctl.songSearchJson === "" ? ({ state: "idle", hits: [] })
                                                           : JSON.parse(ctl.songSearchJson)

    signal openPath(string path)

    function refresh() {
        downloads = JSON.parse(ctl.downloadsJson())
    }
    // Once at once: the timer runs only in the active window, and until its first round an
    // inactive one said "no download folder" (QA 14.10., window started without focus).
    Component.onCompleted: refresh()

    function search(site) {
        var url = ctl.searchUrl(site, field.text)
        if (url === "") {
            field.forceActiveFocus()
            return
        }
        browse(url)
    }

    function browse(url) {
        // QA: the browser stays shut; the log tells what would open.
        if (ctl.qaRun) {
            console.log("qa: open " + url)
            failedUrl = ""
            return
        }
        failedUrl = Qt.openUrlExternally(url) ? "" : url
    }

    // Only on the user's request; a second one waits for the first (one request in flight).
    function searchHere() {
        if (field.text.trim() === "") {
            field.forceActiveFocus()
            return
        }
        ctl.songSearch(field.text)
    }

    spacing: KanteStyle.unit(8)

    Timer {
        interval: 15000
        repeat: true
        triggeredOnStart: true
        running: sources.active && Qt.application.state === Qt.ApplicationActive
        onTriggered: sources.refresh()
    }

    KanteSectionLabel { text: texts.t("search.title"); Layout.fillWidth: true }

    // Wide: field, search and both browser buttons in one row. Slim: the browser buttons
    // side by side below (one per line they stood alone in the middle, QA 14.10.).
    GridLayout {
        Layout.fillWidth: true
        columns: sources.slim ? 2 : 3
        KanteTextField {
            id: field
            Layout.fillWidth: true
            placeholderText: texts.t("search.field")
            Accessible.name: texts.t("search.field")
            onAccepted: sources.searchHere()
            // A search started elsewhere (QA start option) shows its words.
            Component.onCompleted: text = sources.songs.query || ""
        }
        KanteButton {
            text: texts.t("search.web.button")
            emphasis: KanteButton.Emphasis.Primary
            enabled: sources.songs.state !== "running"
            Accessible.name: texts.t("search.web.button.name")
            onClicked: sources.searchHere()
        }
        RowLayout {
            Layout.columnSpan: sources.slim ? 2 : 1
            KanteButton {
                text: texts.t("search.songsterr")
                emphasis: KanteButton.Emphasis.Quiet
                Accessible.name: texts.t("search.songsterr.name")
                onClicked: sources.search("songsterr")
            }
            KanteButton {
                text: texts.t("search.musescore")
                emphasis: KanteButton.Emphasis.Quiet
                Accessible.name: texts.t("search.musescore.name")
                onClicked: sources.search("musescore")
            }
        }
    }
    QQC2.Label {
        Layout.fillWidth: true
        text: texts.t("search.hint")
        wrapMode: Text.WordWrap
        color: KanteStyle.mutedTextColor
    }

    // Songsterr's answer: the hits, or why there are none; the browser buttons stay above.
    QQC2.Label {
        Layout.fillWidth: true
        visible: sources.songs.state === "running" || sources.songs.state === "done"
        text: sources.songs.state === "running" ? texts.t("search.web.running")
              : sources.songs.hits.length === 0 ? texts.t("search.web.none", { query: sources.songs.query })
              : texts.t("search.web.results", { n: sources.songs.hits.length })
        wrapMode: Text.WordWrap
        color: KanteStyle.mutedTextColor
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }
    KanteCallout {
        Layout.fillWidth: true
        visible: sources.songs.state === "error"
        kind: KanteCallout.Kind.Warn
        title: texts.t("search.web.failed")
        text: sources.songs.message || ""
        autoHide: false
    }
    Repeater {
        model: sources.songs.hits
        delegate: KanteListRow {
            required property var modelData
            Layout.fillWidth: true
            text: modelData.title
            subtitle: modelData.artist + " · " + modelData.detail
            Accessible.name: texts.t("search.web.open", { title: modelData.title, artist: modelData.artist, detail: modelData.detail })
            onClicked: sources.browse(modelData.url)
        }
    }

    // No browser: say so and offer the address to copy (P7 reads it with the screen reader).
    KanteCallout {
        Layout.fillWidth: true
        visible: sources.failedUrl !== ""
        kind: KanteCallout.Kind.Warn
        title: texts.t("search.browser.failed")
        text: texts.t("search.browser.copy")
        dismissible: true
        autoHide: false
        onDismissed: sources.failedUrl = ""
    }
    KanteTextField {
        Layout.fillWidth: true
        visible: sources.failedUrl !== ""
        readOnly: true
        text: sources.failedUrl
        Accessible.name: texts.t("search.browser.address")
        onActiveFocusChanged: if (activeFocus) selectAll()
    }

    KanteSectionLabel {
        Layout.fillWidth: true
        Layout.topMargin: KanteStyle.unit(8)
        text: texts.t("search.downloads")
    }
    Repeater {
        model: sources.downloads.files
        delegate: KanteListRow {
            required property var modelData
            Layout.fillWidth: true
            text: modelData.name
            subtitle: modelData.kind + " · " + modelData.age
            Accessible.name: texts.t("search.downloads.open", { name: modelData.name })
            onClicked: sources.openPath(String(modelData.path))
        }
    }
    QQC2.Label {
        Layout.fillWidth: true
        visible: sources.downloads.files.length === 0
        text: sources.downloads.dir === "" ? texts.t("search.downloads.nodir")
                                           : texts.t("search.downloads.none", { dir: sources.downloads.dir })
        wrapMode: Text.WordWrap
        color: KanteStyle.mutedTextColor
    }
}
