import QtQuick
import QtQuick.Layouts
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

    // Every page has its own heading; Kirigami's title bar above it was empty or said the same
    // again and took 45 px (QA 14.10., a beamer at 150 % has 480).
    pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.None

    property alias ctl: ctl
    property alias texts: texts
    property string current: ""
    // Presentation mode of the practice page (LH-ANZ-09): full screen, no sidebar.
    property bool presenting: false
    property int visibilityBefore: Window.Windowed
    property bool drawerBefore: false
    // Narrow windows take the sidebar as an overlay, closed; wide ones show it. Following the
    // width both ways: closing once at start hid it for good when the window became wide only
    // after loading (QA 14.10.).
    onWideScreenChanged: wideScreen ? globalDrawer.open() : globalDrawer.close()

        onPresentingChanged: {
        if (presenting) {
            drawerBefore = globalDrawer.drawerOpen
            globalDrawer.close()
            visibilityBefore = visibility
            showFullScreen()
            return
        }
        visibility = visibilityBefore === Window.FullScreen ? Window.Windowed : visibilityBefore
        // The sidebar comes back as it was before (QA 27.10., P4: gone after Esc); a window that
        // turns narrow later closes it again (onWideScreenChanged).
        if (drawerBefore) {
            globalDrawer.open()
        }
    }
    readonly property var pages: ({
        home: Qt.resolvedUrl("StartPage.qml"),
        setup: Qt.resolvedUrl("SetupPage.qml"),
        progress: Qt.resolvedUrl("ProgressPage.qml"),
        exercises: Qt.resolvedUrl("ExercisePage.qml"),
        quiz: Qt.resolvedUrl("QuizPage.qml"),
        ear: Qt.resolvedUrl("EarPage.qml"),
        learn: Qt.resolvedUrl("LearnPage.qml"),
        lesson: Qt.resolvedUrl("LessonPage.qml"),
        review: Qt.resolvedUrl("ReviewPage.qml"),
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
            offpitch: texts.t("result.offpitch"),
            missed: texts.t("result.missed"),
            early: texts.t("result.early"),
            late: texts.t("result.late")
        })
    }
    // Technique signs (LH-BEW-06, Kante 1.31): names for screen readers and legends.
    Binding {
        target: KanteStyle
        property: "techniqueNames"
        value: ({
            slide: texts.t("technique.slide"),
            hammer: texts.t("technique.hammer"),
            pull: texts.t("technique.pull"),
            slap: texts.t("technique.slap"),
            pop: texts.t("technique.pop"),
            ghost: texts.t("technique.ghost"),
            dead: texts.t("technique.dead")
        })
    }
    Binding {
        target: KanteStyle
        property: "techniqueResultNames"
        value: ({
            played: texts.t("technique.result.played"),
            other: texts.t("technique.result.other"),
            unheard: texts.t("technique.result.unheard")
        })
    }
    // Rehearsal marks for screen readers (Kante 1.50): "Teil Refrain · B".
    Binding { target: KanteStyle; property: "partText"; value: texts.t("lane.part") }
    // String colours (LH-ANZ-06): Kante's palettes ("kante", "mono", "bright"); lane, staff and
    // fretboard take them from KanteStyle (Kante 1.28). States keep their own colours and shapes.
    Binding { target: KanteStyle; property: "stringPalette"; value: ctl.stringColors }
    // Round 6: follow the desktop (animations off there means durations 0), or the own switch.
    readonly property bool systemReducesMotion: Kirigami.Units.longDuration === 0
    Binding { target: KanteStyle; property: "motion"; value: ctl.motion !== "reduce" }
    KanteScope { target: root.contentItem }
    // Its labels take the highlighted text colour from Kirigami's theme.
    KanteScope { target: root.globalDrawer ? root.globalDrawer.contentItem : null }
    color: KanteStyle.themed ? KanteStyle.backgroundColor : Kirigami.Theme.backgroundColor

    // The lesson open on the lesson page and its step, kept while the tuner or an exercise runs.
    property string lessonId: ""
    property int lessonPage: 0
    // The exercise level to open first (setup "Ich spiele schon": the basics); -1 = the plan's.
    property int exerciseLevel: -1
    // The chapter "Kann ich schon" checks on the review page ("" = the questions due).
    property string checkChapter: ""

    // A glossary entry ("" the index) as a layer over the page that asked (J-G2). The page loses
    // its scroll position under the layer; it is put back when the last layer closes.
    property real glossaryReturnY: -1
    // The control that opened it ("?" or link) gets the focus back (QA 28.10. B5: it was gone).
    property Item glossaryReturnFocus: null
    property int glossaryReturnReason: Qt.OtherFocusReason
    function glossary(id) {
        var under = pageStack.currentItem
        if (pageStack.layers.depth === 1 && under && under.flickable) {
            glossaryReturnY = under.flickable.contentY
        }
        if (pageStack.layers.depth === 1) {
            glossaryReturnFocus = activeFocusItem
            glossaryReturnReason = activeFocusItem ? activeFocusItem.focusReason : Qt.OtherFocusReason
        }
        pageStack.layers.push(Qt.resolvedUrl("GlossaryPage.qml"), { termId: id })
    }
    Connections {
        target: pageStack.layers
        function onDepthChanged() {
            if (pageStack.layers.depth > 1) {
                return
            }
            var under = pageStack.currentItem
            var y = root.glossaryReturnY
            var opener = root.glossaryReturnFocus
            var reason = root.glossaryReturnReason
            root.glossaryReturnY = -1
            root.glossaryReturnFocus = null
            // Scroll first, then focus: the opener lies in view at the old position.
            Qt.callLater(function() {
                if (y >= 0 && under && under.flickable) {
                    under.flickable.contentY = y
                }
                if (opener) {
                    opener.forceActiveFocus(reason)
                }
            })
        }
    }

    function show(name) {
        if ((current === "tuner" || current === "setup" || current === "quiz") && name !== current) {
            ctl.stopTuner()
        }
        // Kirigami creates pages from a URL without a visual parent, and Qt 6.12 warns ("not
        // placed in the graphics scene"); a page created here has one. Pages pushed as objects
        // are not destroyed by the page row, so the old one is.
        var old = pageStack.currentItem
        var component = Qt.createComponent(pages[name])
        if (component.status !== Component.Ready) {
            console.warn(component.errorString())
            return
        }
        // Another page: no scroll position or focus to put back.
        glossaryReturnY = -1
        glossaryReturnFocus = null
        while (pageStack.layers.depth > 1) {
            pageStack.layers.pop()
        }
        // Only now: on a load error the drawer kept marking a page that never showed.
        current = name
        var page = component.createObject(pageStack)
        if (pageStack.depth === 0) {
            pageStack.push(page)
        } else {
            pageStack.replace(page)
        }
        if (old) {
            old.destroy()
        }
    }

    globalDrawer: Kirigami.GlobalDrawer {
        // In the presentation the sidebar leaves: "visible: false" alone kept its room and
        // showed it (QA 14.10., P4 at the beamer). Modal and closed, it takes none.
        isMenu: false
        modal: !root.wideScreen || root.presenting
        // Its handle lay over the transport's time in the presentation.
        handleVisible: !root.presenting && modal
        collapsible: true
        collapsed: !root.wideScreen
        // A popup outside the content: it marked the page in the desktop's highlight colour
        // (QA 27.10.: green). Kante's selection colour, as on segmented buttons.
        palette.highlight: KanteStyle.themed ? KanteStyle.accentColor : root.palette.highlight
        palette.highlightedText: KanteStyle.themed ? KanteStyle.accentForegroundColor : root.palette.highlightedText
        // Grouped by what the player does: start; play pieces; learn; look back; tools. Setup lives
        // in the settings and on the start page.
        actions: [
            Kirigami.Action { text: texts.t("nav.home"); icon.name: "go-home"; checked: root.current === "home"; shortcut: "Ctrl+1"; onTriggered: root.show("home") },
            NavSeparator {},
            Kirigami.Action { text: texts.t("nav.library"); icon.name: "folder-music"; checked: root.current === "library"; shortcut: "Ctrl+2"; onTriggered: root.show("library") },
            Kirigami.Action { text: texts.t("nav.practice"); icon.name: "media-playback-start"; enabled: ctl.loaded; checked: root.current === "practice"; shortcut: "Ctrl+3"; onTriggered: root.show("practice") },
            Kirigami.Action { text: texts.t("nav.exercises"); icon.name: "view-media-playlist"; checked: root.current === "exercises"; shortcut: "Ctrl+4"; onTriggered: root.show("exercises") },
            NavSeparator {},
            Kirigami.Action { text: texts.t("nav.learn"); icon.name: "applications-education"; checked: root.current === "learn" || root.current === "lesson" || root.current === "review"; shortcut: "Ctrl+5"; onTriggered: root.show("learn") },
            Kirigami.Action { text: texts.t("nav.quiz"); icon.name: "games-config-board"; checked: root.current === "quiz"; shortcut: "Ctrl+6"; onTriggered: root.show("quiz") },
            Kirigami.Action { text: texts.t("nav.ear"); icon.name: "audio-headphones"; checked: root.current === "ear"; shortcut: "Ctrl+7"; onTriggered: root.show("ear") },
            NavSeparator {},
            Kirigami.Action { text: texts.t("nav.progress"); icon.name: "view-statistics"; checked: root.current === "progress"; shortcut: "Ctrl+8"; onTriggered: root.show("progress") },
            NavSeparator {},
            Kirigami.Action { text: texts.t("nav.tuner"); icon.name: "audio-input-line"; checked: root.current === "tuner"; shortcut: "Ctrl+9"; onTriggered: root.show("tuner") },
            Kirigami.Action { text: texts.t("nav.settings"); icon.name: "settings-configure"; checked: root.current === "settings" || root.current === "setup"; shortcut: "Ctrl+0"; onTriggered: root.show("settings") }
        ]
    }

    DrawerFocus { drawer: root.globalDrawer }

    // Audio and session are polled here, ~60 times a second.
    Timer {
        interval: 16
        running: true
        repeat: true
        onTriggered: ctl.tick()
    }

    // Notes for the user sit in a bar under the page, not over it: the page gets shorter, nothing
    // is hidden or clickable behind a note (QA 14.10.: the latency note covered the transport bar
    // and was see-through).
    // In the content area beside the sidebar (a window footer started under the sidebar); the
    // page stack ends above it.
    Binding { target: root.pageStack; property: "anchors.bottomMargin"; value: noteBar.visible ? noteBar.height : 0 }
    NoteBar {
        id: noteBar
        parent: root.pageStack.parent
        anchors { left: root.pageStack.left; right: root.pageStack.right; bottom: parent.bottom }
        z: 10
        ctl: ctl
        texts: texts
    }

    // QA (P7): --focus-log writes each keyboard focus change, to find stops nobody can see.
    onActiveFocusItemChanged: {
        if (Qt.application.arguments.indexOf("--focus-log") < 0 || !activeFocusItem)
            return
        var it = activeFocusItem
        var r = it.mapToItem(null, 0, 0)
        console.warn("focus", it, it.Accessible.name || it.text || "", Math.round(r.x), Math.round(r.y),
                    Math.round(it.width), Math.round(it.height), it.visible)
    }

    // QA and release screenshots: --screenshot PATH [--screenshot-after MS].
    Timer {
        interval: Math.max(100, ctl.screenshotAfter)
        running: ctl.screenshotPath !== ""
        onTriggered: {
            // --screenshot-notes: the note bar under the page instead (QA; the window itself cannot
            // be grabbed).
            var notes = Qt.application.arguments.indexOf("--screenshot-notes") >= 0
            var item = notes ? noteBar : (root.pageStack.currentItem || root.contentItem)
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

    // Space starts and stops everywhere except in text fields and on a button reached by keyboard,
    // which Space presses (LH-MOD-07; QA 27.10., P7: Space on "practise bar 7 in the workshop" in
    // the summary started a run instead). A clicked button keeps focus but not visual focus.
    readonly property bool spaceFree: {
        var item = activeFocusItem
        return !(item && ((item instanceof QQC2.AbstractButton && item.visualFocus) || item.hasOwnProperty("cursorPosition")))
    }
    Shortcut {
        sequence: "Space"
        enabled: ctl.loaded && root.current === "practice" && root.spaceFree
        onActivated: ctl.togglePlay()
    }

    Component.onCompleted: {
        // No --page: the assistant on first start, else practice for an opened file or the start page.
        var first = ctl.startPage
        if (first === "") {
            first = !ctl.setupDone && !ctl.loaded ? "setup" : (ctl.loaded ? "practice" : "home")
        }
        // First start with animations off on the desktop: the page-wise tab, not the lane.
        if (!ctl.setupDone && ctl.motion === "system" && systemReducesMotion && ctl.view === "lane")
            ctl.changeView("tab")
        show(first)
        if (ctl.startLoop !== "" && ctl.loaded) {
            var bars = ctl.startLoop.split("-")
            ctl.loopBars(Number(bars[0]) - 1, Number(bars[1]) - 1)
        }
        if (ctl.autoprobe && ctl.loaded) {
            ctl.startProbe()
        } else if (ctl.autoplay && ctl.loaded) {
            ctl.togglePlay()
        }
        if (ctl.autocalibrate) {
            ctl.startCalibration()
        }
    }
}
