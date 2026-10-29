use super::*;
use crate::activity_range::ActivityRange;

fn time(input: &str) -> DateTime<Local> {
    local_time(NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S").unwrap()).unwrap()
}

fn window(input: &str) -> TimeWindow {
    let ActivityRange::Window(window) = input.parse().unwrap() else {
        panic!("{input} should parse as a time window")
    };
    window
}

#[test]
fn parses_count_relative_and_bucket_ranges() {
    assert!(matches!(
        "0".parse::<ActivityRange>().unwrap(),
        ActivityRange::Count(0)
    ));
    assert!(matches!(
        "3".parse::<ActivityRange>().unwrap(),
        ActivityRange::Count(3)
    ));
    assert!(matches!(
        "2026".parse::<ActivityRange>().unwrap(),
        ActivityRange::Count(2026)
    ));
    for input in [
        "3days",
        "2026-02",
        "2025-02-03..2026-01-16",
        "2026..2026",
        "2026-01-01..7d",
        "3m..1m",
        "..3d",
        "3d..",
        "..",
    ] {
        assert!(
            matches!(
                input.parse::<ActivityRange>().unwrap(),
                ActivityRange::Window(_)
            ),
            "{input}"
        );
    }

    for input in [
        "",
        "-1",
        "3y",
        "5..2",
        "5..",
        "..5",
        "3d..2026-01-10",
        "a..b..c",
        "2025-01-00",
        "2025-13",
        "2025-1",
        "9223372036854775808d",
    ] {
        assert!(input.parse::<ActivityRange>().is_err(), "{input}");
    }
}

#[test]
fn resolves_relative_and_absolute_ranges_with_bucket_endpoints() {
    let now = time("2026-01-15 13:45:00");
    let cases = [
        ("3m", Some("2025-10-01 00:00:00"), "2026-01-15 13:45:00"),
        ("month", Some("2026-01-01 00:00:00"), "2026-01-15 13:45:00"),
        ("3d", Some("2026-01-12 00:00:00"), "2026-01-15 13:45:00"),
        ("3h", Some("2026-01-15 10:00:00"), "2026-01-15 13:45:00"),
        (
            "2025-12",
            Some("2025-12-01 00:00:00"),
            "2026-01-01 00:00:00",
        ),
        (
            "2025..2025",
            Some("2025-01-01 00:00:00"),
            "2026-01-01 00:00:00",
        ),
        (
            "2025-02-03..2025-02-05",
            Some("2025-02-03 00:00:00"),
            "2025-02-06 00:00:00",
        ),
        ("3m..1m", Some("2025-10-01 00:00:00"), "2026-01-01 00:00:00"),
        (
            "2026-01-01..7d",
            Some("2026-01-01 00:00:00"),
            "2026-01-09 00:00:00",
        ),
        ("..3d", None, "2026-01-13 00:00:00"),
        ("3d..", Some("2026-01-12 00:00:00"), "2026-01-15 13:45:00"),
        ("..", None, "2026-01-15 13:45:00"),
    ];

    for (input, start, end) in cases {
        let expected_start = start.map(time);
        assert_eq!(
            window(input).resolve(now).unwrap(),
            (expected_start, time(end)),
            "{input}"
        );
    }
}

#[test]
fn caps_future_endpoints_and_rejects_empty_or_inverted_ranges() {
    let now = time("2026-01-15 13:45:00");
    assert_eq!(window("..2099").resolve(now).unwrap(), (None, now));
    assert!(window("2099-01").resolve(now).is_err());
    assert!(window("2025-03..2025-01").resolve(now).is_err());
    assert!(window("2026-02..3d").resolve(now).is_err());
}
