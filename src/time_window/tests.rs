use super::*;

fn time(input: &str) -> DateTime<Local> {
    local_time(NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S").unwrap()).unwrap()
}

#[test]
fn parses_and_resolves_calendar_buckets() {
    let now = time("2026-01-15 13:45:00");
    for (input, start, end) in [
        ("1m", "2025-12-01 00:00:00", "2026-01-01 00:00:00"),
        ("month", "2026-01-01 00:00:00", "2026-02-01 00:00:00"),
        ("2025", "2025-01-01 00:00:00", "2026-01-01 00:00:00"),
        ("2025-12", "2025-12-01 00:00:00", "2026-01-01 00:00:00"),
        ("2024-02-29", "2024-02-29 00:00:00", "2024-03-01 00:00:00"),
        ("week", "2026-01-12 00:00:00", "2026-01-19 00:00:00"),
        ("1d", "2026-01-14 00:00:00", "2026-01-15 00:00:00"),
        ("1h", "2026-01-15 12:00:00", "2026-01-15 13:00:00"),
    ] {
        assert_eq!(
            input.parse::<TimePoint>().unwrap().bounds(now).unwrap(),
            (time(start), time(end)),
            "{input}"
        );
    }
    for invalid in [
        "",
        "13",
        "2025-01-00",
        "2025-13",
        "2025-02-29",
        "2025-1",
        "2025-01-01T10:00",
        "9223372036854775808m",
    ] {
        assert!(invalid.parse::<TimePoint>().is_err(), "{invalid}");
    }
    assert!(
        "9223372036854775807m"
            .parse::<TimePoint>()
            .unwrap()
            .bounds(now)
            .is_err()
    );
}

#[test]
fn resolves_open_ends_and_caps_at_now() {
    let now = time("2026-01-15 13:45:00");
    let window = |from: Option<&str>, to: Option<&str>| TimeWindow {
        from: from.map(|p| p.parse().unwrap()),
        to: to.map(|p| p.parse().unwrap()),
    };
    assert_eq!(
        window(Some("1m"), Some("1m")).resolve(now).unwrap(),
        (
            Some(time("2025-12-01 00:00:00")),
            time("2026-01-01 00:00:00")
        )
    );
    assert_eq!(
        window(None, Some("1m")).resolve(now).unwrap(),
        (None, time("2026-01-01 00:00:00"))
    );
    assert_eq!(
        window(Some("2025"), None).resolve(now).unwrap(),
        (Some(time("2025-01-01 00:00:00")), now)
    );
    assert_eq!(
        window(None, Some("2099")).resolve(now).unwrap(),
        (None, now)
    );
    assert_eq!(
        window(Some("month"), Some("month")).resolve(now).unwrap(),
        (Some(time("2026-01-01 00:00:00")), now)
    );
    assert!(window(Some("2099"), None).resolve(now).is_err());
    assert!(
        window(Some("2025-03"), Some("2025-01"))
            .resolve(now)
            .is_err()
    );
}
