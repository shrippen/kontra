import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante
import org.shrippen.kontra

Kirigami.ApplicationWindow {
    id: root

    title: texts.t("app.name") + (ctl.loaded ? " – " + ctl.title : "")
    width: Kirigami.Units.gridUnit * 72
    height: Kirigami.Units.gridUnit * 44
    minimumWidth: Kirigami.Units.gridUnit * 26
    minimumHeight: Kirigami.Units.gridUnit * 22

    property alias ctl: ctl
    property alias texts: texts
    property string current: ""
    readonly property var pages: ({
        library: Qt.resolvedUrl("LibraryPage.qml"),
        practice: Qt.resolvedUrl("PracticePage.qml"),
        tuner: Qt.resolvedUrl("TunerPage.qml"),
        settings: Qt.resolvedUrl("SettingsPage.qml")
    })

    Controller { id: ctl }
    Texts { id: texts; source: ctl.texts }

    // Kante is the default style; Kante Light and System (Breeze) are opt-in (settings).
    Binding {
        target: KanteStyle
        property: "kind"
        value: ctl.style === "kante-light" ? KanteStyle.Kind.KanteLight
             : ctl.style === "system" ? KanteStyle.Kind.System : KanteStyle.Kind.Kante
    }
    // Kante is dark, Kante Light is light (like lmix); System follows the platform.
    Binding {
        target: KanteStyle
        property: "preferDark"
        value: ctl.style === "kante"
    }
    Binding {
        target: KanteStyle
        property: "noteStateNames"
        value: ({
            pending: "",
            hit: texts.t("result.hit"),
            wrong: texts.t("result.wrong"),
            missed: texts.t("result.missed"),
            early: texts.t("result.early"),
            late: texts.t("result.late")
        })
    }
    KanteScope { target: root.contentItem }
    color: KanteStyle.themed ? KanteStyle.backgroundColor : Kirigami.Theme.backgroundColor

    function show(name) {
        if (current === "tuner" && name !== "tuner") {
            ctl.stopTuner()
        }
        current = name
        if (pageStack.depth === 0) {
            pageStack.push(pages[name])
        } else {
            pageStack.replace(pages[name])
        }
    }

    globalDrawer: Kirigami.GlobalDrawer {
        isMenu: false
        modal: !root.wideScreen
        collapsible: true
        collapsed: !root.wideScreen
        actions: [
            Kirigami.Action { text: texts.t("nav.library"); icon.name: "folder-music"; checked: root.current === "library"; onTriggered: root.show("library") },
            Kirigami.Action { text: texts.t("nav.practice"); icon.name: "media-playback-start"; enabled: ctl.loaded; checked: root.current === "practice"; onTriggered: root.show("practice") },
            Kirigami.Action { text: texts.t("nav.tuner"); icon.name: "audio-input-line"; checked: root.current === "tuner"; onTriggered: root.show("tuner") },
            Kirigami.Action { text: texts.t("nav.settings"); icon.name: "settings-configure"; checked: root.current === "settings"; onTriggered: root.show("settings") }
        ]
    }

    // Audio and session are polled here, ~60 times a second.
    Timer {
        interval: 16
        running: true
        repeat: true
        onTriggered: ctl.tick()
    }

    KanteCallout {
        id: notice
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: KanteStyle.unit(12) }
        z: 10
        visible: text !== ""
        kind: ctl.errorText !== "" ? KanteCallout.Kind.Danger : KanteCallout.Kind.Info
        text: ctl.errorText !== "" ? ctl.errorText : ctl.message
        dismissible: true
        autoHide: false
        onDismissed: ctl.dismissMessage()
    }

    // QA and release screenshots: --screenshot PATH [--screenshot-after MS].
    Timer {
        interval: Math.max(100, ctl.screenshotAfter)
        running: ctl.screenshotPath !== ""
        onTriggered: {
            var item = root.pageStack.currentItem || root.contentItem
            item.grabToImage(function(result) {
                result.saveToFile(ctl.screenshotPath)
                if (ctl.quitAfter <= 0) {
                    Qt.quit()
                }
            })
        }
    }
    Timer {
        interval: Math.max(100, ctl.quitAfter)
        running: ctl.quitAfter > 0
        onTriggered: Qt.quit()
    }

    Component.onCompleted: {
        show(ctl.loaded && ctl.startPage === "library" ? "practice" : ctl.startPage)
        if (ctl.autoplay && ctl.loaded) {
            ctl.togglePlay()
        }
        if (ctl.autocalibrate) {
            ctl.startCalibration()
        }
    }
}
