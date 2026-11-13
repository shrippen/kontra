import QtQuick
import QtCore
import QtQuick.Layouts
import QtQuick.Dialogs
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Library (LH-BIB-04/05/17/18/19): the sources on the left (all, recent, origins, linked folders,
// setlists), a table on the right with search, grouping and sorting; "Add pieces" holds the ways
// in (download, folder, file, generate); files and folders can be dragged onto the page.
//
//   ┌ sources ──────┐ ┌ Search …            Group by ▾   [Add pieces ▾] ┐
//   │ All pieces    │ │ All pieces · 214 · sorted by Title, ascending   │
//   │ ORIGIN …      │ │ TITLE ▲   ORIGIN   TEMPO   LAST   RESULT        │
//   │ FOLDERS …     │ │ rows …                                          │
//   │ SETLISTS …    │ └─────────────────────────────────────────────────┘
Kirigami.Page {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    // What the page shows: a source key, a column and direction, a grouping and the search.
    property string source: "all"
    property string sort: "lastPlayed:desc"
    property string group: "none"
    property string query: ""
    // The table's data. Read again when the library, a setlist or a linked folder changed.
    readonly property var view: {
        ctl.recentJson
        ctl.setlistsJson
        ctl.libraryRevision
        return JSON.parse(ctl.libraryJson(source, sort, group, query))
    }
    readonly property bool isSetlist: source.indexOf("setlist:") === 0
    readonly property var tracks: JSON.parse(ctl.trackChoicesJson)
    readonly property bool slim: width < KanteStyle.unit(700)
    readonly property var groups: ["none", "artist", "folder", "origin"]
    readonly property var sourceItems: tree.entries.filter(function(e) { return e.kind !== "heading" })
    readonly property string sourceText: {
        var found = tree.entries.filter(function(e) { return e.key === source })[0]
        return found ? found.text : ""
    }

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("nav.library")
    leftPadding: applicationWindow().readingPad(width)
    rightPadding: leftPadding
    topPadding: KanteStyle.unit(16)
    bottomPadding: KanteStyle.unit(16)

    // The folders are read again whenever the page shows (LH-BIB-18).
    onVisibleChanged: {
        if (visible) {
            ctl.libraryRescan()
        }
    }
    Component.onCompleted: ctl.libraryRescan()

    function open(url) {
        if (ctl.openFile(url)) {
            applicationWindow().show("practice")
        }
    }

    function openFromSetlist(name, index) {
        if (ctl.openFromSetlist(name, index)) {
            applicationWindow().show("practice")
        }
    }

    function pick(key) {
        source = key
    }

    function addWay(way) {
        if (way === "download") {
            downloadDialog.open()
        } else if (way === "folder") {
            folderDialog.open()
        } else if (way === "file") {
            dialog.open()
        } else {
            generateDialog.open()
        }
    }

    // Files and folders dropped on the page: a folder is linked, one file opens, several go
    // into the library (LH-BIB-11).
    function drop(urls) {
        var folders = urls.filter(function(u) { return ctl.isFolder(u) })
        var files = urls.filter(function(u) { return !ctl.isFolder(u) })
        folders.forEach(function(u) { ctl.linkFolder(u) })
        if (files.length === 1 && folders.length === 0) {
            open(files[0])
        } else if (files.length > 0) {
            ctl.importFiles(JSON.stringify(files))
        }
    }

    // Where the last own piece came from, else Downloads, where tabs from the web land (QA
    // 14.10.: the dialog opened in the program's working folder).
    function startFolder() {
        var recent = JSON.parse(ctl.recentJson)
            .filter(function(e) {
                var path = e.path.replace(/\\/g, "/")
                return path.indexOf("/exercises/") < 0 && path.indexOf("/pieces/") < 0
            })
            .sort(function(a, b) { return b.lastUnix - a.lastUnix })
        if (recent.length > 0) {
            return ctl.folderUrl(recent[0].path)
        }
        return StandardPaths.writableLocation(StandardPaths.DownloadLocation)
    }

    FolderDialog {

        // QA: Qt's dialog in the window, never the desktop portal.

        options: ctl.qaRun ? FolderDialog.DontUseNativeDialog : 0
        id: folderDialog
        title: texts.t("library.folder.pick")
        currentFolder: StandardPaths.writableLocation(StandardPaths.MusicLocation)
        onAccepted: folderWay.ask(selectedFolder.toString())
    }

    AddFolderDialog { id: folderWay; ctl: page.ctl; texts: page.texts }

    FileDialog {

        // QA: Qt's dialog in the window, never the desktop portal.

        options: ctl.qaRun ? FileDialog.DontUseNativeDialog : 0
        id: packageDialog
        title: texts.t("dialog.package")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "kontra"
        nameFilters: [texts.t("package.files") + " (*.kontra)"]
        property string path: ""
        property bool withAudio: false
        onAccepted: {
            ctl.exportPackage(path, selectedFile.toString(), withAudio ? "embed" : "link")
            table.sharePath = ""
        }
    }

    FileDialog {

        // QA: Qt's dialog in the window, never the desktop portal.

        options: ctl.qaRun ? FileDialog.DontUseNativeDialog : 0
        id: dialog
        title: texts.t("dialog.open")
        currentFolder: page.startFolder()
        nameFilters: [texts.t("library.formats") + " (*.gp *.gp3 *.gp4 *.gp5 *.gpx *.mid *.midi *.musicxml *.xml *.mxl *.mscz *.mscx *.kontra)"]
        onAccepted: page.open(selectedFile.toString())
    }

    DownloadDialog {
        id: downloadDialog
        ctl: page.ctl
        texts: page.texts
        onOpenPath: function(path) { page.open(path) }
    }

    GenerateDialog {
        id: generateDialog
        ctl: page.ctl
        texts: page.texts
        onGenerated: applicationWindow().show("practice")
    }

    KanteDialog {
        id: newSetlistDialog
        parent: QQC2.Overlay.overlay
        anchors.centerIn: parent
        modal: true
        size: KanteDialog.Size.Small
        title: texts.t("library.setlist.new.title")
        standardButtons: QQC2.Dialog.Ok | QQC2.Dialog.Cancel
        acceptText: texts.t("setlist.create")
        acceptEnabled: newSetlistName.text.trim() !== ""
        onOpened: {
            newSetlistName.text = ""
            newSetlistName.forceActiveFocus()
        }
        onAccepted: {
            ctl.setlistCreate(newSetlistName.text)
            page.source = "setlist:" + newSetlistName.text.trim()
        }
        KanteTextField {
            id: newSetlistName
            anchors.left: parent.left
            anchors.right: parent.right
            placeholderText: texts.t("setlist.new")
            Accessible.name: texts.t("setlist.new")
            onAccepted: {
                if (text.trim() !== "") {
                    newSetlistDialog.accept()
                }
            }
        }
    }

    AddPiecesMenu {
        id: addMenu
        texts: page.texts
        onChosen: function(way) { page.addWay(way) }
    }

    // Anything dragged onto the page: the overlay says what happens (LH-BIB-11, LH-BIB-17).
    DropArea {
        id: drop
        anchors.fill: parent
        z: 1
        keys: ["text/uri-list"]
        onDropped: function(event) {
            event.accept(Qt.CopyAction)
            page.drop(event.urls.map(String))
        }
    }
    KanteDropZone {
        anchors.fill: parent
        z: 2
        visible: drop.containsDrag
        kind: KanteDropZone.Kind.Cell
        ColumnLayout {
            anchors.centerIn: parent
            spacing: KanteStyle.unit(4)
            QQC2.Label {
                Layout.alignment: Qt.AlignHCenter
                text: texts.t("library.drop.title")
                color: KanteStyle.strongTextColor
                font.bold: true
            }
            QQC2.Label {
                Layout.alignment: Qt.AlignHCenter
                text: texts.t("library.drop.hint")
                color: KanteStyle.mutedTextColor
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(16)

        LibraryTree {
            id: tree
            visible: !page.slim
            Layout.preferredWidth: KanteStyle.unit(232)
            Layout.fillHeight: true
            texts: page.texts
            counts: page.view.counts
            source: page.source
            onPicked: function(key) { page.pick(key) }
            onAddFolderRequested: folderDialog.open()
            onNewSetlistRequested: newSetlistDialog.open()
            onUnlinkRequested: function(path) {
                ctl.unlinkFolder(path)
                if (page.source.indexOf("folder:" + path) === 0) {
                    page.pick("all")
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: KanteStyle.unit(12)

            KanteHeading { text: texts.t("nav.library"); pageTitle: true; Layout.fillWidth: true }

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

            RowLayout {
                Layout.fillWidth: true
                spacing: KanteStyle.unit(8)
                // Narrow: the sources are a choice instead of a tree.
                KanteComboBox {
                    visible: page.slim
                    Layout.preferredWidth: KanteStyle.unit(180)
                    Accessible.name: texts.t("library.source.pick")
                    model: page.sourceItems.map(function(e) { return (e.sign ? e.sign + " " : "") + e.text })
                    currentIndex: page.sourceItems.map(function(e) { return e.key }).indexOf(page.source)
                    onActivated: function(index) {
                        var entry = page.sourceItems[index]
                        if (entry.kind === "item") {
                            page.pick(entry.key)
                        } else if (entry.id === "folder") {
                            folderDialog.open()
                        } else {
                            newSetlistDialog.open()
                        }
                    }
                }
                KanteTextField {
                    Layout.fillWidth: true
                    visible: !page.isSetlist
                    placeholderText: texts.t("library.search")
                    Accessible.name: texts.t("library.search")
                    onTextChanged: page.query = text
                }
                Item { Layout.fillWidth: true; visible: page.isSetlist }
                KanteComboBox {
                    visible: !page.isSetlist && !page.slim
                    Layout.preferredWidth: KanteStyle.unit(220)
                    Accessible.name: texts.t("library.group")
                    displayText: texts.t("library.group") + ": " + texts.t("library.group." + page.group)
                    model: page.groups.map(function(g) { return texts.t("library.group." + g) })
                    currentIndex: page.groups.indexOf(page.group)
                    onActivated: function(index) { page.group = page.groups[index] }
                }
                KanteButton {
                    id: addButton
                    text: texts.t("library.add") + " ▾"
                    emphasis: KanteButton.Emphasis.Primary
                    Accessible.name: texts.t("library.add")
                    onClicked: addMenu.openAt(addButton)
                }
            }

            // Where the table stands: the source, how many pieces, the column it is sorted by.
            QQC2.Label {
                visible: !page.isSetlist
                Layout.fillWidth: true
                elide: Text.ElideRight
                color: KanteStyle.mutedTextColor
                text: {
                    var key = page.sort.split(":")[0]
                    var names = { title: "title", origin: "origin", tempo: "tempo", lastPlayed: "last", result: "result" }
                    return texts.t("library.crumb.sorted", {
                        source: page.sourceText,
                        n: page.view.rows.length,
                        column: texts.t("library.col." + names[key]),
                        direction: table.directionText(page.sort.split(":")[1] || "asc")
                    })
                }
            }

            KanteEmptyState {
                visible: !page.isSetlist && page.view.rows.length === 0
                Layout.fillWidth: true
                title: page.query !== "" ? texts.t("library.nomatch")
                    : page.source === "all" ? texts.t("library.empty.title") : texts.t("library.source.empty")
                text: page.source === "all" && page.query === "" ? texts.t("library.empty.hint") : ""
            }

            LibraryTable {
                id: table
                visible: !page.isSetlist && page.view.rows.length > 0
                Layout.fillWidth: true
                Layout.fillHeight: true
                ctl: page.ctl
                texts: page.texts
                view: page.view
                sort: page.sort
                compact: page.width < KanteStyle.unit(900)
                onSortChosen: function(sort) { page.sort = sort }
                onOpenPiece: function(path) { page.open(path) }
                onShareRequested: function(path, title, withAudio) {
                    packageDialog.path = path
                    packageDialog.withAudio = withAudio
                    packageDialog.selectedFile = StandardPaths.writableLocation(StandardPaths.DocumentsLocation) + "/" + String(title).replace(/[\/:*?"<>|]+/g, "-") + ".kontra"
                    packageDialog.open()
                }
            }

            LibrarySetlist {
                visible: page.isSetlist
                Layout.fillWidth: true
                Layout.fillHeight: true
                ctl: page.ctl
                texts: page.texts
                name: page.source.substring("setlist:".length)
                onOpenRequested: function(name, index) { page.openFromSetlist(name, index) }
                onDeleted: page.pick("all")
            }
        }
    }
}
