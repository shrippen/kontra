import QtQuick
import QtCore
import QtQuick.Layouts
import QtQuick.Dialogs
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Library (LH-BIB-04/05): open a score, pick the bass track if needed, recent pieces.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property string query: ""
    // LH-BIB-04: sort by last played (default), title or result.
    property string sort: "recent"
    readonly property var recent: {
        var list = JSON.parse(ctl.recentJson).filter(function(e) {
            var q = query.trim().toLowerCase()
            return q === "" || (e.title + " " + e.artist + " " + e.path + " " + (e.tags || []).join(" ")).toLowerCase().indexOf(q) >= 0
        })
        if (sort === "title")
            list.sort(function(a, b) { return a.title.localeCompare(b.title) })
        else if (sort === "best")
            list.sort(function(a, b) { return (b.best || 0) - (a.best || 0) })
        return list
    }
    readonly property var setlists: JSON.parse(ctl.setlistsJson)
    readonly property var tracks: JSON.parse(ctl.trackChoicesJson)
    readonly property bool slim: width < KanteStyle.unit(700)
    // LH-BIB-07: the piece being shared and whether its backing audio goes along.
    property string sharePath: ""
    property bool shareAudio: false

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("nav.library")

    function open(url) {
        if (ctl.openFile(url)) {
            applicationWindow().show("practice")
        }
    }

    FolderDialog {
        id: folderDialog
        onAccepted: ctl.importFolder(selectedFolder.toString(), true)
    }

    function openFromSetlist(name, index) {
        if (ctl.openFromSetlist(name, index)) {
            applicationWindow().show("practice")
        }
    }

    FileDialog {
        id: packageDialog
        fileMode: FileDialog.SaveFile
        defaultSuffix: "kontra"
        nameFilters: [texts.t("package.files") + " (*.kontra)"]
        onAccepted: {
            ctl.exportPackage(page.sharePath, selectedFile.toString(), page.shareAudio ? "embed" : "link")
            page.sharePath = ""
        }
    }

    FileDialog {
        id: dialog
        nameFilters: [texts.t("library.formats") + " (*.gp *.gp3 *.gp4 *.gp5 *.gpx *.mid *.midi *.musicxml *.xml *.mxl *.mscz *.mscx *.kontra)"]
        onAccepted: page.open(selectedFile.toString())
    }

    // Files dragged onto the page: one opens, several go into the library (LH-BIB-11).
    DropArea {
        id: drop
        parent: page
        anchors.fill: parent
        keys: ["text/uri-list"]
        onDropped: function(event) {
            var urls = event.urls.map(String)
            event.accept(Qt.CopyAction)
            if (urls.length === 1)
                page.open(urls[0])
            else
                ctl.importFiles(JSON.stringify(urls))
        }
    }
    KanteDropZone {
        parent: page
        anchors.fill: parent
        anchors.margins: KanteStyle.unit(8)
        visible: drop.containsDrag
        kind: KanteDropZone.Kind.Cell
        QQC2.Label {
            anchors.centerIn: parent
            text: texts.t("search.drop")
            color: KanteStyle.strongTextColor
        }
    }

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: texts.t("nav.library"); pageTitle: true; Layout.fillWidth: true }
            KanteButton {
                text: texts.t("library.folder")
                emphasis: KanteButton.Emphasis.Quiet
                onClicked: folderDialog.open()
            }
            KanteButton {
                text: texts.t("library.open")
                emphasis: KanteButton.Emphasis.Primary
                onClicked: dialog.open()
            }
        }

        // No bass track recognized: the user chooses (bass guess below 3 points).
        ColumnLayout {
            visible: page.tracks.length > 0
            Layout.fillWidth: true
            KanteSectionLabel { text: texts.t("track.choose"); Layout.fillWidth: true }
            Repeater {
                model: page.tracks
                delegate: KanteListRow {
                    required property int index
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData
                    leadingText: String(index + 1)
                    onClicked: {
                        if (ctl.chooseTrack(index)) {
                            applicationWindow().show("practice")
                        }
                    }
                }
            }
        }

        KanteEmptyState {
            visible: JSON.parse(ctl.recentJson).length === 0 && page.tracks.length === 0
            Layout.fillWidth: true
            title: texts.t("library.empty.title")
            text: texts.t("library.empty.hint")
            // No file of one's own yet (P1): the bundled exercises are one step away.
            KanteButton {
                text: texts.t("library.empty.exercises")
                onClicked: applicationWindow().show("exercises")
            }
        }

        KanteTextField {
            Layout.fillWidth: true
            visible: JSON.parse(ctl.recentJson).length > 5
            placeholderText: texts.t("library.search")
            onTextChanged: page.query = text
        }
        QQC2.Label {
            visible: page.query !== "" && page.recent.length === 0
            text: texts.t("library.nomatch")
            color: KanteStyle.mutedTextColor
        }

        ColumnLayout {
            visible: page.recent.length > 0
            Layout.fillWidth: true
            RowLayout {
                Layout.fillWidth: true
                KanteSectionLabel { text: texts.t("library.pieces"); Layout.fillWidth: true }
                KanteSegmented {
                    readonly property var values: ["recent", "title", "best"]
                    Accessible.name: texts.t("library.sort")
                    model: [texts.t("library.sort.recent"), texts.t("library.sort.title"), texts.t("library.sort.best")]
                    currentIndex: values.indexOf(page.sort)
                    onActivated: function(index) { page.sort = values[index] }
                }
            }
            Repeater {
                model: page.recent
                delegate: ColumnLayout {
                    id: piece
                    required property var modelData
                    property bool editing: false
                    readonly property bool sharing: page.sharePath === String(modelData.path)
                    Layout.fillWidth: true
                    spacing: 0
                    KanteListRow {
                        Layout.fillWidth: true
                        text: piece.modelData.title
                        // Tags (LH-BIB-04) after the artist; the search finds them too.
                        // Two pieces of one title: the file name tells them apart (QA 13.10.).
                        readonly property bool twin: page.recent.filter(function(r) {
                            return r.title === piece.modelData.title }).length > 1
                        readonly property string result: piece.modelData.best !== null && piece.modelData.best !== undefined
                              ? texts.t("library.best", { percent: Math.round(piece.modelData.best * 100) })
                                + " · " + (piece.modelData.practiceMin < 1 ? texts.t("library.underminute") : texts.t("library.minutes", { minutes: Math.round(piece.modelData.practiceMin) }))
                              : texts.t("library.unplayed")
                        // In a slim window the result goes under the title, which keeps its room.
                        subtitle: [piece.modelData.artist]
                                  .concat(twin ? [String(piece.modelData.path).split("/").pop()] : [])
                                  .concat(piece.modelData.tags.map(function(t) { return "#" + t }))
                                  .concat(page.slim ? [result] : [])
                                  .filter(function(s) { return s !== "" }).join(" · ")
                        meta: page.slim ? "" : result
                        onClicked: page.open(String(piece.modelData.path))
                        trailing: Row {
                            spacing: KanteStyle.unit(4)
                            KanteButton {
                                text: texts.t("library.tags")
                                emphasis: KanteButton.Emphasis.Quiet
                                checkable: true
                                checked: piece.editing
                                Accessible.name: texts.t("library.tags.edit", { title: piece.modelData.title })
                                onClicked: piece.editing = !piece.editing
                            }
                            // LH-BIB-07: share the piece as a .kontra package.
                            KanteButton {
                                text: texts.t("package.share")
                                emphasis: KanteButton.Emphasis.Quiet
                                Accessible.name: texts.t("package.share.name", { title: piece.modelData.title })
                                onClicked: page.sharePath = piece.sharing ? "" : String(piece.modelData.path)
                            }
                        }
                    }
                    KanteTagPicker {
                        Layout.fillWidth: true
                        visible: piece.editing
                        tags: piece.modelData.tags
                        suggestions: piece.editing ? JSON.parse(ctl.tagsJson()) : []
                        placeholderText: texts.t("library.tags.add")
                        removeText: texts.t("library.tags.remove")
                        Accessible.name: texts.t("library.tags.edit", { title: piece.modelData.title })
                        onEdited: function(tags) { ctl.setTags(String(piece.modelData.path), JSON.stringify(tags)) }
                    }
                    // Audio goes along only when chosen here (J-D2).
                    RowLayout {
                        visible: piece.sharing
                        Layout.fillWidth: true
                        Layout.leftMargin: KanteStyle.unit(16)
                        QQC2.CheckBox {
                            id: withAudio
                            enabled: piece.modelData.hasBacking === true
                            text: piece.modelData.hasBacking === true ? texts.t("package.audio") : texts.t("package.audio.none")
                            Accessible.name: text
                        }
                        Item { Layout.fillWidth: true }
                        KanteButton {
                            text: texts.t("package.save")
                            emphasis: KanteButton.Emphasis.Primary
                            onClicked: {
                                page.shareAudio = withAudio.checked
                                packageDialog.currentFile = StandardPaths.writableLocation(StandardPaths.DocumentsLocation) + "/" + String(piece.modelData.title).replace(/[\/:*?"<>|]+/g, "-") + ".kontra"
                                packageDialog.open()
                            }
                        }
                    }
                }
            }
        }

        // Songs from Songsterr and MuseScore via the browser, new downloads (LH-BIB-11, J-B6).
        SongSources {
            Layout.fillWidth: true
            Layout.topMargin: KanteStyle.unit(16)
            ctl: page.ctl
            texts: page.texts
            active: page.visible
            onOpenPath: function(path) { page.open(path) }
        }

        // Setlists (LH-BIB-04, J-B1): an order of pieces for a gig; playing one offers the next.
        // Below the pieces: those are what most players come for (QA 13.10.).
        KanteSectionLabel { text: texts.t("setlist.title"); Layout.fillWidth: true; Layout.topMargin: KanteStyle.unit(16) }
        Repeater {
            model: page.setlists
            delegate: ColumnLayout {
                id: list
                required property var modelData
                Layout.fillWidth: true
                spacing: 0
                RowLayout {
                    Layout.fillWidth: true
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: list.modelData.name + " · " + texts.t("setlist.count", { n: list.modelData.pieces.length })
                        color: KanteStyle.strongTextColor
                        font.bold: true
                    }
                    KanteButton {
                        visible: ctl.loaded
                        text: texts.t("setlist.addloaded")
                        emphasis: KanteButton.Emphasis.Quiet
                        onClicked: ctl.setlistAddLoaded(list.modelData.name)
                    }
                    // Two steps: a setlist is gone for good (the pieces stay in the library).
                    KanteButton {
                        property bool armed: false
                        text: armed ? texts.t("setlist.delete.sure") : texts.t("setlist.delete")
                        emphasis: armed ? KanteButton.Emphasis.Primary : KanteButton.Emphasis.Quiet
                        onClicked: armed ? ctl.setlistDelete(list.modelData.name) : armed = true
                        onActiveFocusChanged: if (!activeFocus) armed = false
                        Timer { running: parent.armed; interval: 4000; onTriggered: parent.armed = false }
                    }
                }
                Repeater {
                    model: list.modelData.pieces
                    delegate: RowLayout {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        KanteListRow {
                            Layout.fillWidth: true
                            leadingText: String(index + 1)
                            text: modelData.title
                            subtitle: modelData.artist
                            disabledReason: modelData.missing ? texts.t("setlist.missing") : ""
                            enabled: !modelData.missing
                            onClicked: page.openFromSetlist(list.modelData.name, index)
                        }
                        KanteButton {
                            Layout.preferredWidth: KanteStyle.unit(44)
                            text: "↑"; emphasis: KanteButton.Emphasis.Quiet; enabled: index > 0
                            Accessible.name: texts.t("setlist.up")
                            onClicked: ctl.setlistMove(list.modelData.name, index, -1)
                        }
                        KanteButton {
                            Layout.preferredWidth: KanteStyle.unit(44)
                            text: "↓"; emphasis: KanteButton.Emphasis.Quiet; enabled: index < list.modelData.pieces.length - 1
                            Accessible.name: texts.t("setlist.down")
                            onClicked: ctl.setlistMove(list.modelData.name, index, 1)
                        }
                        KanteButton {
                            Layout.preferredWidth: KanteStyle.unit(44)
                            text: "×"; emphasis: KanteButton.Emphasis.Quiet
                            Accessible.name: texts.t("setlist.remove")
                            onClicked: ctl.setlistRemove(list.modelData.name, index)
                        }
                    }
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            KanteTextField {
                id: newSetlist
                Layout.preferredWidth: KanteStyle.unit(320)
                placeholderText: texts.t("setlist.new")
                Accessible.name: texts.t("setlist.new")
                onAccepted: { ctl.setlistCreate(text); text = "" }
            }
            KanteButton {
                text: texts.t("setlist.create")
                enabled: newSetlist.text.trim() !== ""
                onClicked: { ctl.setlistCreate(newSetlist.text); newSetlist.text = "" }
            }
            Item { Layout.fillWidth: true }
        }
    }
}
