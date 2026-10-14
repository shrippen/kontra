import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import Kante

// Songs from outside (LH-BIB-11, QUELLEN-SONGS.md): search Songsterr or MuseScore in the browser;
// what the browser saves shows under "New in Downloads", one click opens it. Kontra fetches nothing
// itself: both sites forbid automated access. Offline only the browser fails, the rest works.
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

    signal openPath(string path)

    function refresh() {
        downloads = JSON.parse(ctl.downloadsJson())
    }

    function search(site) {
        var url = ctl.searchUrl(site, field.text)
        if (url === "") {
            field.forceActiveFocus()
            return
        }
        failedUrl = Qt.openUrlExternally(url) ? "" : url
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

    GridLayout {
        Layout.fillWidth: true
        columns: sources.slim ? 1 : 3
        KanteTextField {
            id: field
            Layout.fillWidth: true
            placeholderText: texts.t("search.field")
            Accessible.name: texts.t("search.field")
            onAccepted: sources.search("songsterr")
        }
        KanteButton {
            Layout.fillWidth: sources.slim
            text: texts.t("search.songsterr")
            Accessible.name: texts.t("search.songsterr.name")
            onClicked: sources.search("songsterr")
        }
        KanteButton {
            Layout.fillWidth: sources.slim
            text: texts.t("search.musescore")
            Accessible.name: texts.t("search.musescore.name")
            onClicked: sources.search("musescore")
        }
    }
    QQC2.Label {
        Layout.fillWidth: true
        text: texts.t("search.hint")
        wrapMode: Text.WordWrap
        color: KanteStyle.mutedTextColor
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
