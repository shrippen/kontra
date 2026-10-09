import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import Kante

// Progress (LH-FOR-01, journey J-C1): practice time per day and what each piece has reached.
// Encouraging, no ranking: it compares the player only with themselves.
Kirigami.ScrollablePage {
    id: page

    readonly property var ctl: applicationWindow().ctl
    readonly property var texts: applicationWindow().texts
    property var data: ({ pieces: [], runs: [] })
    readonly property int dayCount: 14

    // Minutes per local day, oldest first, the last entry is today.
    readonly property var days: {
        var out = []
        var today = new Date()
        today.setHours(0, 0, 0, 0)
        for (var i = dayCount - 1; i >= 0; i--) {
            var start = new Date(today.getTime())
            start.setDate(today.getDate() - i)
            out.push({ start: start.getTime(), minutes: 0 })
        }
        for (var r = 0; r < data.runs.length; r++) {
            var t = data.runs[r].unix * 1000
            for (var d = out.length - 1; d >= 0; d--) {
                if (t >= out[d].start) {
                    if (d === out.length - 1 || t < out[d + 1].start) out[d].minutes += data.runs[r].s / 60
                    break
                }
            }
        }
        return out
    }
    readonly property var lastWeek: days.slice(dayCount - 7)
    readonly property int weekMinutes: Math.round(lastWeek.reduce(function(s, d) { return s + d.minutes }, 0))
    readonly property int weekDays: lastWeek.filter(function(d) { return d.minutes > 0.5 }).length

    function reload() { data = JSON.parse(ctl.progressJson()) }
    function dateText(unix) {
        var date = new Date(unix * 1000)
        return date.getDate() + "." + (date.getMonth() + 1) + "."
    }

    background: Rectangle { color: KanteStyle.backgroundColor }
    title: texts.t("progress.title")
    padding: KanteStyle.unit(16)

    Component.onCompleted: reload()

    ColumnLayout {
        spacing: KanteStyle.unit(16)

        KanteHeading { text: texts.t("progress.title"); pageTitle: true }

        KanteEmptyState {
            Layout.fillWidth: true
            visible: page.data.pieces.length === 0
            title: texts.t("progress.empty.title")
            text: texts.t("progress.empty.text")
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: page.data.pieces.length > 0
            spacing: KanteStyle.unit(16)

            QQC2.Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: KanteStyle.strongTextColor
                font.pointSize: KanteStyle.defaultFont.pointSize * 1.3
                text: page.weekMinutes > 0
                    ? texts.t("progress.week", { minutes: page.weekMinutes, days: page.weekDays })
                    : texts.t("progress.week.none")
            }

            KanteSectionLabel { text: texts.t("progress.days") }
            KanteBarChart {
                Layout.fillWidth: true
                Layout.preferredHeight: KanteStyle.unit(180)
                values: page.days.map(function(d) { return Math.round(d.minutes) })
                labels: page.days.map(function(d) {
                    var date = new Date(d.start)
                    return date.getDate() + "." + (date.getMonth() + 1) + "."
                })
                highlight: page.dayCount - 1
                axis: true
                unit: "min"
                Accessible.name: texts.t("progress.days")
                Accessible.description: texts.t("progress.week", { minutes: page.weekMinutes, days: page.weekDays })
            }

            KanteSectionLabel { text: texts.t("progress.pieces") }
            Repeater {
                model: page.data.pieces
                delegate: ColumnLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: KanteStyle.unit(2)
                    QQC2.Label {
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        text: modelData.title + (modelData.artist ? " · " + modelData.artist : "")
                        color: KanteStyle.strongTextColor
                        font.bold: true
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        color: KanteStyle.textColor
                        text: [
                            modelData.best !== null ? texts.t("library.best", { percent: Math.round(modelData.best * 100) }) : "",
                            modelData.bestSpeed !== null ? texts.t("progress.speed", { percent: Math.round(modelData.bestSpeed * 100) }) : "",
                            modelData.practiceMin < 1 ? texts.t("library.underminute") : texts.t("library.minutes", { minutes: Math.round(modelData.practiceMin) }),
                            texts.t("progress.runs", { n: modelData.runs })
                        ].filter(function(s) { return s !== "" }).join(" · ")
                    }
                    // Per section (LH-FOR-01): best hit rate, its tempo and its date.
                    Repeater {
                        model: modelData.sections
                        delegate: QQC2.Label {
                            required property var modelData
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: KanteStyle.mutedTextColor
                            text: texts.t("progress.section.best", {
                                label: modelData.label,
                                percent: Math.round(modelData.best * 100),
                                speed: Math.round(modelData.speed * 100),
                                date: page.dateText(modelData.unix)
                            })
                        }
                    }
                    // Workshop tempo over the last 30 days (LH-FOR-02), per section with two runs or more.
                    Repeater {
                        model: modelData.sections.filter(function(sec) { return sec.history.length > 1 })
                        delegate: ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Layout.topMargin: KanteStyle.unit(8)
                            spacing: KanteStyle.unit(4)
                            QQC2.Label {
                                text: texts.t("progress.history") + " · " + modelData.label
                                color: KanteStyle.textColor
                            }
                            KanteLineChart {
                                Layout.fillWidth: true
                                Layout.preferredHeight: KanteStyle.unit(140)
                                axis: true
                                unit: "%"
                                minValue: 0
                                series: [
                                    modelData.history.map(function(h) { return Math.round(h.speed * 100) }),
                                    modelData.history.map(function(h) { return Math.round(h.hitRate * 100) })
                                ]
                                labels: modelData.history.map(function(h) { return page.dateText(h.unix) })
                                Accessible.name: texts.t("progress.history") + " · " + modelData.label
                                Accessible.description: modelData.history.map(function(h) {
                                    return page.dateText(h.unix) + ": " + Math.round(h.speed * 100) + " %, " + Math.round(h.hitRate * 100) + " %"
                                }).join("; ")
                            }
                            // x axis: first and last run; legend for the two lines.
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: KanteStyle.unit(12)
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    color: KanteStyle.mutedTextColor
                                    text: texts.t("progress.history.axis", {
                                        from: page.dateText(modelData.history[0].unix),
                                        to: page.dateText(modelData.history[modelData.history.length - 1].unix)
                                    })
                                }
                                Repeater {
                                    model: ["progress.history.speed", "progress.history.rate"]
                                    delegate: RowLayout {
                                        required property int index
                                        required property string modelData
                                        spacing: KanteStyle.unit(4)
                                        KanteSwatch { swatchColor: KanteStyle.dataColor(index); size: KanteSwatch.Size.Small }
                                        QQC2.Label { text: texts.t(modelData); color: KanteStyle.textColor }
                                    }
                                }
                            }
                        }
                    }
                    Rectangle { Layout.fillWidth: true; Layout.topMargin: KanteStyle.unit(8); height: 1; color: KanteStyle.frameColor }
                }
            }
        }
    }
}
