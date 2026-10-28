use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeDelta, Utc};

struct Fixture(PathBuf);

impl Fixture {
    fn new(entries: &str) -> Self {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("timetrack-show-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        fs::write(
            path.join("config.toml"),
            include_str!("../assets/default_config.toml"),
        )
        .unwrap();
        let fixture = Self(path);
        fixture.write(entries);
        fixture
    }

    fn write(&self, entries: &str) {
        fs::write(self.0.join("entries"), entries).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_timetrack"))
            .arg("show")
            .args(args)
            .env("TIMETRACK_HOME", &self.0)
            .env("TIMETRACK_DATA_HOME", &self.0)
            .env("TZ", "UTC")
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }

    fn show(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "args: {args:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn start(time: DateTime<Utc>, key: &str, attendance: &str) -> String {
    format!("{time}\t{key}\t{attendance}\t{key}\t{key}\n")
}

fn end(time: DateTime<Utc>) -> String {
    format!("{time}\t__END\n\n")
}

fn time(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input).unwrap().to_utc()
}

#[test]
fn latest_and_counts_select_complete_groups_and_ranges() {
    let first = time("2025-01-02T09:00:00Z");
    let mut entries = start(first - TimeDelta::days(1), "old", "ON");
    entries += &end(first - TimeDelta::days(1) + TimeDelta::hours(1));
    // More than two read batches, with A returning after B.
    for minute in 0..96 {
        entries += &start(
            first + TimeDelta::minutes(minute),
            if minute % 2 == 0 { "b" } else { "a" },
            "ON",
        );
    }
    entries += &end(first + TimeDelta::minutes(96));
    let fixture = Fixture::new(&entries);

    assert_eq!(
        fixture.show(&["collapsed", "-m"]),
        "2025-01-02\t0.80\tON\ta\ta\n"
    );
    let rows = fixture.show(&["collapsed", "--last", "3", "-m"]);
    assert_eq!(rows.lines().count(), 3);
    assert!(rows.contains("2025-01-01\t1.00\tON\told\told"));
    assert!(rows.contains("2025-01-02\t0.80\tON\ta\ta"));
    assert!(rows.contains("2025-01-02\t0.80\tON\tb\tb"));
    assert_eq!(
        fixture
            .show(&["collapsed", "-l", "1", "-m"])
            .lines()
            .count(),
        1
    );
    assert_eq!(
        fixture.show(&["collapsed", "-l", "0", "-m"]),
        fixture.show(&["collapsed", "-m"])
    );
    let latest_attendance = fixture.show(&["attendance", "-m"]);
    assert!(latest_attendance.starts_with("2025-01-02 09:00:00"));
    assert!(latest_attendance.contains("2025-01-02 10:36:00"));
    assert_eq!(
        fixture
            .show(&["attendance", "-l", "2", "-m"])
            .lines()
            .count(),
        2
    );
    assert_eq!(
        fixture.show(&["entries", "-l", "3", "-m"]).lines().count(),
        3
    );
    assert_eq!(fixture.show(&["time", "-m"]), "60.00\n");
    assert_eq!(fixture.show(&["time", "-l", "3", "-m"]), "180.00\n");
    assert_eq!(
        fixture.show(&["-m"]),
        end(first + TimeDelta::minutes(96))
            .replace(" UTC", " +00:00")
            .trim_end()
            .to_owned()
            + "\n"
    );
    assert_eq!(
        fixture.show(&[]).trim(),
        "You are not tracking any activity"
    );
    for mode in ["collapsed", "attendance", "time"] {
        let panel = fixture.show(&[mode]);
        assert!(panel.contains("-> "));
        assert!(!panel.contains("Tracking activity"));
        assert!(!panel.contains('│'));
    }
    let multi = fixture.show(&["collapsed", "attendance", "time"]);
    for header in [":Collapsed", ":Attendance", ":Time"] {
        assert!(multi.contains(header));
    }
}

