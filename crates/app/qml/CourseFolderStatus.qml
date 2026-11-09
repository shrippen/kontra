import QtQuick
import Kante

// A course folder open live (LH-LRN-21, P4): which folder, what the last check found, and that
// nothing learned is stored meanwhile; "Close" goes back to the added courses.
KanteCallout {
    id: status

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    readonly property var folder: ctl.courseFolderJson !== "" ? JSON.parse(ctl.courseFolderJson) : null
    // Shown so far; the rest are counted (`kontra-cli course check` lists all).
    readonly property int shown: 5

    visible: folder !== null
    kind: folder && folder.ok ? KanteCallout.Kind.Ok : KanteCallout.Kind.Warn
    autoHide: false
    title: folder ? texts.t("courses.folder.title", { dir: folder.dir }) : ""
    text: {
        if (!folder) {
            return ""
        }
        if (folder.error) {
            return texts.t("courses.folder.error", { error: folder.error })
        }
        if (folder.problems.length > 0) {
            const more = folder.problems.length - shown
            return folder.problems.slice(0, shown).join("\n")
                + (more > 0 ? "\n" + texts.t("courses.folder.more", { count: more }) : "")
        }
        return texts.t("courses.folder.ok")
    }
    KanteButton {
        text: status.texts.t("courses.folder.close")
        onClicked: status.ctl.openCourseFolder("")
    }
}
