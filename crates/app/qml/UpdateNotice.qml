import QtQuick
import Kante

// Update notice (LH-ALL-07): a newer release exists. A link to its page, no download; "×" hides
// it until the next release. Appears only after the check at start has found one.
KanteCallout {
    id: notice

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var found: JSON.parse(ctl.updateNotice("poll"))

    visible: found.version !== ""
    kind: KanteCallout.Kind.Info
    dismissible: true
    autoHide: false
    title: texts.t("update.title", { version: found.version })
    text: texts.t("update.text")
    onDismissed: found = JSON.parse(ctl.updateNotice("dismiss"))
    // The request runs in the background; its answer is taken in here.
    Timer {
        interval: 1000
        repeat: true
        running: notice.found.pending
        onTriggered: notice.found = JSON.parse(notice.ctl.updateNotice("poll"))
    }
    KanteButton {
        text: notice.texts.t("update.open")
        onClicked: Qt.openUrlExternally(notice.found.url)
    }
}
