import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import Kante

// Songs from the web (LH-BIB-11, J-B6) as a dialog: the search, the browser buttons and what
// the browser saved lately. Kontra downloads no tabs.
KanteDialog {
    id: dialog

    required property var ctl
    required property var texts

    // A piece was chosen under "New in Downloads"; the dialog has closed.
    signal openPath(string path)

    parent: QQC2.Overlay.overlay
    anchors.centerIn: parent
    modal: true
    size: KanteDialog.Size.Large
    title: texts.t("search.title")
    standardButtons: QQC2.Dialog.Close

    QQC2.ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth
        // The sources refresh only while the dialog is open.
        SongSources {
            width: parent.width
            ctl: dialog.ctl
            texts: dialog.texts
            active: dialog.visible
            onOpenPath: function(path) {
                dialog.close()
                dialog.openPath(path)
            }
        }
    }
}
