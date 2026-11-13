import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import Kante

// A folder was chosen (LH-BIB-17, LH-BIB-18): link it (read in place, nothing copied), import its
// pieces once, or import them as a setlist (J-B1). Link is the default.
KanteDialog {
    id: dialog

    required property var ctl
    required property var texts
    // The chosen folder as a URL, and its name for the dialog.
    property string folderUrl: ""
    readonly property string folderName: decodeURIComponent(folderUrl.split("/").filter(function(n) { return n !== "" }).pop() || "")
    readonly property var ways: ["link", "import", "setlist"]
    property int way: 0

    function ask(url) {
        folderUrl = url
        way = 0
        open()
    }

    parent: QQC2.Overlay.overlay
    anchors.centerIn: parent
    modal: true
    size: KanteDialog.Size.Small
    title: texts.t("library.folder.how.title")
    standardButtons: QQC2.Dialog.Ok | QQC2.Dialog.Cancel
    acceptText: texts.t("library.folder.how.add")
    onAccepted: {
        if (ways[way] === "link") {
            ctl.linkFolder(folderUrl)
            return
        }
        ctl.importFolder(folderUrl, ways[way] === "setlist")
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: KanteStyle.unit(12)
        QQC2.Label {
            Layout.fillWidth: true
            text: dialog.folderName
            color: KanteStyle.strongTextColor
            font.bold: true
            elide: Text.ElideMiddle
        }
        KanteSegmented {
            Accessible.name: dialog.texts.t("library.folder.how.title")
            model: dialog.ways.map(function(w) { return dialog.texts.t("library.folder.how." + w) })
            currentIndex: dialog.way
            onActivated: function(index) { dialog.way = index }
        }
        QQC2.Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: dialog.texts.t("library.folder.how." + dialog.ways[dialog.way] + ".hint")
            color: KanteStyle.mutedTextColor
        }
    }
}
