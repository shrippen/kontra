import QtQuick

// How many calendar days (local time) lie between a moment and now: 0 today, 1 yesterday.
// Not spans of 24 hours: yesterday 23:00 is "yesterday" at 08:00 as well.
QtObject {
    readonly property int msPerDay: 86400000

    function startOfDay(ms) {
        var d = new Date(ms)
        return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
    }

    // `nowMs` in milliseconds, `thenUnix` in seconds. Rounded: a day with a clock change has 23
    // or 25 hours.
    function days(nowMs, thenUnix) {
        return Math.round((startOfDay(nowMs) - startOfDay(thenUnix * 1000)) / msPerDay)
    }
}