#[test]
fn windows_include_entire_end_bucket_and_clip_crossing_activities() {
    let mut entries = start(time("2024-12-31T23:00:00Z"), "a", "ON");
    entries += &end(time("2025-01-01T01:00:00Z"));
    entries += &start(time("2025-02-20T23:00:00Z"), "b", "ON");
    entries += &end(time("2025-02-21T01:00:00Z"));
    entries += &start(time("2025-03-01T09:00:00Z"), "c", "ON");
    entries += &end(time("2025-03-01T10:00:00Z"));
    let fixture = Fixture::new(&entries);

    let rows = fixture.show(&["--from", "2025-01", "--to", "2025-02-20", "-m"]);
    assert_eq!(rows.lines().count(), 2);
    assert!(rows.starts_with("2025-01-01 00:00:00"));
    assert!(rows.contains("2025-02-20 23:59:59.999999999"));
    assert!(!rows.contains("2025-02-21"));
    assert!(!rows.contains("2025-03-01"));
    assert_eq!(
        fixture.show(&["time", "--in", "2025-01", "-m"]),
        "3600.00\n"
    );
    assert_eq!(
        fixture.show(&["time", "--to", "2025-01", "-m"]),
        "7200.00\n"
    );
    assert_eq!(
        fixture.show(&["time", "--from", "2025-03", "-m"]),
        "3600.00\n"
    );
    assert!(
        fixture
            .show(&["--to", "2000"])
            .contains("in the requested timeframe")
    );
    assert!(
        fixture
            .show(&["collapsed", "--in", "2025-02"])
            .contains("1.00")
    );
    assert!(
        fixture
            .show(&["attendance", "--in", "2025-02-20"])
            .contains("1.00")
    );
}

#[test]
fn relative_month_windows_and_unsplit_latest_time() {
    let now = Utc::now();
    let month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let previous = month.pred_opt().unwrap();
    let previous = NaiveDate::from_ymd_opt(previous.year(), previous.month(), 1).unwrap();
    let start_previous = previous.and_hms_opt(9, 0, 0).unwrap().and_utc();
    let start_current = month.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let mut entries = start(start_previous, "old", "ON");
    entries += &end(start_previous + TimeDelta::hours(1));
    entries += &start(start_current - TimeDelta::hours(2), "overnight", "ON");
    entries += &end(start_current + TimeDelta::hours(2));
    let fixture = Fixture::new(&entries);

    assert_eq!(fixture.show(&["time", "--in", "1m", "-m"]), "10800.00\n");
    assert_eq!(
        fixture.show(&["time", "--from", "1m", "--to", "1m", "-m"]),
        "10800.00\n"
    );
    assert_eq!(fixture.show(&["time", "-m"]), "14400.00\n");
    // The historical latest entry is ongoing; a future --to must never extend it beyond now.
    fixture.write(&start(now - TimeDelta::minutes(10), "ongoing", "ON"));
    let seconds = fixture
        .show(&["time", "--to", "2099", "-m"])
        .trim()
        .parse::<f64>()
        .unwrap();
    assert!((600.0..610.0).contains(&seconds), "{seconds}");
    let ongoing_row = fixture.show(&["entries", "--to", "2099", "-m"]);
    assert!(ongoing_row.ends_with("\tongoing\tON\tongoing\tongoing\n"));
    let raw = fixture.show(&["-m"]);
    assert_eq!(raw.split('\t').count(), 5);
    assert!(fixture.show(&[]).contains("Tracking activity 'ongoing'"));
}

#[test]
fn cli_rejects_conflicts_and_invalid_windows_and_handles_empty_data() {
    let fixture = Fixture::new("");
    for args in [
        vec!["--last", "1", "--from", "2025"],
        vec!["--last", "1", "--to", "2025"],
        vec!["--last", "1", "--in", "2025"],
        vec!["--in", "2025", "--from", "2025"],
        vec!["--in", "2025", "--to", "2025"],
        vec!["--from", "2025-01-00"],
        vec!["--to", "13"],
        vec!["--from", "2025-03", "--to", "2025-01"],
        vec!["--from", "2099"],
    ] {
        assert!(!fixture.run(&args).status.success(), "{args:?}");
    }
    for mode in ["entries", "collapsed", "attendance", "time"] {
        assert_eq!(
            fixture.show(&[mode]).trim(),
            "You have not recorded any data yet"
        );
        assert_eq!(
            fixture.show(&[mode, "--last", "3"]).trim(),
            "You have not recorded any data yet"
        );
    }
    // Keep the test process's timezone unchanged; only the child uses UTC.
    let _ = Local::now();
}
