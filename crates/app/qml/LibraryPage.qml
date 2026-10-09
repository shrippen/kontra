import QtQuick
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
    readonly property var recent: JSON.parse(ctl.recentJson).filter(function(e) {
        var q = query.trim().toLowerCase()
        return q === "" || (e.title + " " + e.artist + " " + e.path).toLowerCase().indexOf(q) >= 0
    })
    readonly property var tracks: JSON.parse(ctl.trackChoicesJson)

    // Kirigami pages paint the platform background; Kante's ground replaces it.
    background: Rectangle { color: KanteStyle.backgroundColor }

    title: texts.t("nav.library")

    function open(url) {
        if (ctl.openFile(url)) {
            applicationWindow().show("practice")
        }
    }

    FileDialog {
        id: dialog
        nameFilters: [texts.t("library.formats") + " (*.gp *.gp3 *.gp4 *.gp5 *.gpx *.mid *.midi *.musicxml *.xml *.mxl)"]
        onAccepted: page.open(selectedFile.toString())
    }

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        RowLayout {
            Layout.fillWidth: true
            KanteHeading { text: texts.t("nav.library"); pageTitle: true; Layout.fillWidth: true }
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
            KanteSectionLabel { text: texts.t("library.recent"); Layout.fillWidth: true }
            Repeater {
                model: page.recent
                delegate: KanteListRow {
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData.title
                    subtitle: modelData.artist
                    meta: modelData.best !== null && modelData.best !== undefined
                          ? texts.t("library.best", { percent: Math.round(modelData.best * 100) })
                            + " · " + texts.t("library.minutes", { minutes: Math.round(modelData.practiceMin) })
                          : String(modelData.path).split("/").pop()
                    onClicked: page.open(String(modelData.path))
                }
            }
        }
    }
}
