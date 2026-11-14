import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import Kante

// The library's table (LH-BIB-19): title with artist and format, origin, tempo, last played and
// result; every column sorts (a second click reverses it), groups have a heading and a count.
// A row opens the piece; "…" holds tags and sharing.
//
//   TITEL ▲            HERKUNFT      TEMPO   ZULETZT            ERGEBNIS
//   QUEEN · 3                                                    (group heading)
//   Another One …      ↓ Download     110    played today     ▮▮▮▯ 62 %   …
//    Queen · GP
ColumnLayout {
    id: table

    required property var ctl
    required property var texts
    // `rows` and `groups` of Core::library_json.
    required property var view
    // "title:asc", "tempo:desc": the column and the direction, as library_json takes it.
    property string sort: "title:asc"
    // Narrow: tempo and last played drop out (they stay in the screen reader's description).
    property bool compact: false

    readonly property var signs: ({ exercise: "◆", generated: "◇", download: "↓", imported: "●", folder: "□" })
    readonly property var tones: ({
        exercise: KanteStyle.positiveTextColor,
        generated: KanteStyle.tagColor,
        download: KanteStyle.infoColor,
        imported: KanteStyle.strongTextColor,
        folder: KanteStyle.warningColor
    })
    // Direction a column starts with: figures that matter most come first.
    readonly property var firstDirection: ({ title: "asc", origin: "asc", tempo: "asc", lastPlayed: "desc", result: "desc" })
    readonly property string sortKey: sort.split(":")[0]
    readonly property string sortDirection: sort.split(":")[1] || "asc"
    readonly property var columns: allColumns.filter(function(c) { return !compact || (c.id !== "tempo" && c.id !== "last") })
    readonly property var allColumns: {
        return [
            { id: "title", sort: "title", title: texts.t("library.col.title"), width: 0 },
            { id: "origin", sort: "origin", title: texts.t("library.col.origin"), width: KanteStyle.unit(112) },
            { id: "tempo", sort: "tempo", title: texts.t("library.col.tempo"), width: KanteStyle.unit(56), align: Text.AlignRight },
            { id: "last", sort: "lastPlayed", title: texts.t("library.col.last"), width: KanteStyle.unit(128) },
            { id: "result", sort: "result", title: texts.t("library.col.result"), width: KanteStyle.unit(112) }
        ]
    }
    // Same titles: the file name tells them apart (QA 13.10.).
    readonly property var titleCounts: {
        var counts = ({})
        view.rows.forEach(function(r) { counts[r.title] = (counts[r.title] || 0) + 1 })
        return counts
    }
    // Pieces with a backing track of their own, by path: the sharing panel offers the audio.
    readonly property var backed: {
        var map = ({})
        JSON.parse(ctl.recentJson).forEach(function(e) { map[String(e.path)] = e.hasBacking === true })
        return map
    }
    // The piece whose tags or sharing panel is open; on the table, so adding a tag (which
    // rebuilds the rows) keeps the editor open with the keys in it (QA 14.10.).
    property string tagsPath: ""
    property string sharePath: ""

    // The tag editor of the piece just chosen takes the keys, once the menu has closed.
    signal focusTagsRequested()
    signal sortChosen(string sort)
    signal openPiece(string path)
    signal shareRequested(string path, string title, bool withAudio)

    spacing: 0

    // "today", "yesterday", "3 days ago", else the date; "" when the piece was never played.
    DayAge { id: dayAge }
    function lastPlayed(unix) {
        if (!unix) {
            return ""
        }
        var days = dayAge.days(Date.now(), unix)
        if (days <= 0) {
            return texts.t("library.last.today")
        }
        if (days === 1) {
            return texts.t("library.last.yesterday")
        }
        if (days < 30) {
            return texts.t("library.last.days", { n: days })
        }
        return texts.t("library.last.date", { date: new Date(unix * 1000).toLocaleDateString(Qt.locale(), Locale.ShortFormat) })
    }

    // The same in the few words the column has room for: "today", "3 days ago", the date.
    function lastShort(unix) {
        if (!unix) {
            return ""
        }
        var days = dayAge.days(Date.now(), unix)
        if (days <= 0) {
            return texts.t("library.last.short.today")
        }
        if (days === 1) {
            return texts.t("library.last.short.yesterday")
        }
        if (days < 30) {
            return texts.t("library.last.short.days", { n: days })
        }
        return new Date(unix * 1000).toLocaleDateString(Qt.locale(), Locale.ShortFormat)
    }

    // The rows are one Tab stop (QA 14.11., K1: 300 stops for 152 exercises): only the current
    // row and its "…" take Tab; Up, Down, Page Up, Page Down, Home and End move between rows.
    // Not the ListView's currentIndex: setting that gives the delegate the focus, away from the row.
    property int focusIndex: 0
    readonly property int stopIndex: Math.max(0, Math.min(focusIndex, list.count - 1))
    function focusRow(i) {
        var to = Math.max(0, Math.min(i, list.count - 1))
        list.positionViewAtIndex(to, ListView.Contain)
        var line = list.itemAtIndex(to)
        if (line) {
            line.focusRow()
        }
        // After the focus moved: Qt refuses to take Tab from the item that still has it.
        focusIndex = to
    }
    readonly property int pageRows: 10

    function directionText(direction) {
        return texts.t(direction === "desc" ? "library.dir.desc" : "library.dir.asc")
    }

    function sortBy(key) {
        var direction = key === sortKey ? (sortDirection === "asc" ? "desc" : "asc") : firstDirection[key]
        sortChosen(key + ":" + direction)
    }

    function groupText(key) {
        var found = view.groups.filter(function(g) { return g.key === key })[0]
        return found ? found.title + " · " + found.count : ""
    }

    function columnTitle(c) {
        if (c.sort !== sortKey) {
            return c.title
        }
        return c.title + (sortDirection === "desc" ? " ▼" : " ▲")
    }

    // Narrow: tempo and last played have no column, so the sort is a choice with every column and
    // a button for the direction.
    RowLayout {
        objectName: "sortChoice"
        visible: table.compact
        Layout.fillWidth: true
        Layout.bottomMargin: KanteStyle.unit(4)
        spacing: KanteStyle.unit(8)
        KanteComboBox {
            Layout.fillWidth: true
            Accessible.name: table.texts.t("library.sortby")
            displayText: table.texts.t("library.sortby") + ": " + table.allColumns.filter(function(c) { return c.sort === table.sortKey })[0].title
            model: table.allColumns.map(function(c) { return c.title })
            currentIndex: table.allColumns.map(function(c) { return c.sort }).indexOf(table.sortKey)
            onActivated: function(index) {
                var key = table.allColumns[index].sort
                if (key !== table.sortKey) {
                    table.sortChosen(key + ":" + table.firstDirection[key])
                }
            }
        }
        KanteButton {
            text: table.sortDirection === "desc" ? "▼" : "▲"
            emphasis: KanteButton.Emphasis.Quiet
            Accessible.name: table.texts.t("library.sortby.reverse", { direction: table.directionText(table.sortDirection) })
            onClicked: table.sortChosen(table.sortKey + ":" + (table.sortDirection === "desc" ? "asc" : "desc"))
        }
    }

    // The header: titles from KanteTableHeader, a button over each (sorting by the keyboard too).
    Item {
        Layout.fillWidth: true
        implicitHeight: header.implicitHeight + KanteStyle.unit(4)

        KanteTableHeader {
            id: header
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: KanteStyle.unit(12)
            anchors.rightMargin: KanteStyle.unit(12) + moreWidth + header.spacing
            readonly property real moreWidth: KanteStyle.unit(36)
            columns: table.columns.map(function(c) {
                return { title: table.columnTitle(c), width: c.width, align: c.align }
            })
        }

        Repeater {
            model: table.columns
            delegate: FocusScope {
                id: sorter
                required property int index
                required property var modelData
                readonly property bool active: modelData.sort === table.sortKey
                x: header.x + header.columnX(index)
                y: header.y
                width: header.columnWidth(index)
                height: header.height
                activeFocusOnTab: true
                Accessible.role: Accessible.Button
                Accessible.name: active
                    ? table.texts.t("library.col.sorted", { column: modelData.title, direction: table.directionText(table.sortDirection) })
                    : table.texts.t("library.col.sort", { column: modelData.title })
                Accessible.onPressAction: table.sortBy(modelData.sort)
                Keys.onReturnPressed: table.sortBy(modelData.sort)
                Keys.onSpacePressed: table.sortBy(modelData.sort)

                TapHandler { onTapped: table.sortBy(sorter.modelData.sort) }
                Rectangle {
                    anchors.fill: parent
                    visible: sorter.activeFocus
                    color: "transparent"
                    border.width: 2
                    border.color: KanteStyle.focusColor
                }
            }
        }
    }

    ListView {
        id: list
        objectName: "rows"
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: table.view.rows
        reuseItems: false
        QQC2.ScrollBar.vertical: QQC2.ScrollBar {}

        delegate: ColumnLayout {
            id: line
            required property int index
            required property var modelData
            readonly property bool grouped: table.view.groups.length > 0
            readonly property bool firstOfGroup: grouped && (index === 0 || table.view.rows[index - 1].group !== modelData.group)
            readonly property bool editing: table.tagsPath === String(modelData.path)
            readonly property bool sharing: table.sharePath === String(modelData.path)
            readonly property bool hasBacking: table.backed[String(modelData.path)] === true
            readonly property bool played: modelData.bestPercent !== null && modelData.bestPercent !== undefined
            readonly property string last: table.lastPlayed(modelData.lastPlayed)
            readonly property string originText: table.texts.t("library.badge." + modelData.origin)
            width: ListView.view.width
            spacing: 0
            readonly property bool stop: index === table.stopIndex
            function focusRow() {
                row.forceActiveFocus()
            }

            KanteSectionLabel {
                visible: line.firstOfGroup
                Layout.fillWidth: true
                Layout.topMargin: KanteStyle.unit(12)
                Layout.leftMargin: KanteStyle.unit(12)
                text: table.groupText(line.modelData.group)
                rule: true
                Accessible.role: Accessible.Heading
                Accessible.name: text
            }

            KanteListRow {
                id: row
                objectName: "row"
                Layout.fillWidth: true
                density: KanteListRow.Density.Comfortable
                text: line.modelData.title
                // Artist and format, a twin's file name, tags; a missing file in words.
                subtitle: [line.modelData.artist, line.modelData.format]
                    .concat(table.titleCounts[line.modelData.title] > 1 ? [String(line.modelData.path).split(/[\/\\]/).pop()] : [])
                    .concat(line.modelData.tags.map(function(t) { return "#" + t }))
                    .concat(line.modelData.missing ? [table.texts.t("library.row.missing")] : [])
                    .filter(function(s) { return s !== "" }).join(" · ")
                // What the columns show, for a screen reader (and the columns left out when narrow).
                Accessible.description: [subtitle, line.originText,
                    line.modelData.bpm ? Math.round(line.modelData.bpm) + " BPM" : "", line.last,
                    line.played ? table.texts.t("library.row.result", { percent: line.modelData.bestPercent }) : table.texts.t("library.unplayed")]
                    .filter(function(s) { return s !== "" }).join(", ")
                onClicked: table.openPiece(String(line.modelData.path))
                activeFocusOnTab: line.stop
                Keys.onDownPressed: table.focusRow(line.index + 1)
                Keys.onUpPressed: table.focusRow(line.index - 1)
                Keys.onPressed: function(event) {
                    var moves = { [Qt.Key_PageDown]: table.pageRows, [Qt.Key_PageUp]: -table.pageRows }
                    if (event.key === Qt.Key_Home) {
                        table.focusRow(0)
                    } else if (event.key === Qt.Key_End) {
                        table.focusRow(list.count - 1)
                    } else if (moves[event.key] !== undefined) {
                        table.focusRow(line.index + moves[event.key])
                    } else {
                        return
                    }
                    event.accepted = true
                }

                trailing: Row {
                    spacing: header.spacing
                    Repeater {
                        model: table.columns.slice(1)
                        delegate: Item {
                            id: cell
                            required property var modelData
                            width: modelData.width
                            height: KanteStyle.unit(24)

                            // One sign, the word, a colour of its own: the sign and the word tell it
                            // without the colour.
                            Rectangle {
                                visible: cell.modelData.id === "origin"
                                anchors.verticalCenter: parent.verticalCenter
                                width: Math.min(badge.implicitWidth + KanteStyle.unit(16), parent.width)
                                height: KanteStyle.unit(24)
                                color: "transparent"
                                border.width: 1
                                border.color: table.tones[line.modelData.origin]
                                QQC2.Label {
                                    id: badge
                                    anchors.fill: parent
                                    anchors.leftMargin: KanteStyle.unit(8)
                                    anchors.rightMargin: KanteStyle.unit(8)
                                    verticalAlignment: Text.AlignVCenter
                                    elide: Text.ElideRight
                                    text: table.signs[line.modelData.origin] + " " + line.originText
                                    color: table.tones[line.modelData.origin]
                                    font: KanteStyle.active ? KanteStyle.labelFont() : font
                                }
                            }
                            QQC2.Label {
                                visible: cell.modelData.id === "tempo"
                                anchors.fill: parent
                                horizontalAlignment: Text.AlignRight
                                verticalAlignment: Text.AlignVCenter
                                text: line.modelData.bpm ? String(Math.round(line.modelData.bpm)) : "—"
                                color: KanteStyle.mutedTextColor
                            }
                            QQC2.Label {
                                visible: cell.modelData.id === "last"
                                anchors.fill: parent
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideRight
                                text: line.last !== "" ? table.lastShort(line.modelData.lastPlayed) : "—"
                                color: KanteStyle.mutedTextColor
                            }
                            RowLayout {
                                visible: cell.modelData.id === "result"
                                anchors.fill: parent
                                spacing: KanteStyle.unit(6)
                                KanteProgressBar {
                                    visible: line.played
                                    Layout.fillWidth: true
                                    value: line.played ? line.modelData.bestPercent / 100 : 0
                                }
                                QQC2.Label {
                                    text: line.played ? line.modelData.bestPercent + " %" : "—"
                                    color: KanteStyle.mutedTextColor
                                }
                            }
                        }
                    }
                    KanteToolButton {
                        id: moreButton
                        objectName: "more"
                        activeFocusOnTab: line.stop
                        width: header.moreWidth
                        text: "…"
                        Accessible.name: table.texts.t("library.row.more", { title: line.modelData.title })
                        onClicked: {
                            moreMenu.piece = line.modelData
                            moreMenu.popup(moreButton, 0, moreButton.height)
                        }
                    }
                }
            }

            KanteTagPicker {
                id: tagPicker
                Layout.fillWidth: true
                visible: line.editing
                // Rebuilt after a tag was added: the keys go back into the field.
                Component.onCompleted: if (line.editing) search.forceActiveFocus()
                Connections {
                    target: table
                    function onFocusTagsRequested() {
                        if (line.editing) {
                            tagPicker.search.forceActiveFocus()
                        }
                    }
                }
                tags: line.modelData.tags
                suggestions: line.editing ? JSON.parse(table.ctl.tagsJson()) : []
                placeholderText: table.texts.t("library.tags.add")
                removeText: table.texts.t("library.tags.remove")
                Accessible.name: table.texts.t("library.tags.edit", { title: line.modelData.title })
                onEdited: function(tags) { table.ctl.setTags(String(line.modelData.path), JSON.stringify(tags)) }
            }

            // Audio goes along only when chosen here (J-D2).
            RowLayout {
                visible: line.sharing
                Layout.fillWidth: true
                Layout.leftMargin: KanteStyle.unit(16)
                QQC2.CheckBox {
                    id: withAudio
                    enabled: line.hasBacking
                    text: line.hasBacking ? table.texts.t("package.audio") : table.texts.t("package.audio.none")
                    Accessible.name: text
                }
                Item { Layout.fillWidth: true }
                KanteButton {
                    text: table.texts.t("package.save")
                    emphasis: KanteButton.Emphasis.Primary
                    onClicked: table.shareRequested(String(line.modelData.path), line.modelData.title, withAudio.checked)
                }
            }
        }
    }

    QQC2.Menu {
        id: moreMenu
        property var piece: ({ path: "", title: "" })
        readonly property string path: String(piece.path)
        property bool tagsChosen: false
        onClosed: {
            if (tagsChosen) {
                tagsChosen = false
                table.focusTagsRequested()
            }
        }
        KantePopupSkin { popup: moreMenu }
        KanteMenuItem {
            text: table.texts.t("library.tags")
            Accessible.name: table.texts.t("library.tags.edit", { title: moreMenu.piece.title })
            onTriggered: {
                table.tagsPath = table.tagsPath === moreMenu.path ? "" : moreMenu.path
                moreMenu.tagsChosen = table.tagsPath !== ""
            }
        }
        // LH-BIB-07: share the piece as a .kontra package.
        KanteMenuItem {
            text: table.texts.t("package.share")
            Accessible.name: table.texts.t("package.share.name", { title: moreMenu.piece.title })
            onTriggered: table.sharePath = table.sharePath === moreMenu.path ? "" : moreMenu.path
        }
    }
}
