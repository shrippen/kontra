import QtQuick
import QtTest
import ".."

// Library "played N days ago": calendar days in local time, not spans of 24 hours (PR #299).
// Run by scripts/check-video.sh.
Item {
    DayAge { id: age }

    TestCase {
        name: "DayAge"

        // Local time, so the test holds in any time zone.
        function unix(y, m, d, h, min) {
            return new Date(y, m - 1, d, h, min).getTime() / 1000
        }
        function ms(y, m, d, h, min) {
            return new Date(y, m - 1, d, h, min).getTime()
        }

        function test_across_midnight() {
            // Yesterday 23:00 seen at 08:00: nine hours, but yesterday.
            compare(age.days(ms(2026, 11, 12, 8, 0), unix(2026, 11, 11, 23, 0)), 1)
            // This morning 00:30 seen at 23:30: today.
            compare(age.days(ms(2026, 11, 12, 23, 30), unix(2026, 11, 12, 0, 30)), 0)
            // Just over 24 hours but two calendar days apart.
            compare(age.days(ms(2026, 11, 12, 1, 0), unix(2026, 11, 10, 23, 0)), 2)
        }

        function test_month_and_dst() {
            compare(age.days(ms(2026, 11, 2, 12, 0), unix(2026, 10, 31, 12, 0)), 2)
            // The clocks change on the last Sunday of October and March in Europe.
            compare(age.days(ms(2026, 10, 27, 12, 0), unix(2026, 10, 24, 12, 0)), 3)
            compare(age.days(ms(2026, 3, 30, 12, 0), unix(2026, 3, 27, 12, 0)), 3)
        }
    }
}
