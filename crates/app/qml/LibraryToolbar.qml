import QtQuick
import QtQuick.Layouts
import Kante

// The library's tools above the table: source choice (narrow only, the tree is hidden then),
// search, grouping and "Add pieces". Narrow, search and grouping each get a line below, so every
// function stays reachable.
//
//   wide    [Search ………………] [Group by: None ▾] [Add pieces ▾]
//   narrow  [Source ▾] ………… [Add pieces ▾]
//           [Search ………………………]
//           [Group by: None ▾]
ColumnLayout {
    id: bar

    required property var texts
    property bool slim: false
    property bool isSetlist: false
    property string group: "none"
    property var groups: []
    // Sources as choice entries (items and actions of the tree) and the one shown.
    property var sourceItems: []
    property string source: "all"
    // The search words the table is filtered by; a field built anew shows them.
    property string query: ""

    signal queryEdited(string text)
    signal groupChosen(string key)
    signal sourceChosen(var entry)
    signal addRequested(Item button)

    spacing: KanteStyle.unit(8)

    Component {
        id: groupChoice
        KanteComboBox {
            Accessible.name: bar.texts.t("library.group")
            displayText: bar.texts.t("library.group") + ": " + bar.texts.t("library.group." + bar.group)
            model: bar.groups.map(function(g) { return bar.texts.t("library.group." + g) })
            currentIndex: bar.groups.indexOf(bar.group)
            onActivated: function(index) { bar.groupChosen(bar.groups[index]) }
        }
    }

    Component {
        id: searchField
        KanteTextField {
            placeholderText: bar.texts.t("library.search")
            Accessible.name: bar.texts.t("library.search")
            // Shows the words the table is filtered by; typing reports them (a field built anew
            // is not empty while the table stays filtered).
            text: bar.query
            onTextEdited: bar.queryEdited(text)
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: KanteStyle.unit(8)

        KanteComboBox {
            objectName: "sourceChoice"
            visible: bar.slim
            Layout.preferredWidth: KanteStyle.unit(180)
            Accessible.name: bar.texts.t("library.source.pick")
            model: bar.sourceItems.map(function(e) { return (e.sign ? e.sign + " " : "") + e.text })
            currentIndex: bar.sourceItems.map(function(e) { return e.key }).indexOf(bar.source)
            onActivated: function(index) { bar.sourceChosen(bar.sourceItems[index]) }
        }
        Loader {
            objectName: "searchWide"
            active: !bar.isSetlist && !bar.slim
            visible: active
            Layout.fillWidth: true
            sourceComponent: searchField
            onLoaded: item.width = Qt.binding(function() { return width })
        }
        Item { Layout.fillWidth: true; visible: bar.isSetlist || bar.slim }
        Loader {
            objectName: "groupWide"
            active: !bar.isSetlist && !bar.slim
            visible: active
            Layout.preferredWidth: KanteStyle.unit(220)
            sourceComponent: groupChoice
            onLoaded: item.width = Qt.binding(function() { return width })
        }
        KanteButton {
            id: addButton
            text: bar.texts.t("library.add") + " ▾"
            emphasis: KanteButton.Emphasis.Primary
            Accessible.name: bar.texts.t("library.add")
            onClicked: bar.addRequested(addButton)
        }
    }

    Loader {
        objectName: "searchNarrow"
        active: !bar.isSetlist && bar.slim
        visible: active
        Layout.fillWidth: true
        sourceComponent: searchField
        onLoaded: item.width = Qt.binding(function() { return width })
    }
    Loader {
        objectName: "groupNarrow"
        active: !bar.isSetlist && bar.slim
        visible: active
        Layout.fillWidth: true
        sourceComponent: groupChoice
        onLoaded: item.width = Qt.binding(function() { return width })
    }
}
