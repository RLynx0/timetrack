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

    fn command(&self, args: &[&str]) -> Command {
        let mut command = self.root_command();
        command.arg("show").args(args);
        command
    }

    fn root_command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_timetrack"));
        command
            .env("TIMETRACK_HOME", &self.0)
            .env("TIMETRACK_DATA_HOME", &self.0)
            .env("TZ", "UTC")
            .env("NO_COLOR", "1")
            .env_remove("FORCE_COLOR")
            .env_remove("MAX_TABLE_WIDTH");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
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
fn piped_tables_preserve_long_descriptions() {
    let description = "A description longer than a typical terminal width ".repeat(5);
    let mut entries = start(time("2025-01-02T09:00:00Z"), &description, "ON");
    entries += &end(time("2025-01-02T10:00:00Z"));
    let output = Fixture::new(&entries).show(&["entries", "-r", "1"]);
    assert!(output.contains(&description));
    assert!(!output.contains("..."));
}

#[test]
fn max_table_width_limits_piped_tables() {
    let description = "A description longer than a typical terminal width ".repeat(5);
    let mut entries = start(time("2025-01-02T09:00:00Z"), &description, "ON");
    entries += &end(time("2025-01-02T10:00:00Z"));
    let fixture = Fixture::new(&entries);

    for width in [80, 120] {
        let output = fixture
            .command(&["entries", "-r", "1"])
            .env("MAX_TABLE_WIDTH", width.to_string())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let table = String::from_utf8(output.stdout).unwrap();
        assert!(table.contains("..."));
        assert!(!table.contains(&description));
        assert!(
            table
                .lines()
                .all(|line| unicode_width::UnicodeWidthStr::width(line) == width)
        );
    }
}

#[test]
fn invalid_max_table_width_reports_an_error() {
    let mut entries = start(time("2025-01-02T09:00:00Z"), "work", "ON");
    entries += &end(time("2025-01-02T10:00:00Z"));
    let fixture = Fixture::new(&entries);

    for width in ["", "wide", "-1", "9999999999999999999999999999999999999999"] {
        let output = fixture
            .command(&["entries", "-r", "1"])
            .env("MAX_TABLE_WIDTH", width)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{width:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("MAX_TABLE_WIDTH"));
    }
}

#[test]
fn global_directory_flags_override_env_before_and_after_subcommands() {
    let entries = start(time("2025-01-02T09:00:00Z"), "chosen-data", "ON")
        + &end(time("2025-01-02T10:00:00Z"));
    let config = Fixture::new("");
    let data = Fixture::new(&entries);
    for before in [true, false] {
        let mut command = config.root_command();
        command
            .env("TIMETRACK_HOME", config.0.join("missing"))
            .env("TIMETRACK_DATA_HOME", data.0.join("missing"));
        if !before {
            command.args(["show", "-r", "1"]);
        }
        command
            .arg("--config-home")
            .arg(&config.0)
            .arg("--data-home")
            .arg(&data.0);
        if before {
            command.args(["show", "-r", "1"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("chosen-data"));
    }
}

#[test]
fn home_and_xdg_environment_preserve_directory_fallbacks() {
    let entries = start(time("2025-01-02T09:00:00Z"), "fallback-data", "ON")
        + &end(time("2025-01-02T10:00:00Z"));
    let fixture = Fixture::new(&entries);
    for base in [".config", "config-base"] {
        let path = fixture.0.join(base).join("timetrack");
        fs::create_dir_all(&path).unwrap();
        fs::copy(fixture.0.join("config.toml"), path.join("config.toml")).unwrap();
    }
    for base in [".local/share", "data-base"] {
        let path = fixture.0.join(base).join("timetrack");
        fs::create_dir_all(&path).unwrap();
        fs::copy(fixture.0.join("entries"), path.join("entries")).unwrap();
    }

    for xdg in [false, true] {
        let mut command = fixture.root_command();
        command.env_clear().env("TZ", "UTC").env("NO_COLOR", "1");
        if xdg {
            command
                .env("HOME", fixture.0.join("missing"))
                .env("XDG_CONFIG_HOME", fixture.0.join("config-base"))
                .env("XDG_DATA_HOME", fixture.0.join("data-base"));
        } else {
            command.env("HOME", &fixture.0);
        }
        let output = command.args(["show", "-r", "1"]).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("fallback-data"));
    }
}

#[test]
fn missing_home_reports_directory_fallback_error() {
    let fixture = Fixture::new("");
    let output = fixture.command(&[]).env_clear().output().unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("failed to show"));
    assert!(error.contains("environment variable not found"));
}

#[test]
fn max_table_width_flag_overrides_env() {
    let description = "A description longer than a typical terminal width ".repeat(5);
    let entries = start(time("2025-01-02T09:00:00Z"), &description, "ON")
        + &end(time("2025-01-02T10:00:00Z"));
    let fixture = Fixture::new(&entries);
    for before in [true, false] {
        let mut command = fixture.root_command();
        command.env("MAX_TABLE_WIDTH", "120");
        if before {
            command.args(["--max-table-width", "80"]);
        }
        command.args(["show", "-r", "1"]);
        if !before {
            command.args(["--max-table-width", "80"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let table = String::from_utf8(output.stdout).unwrap();
        assert!(table.contains("..."));
        assert!(
            table
                .lines()
                .all(|line| unicode_width::UnicodeWidthStr::width(line) == 80)
        );
    }
}

#[test]
fn color_flags_and_environment_preserve_force_precedence() {
    let fixture = Fixture::new(&start(time("2025-01-02T09:00:00Z"), "work", "ON"));
    for (flags, force, no, colored) in [
        (vec!["--force-color"], None, Some("1"), true),
        (vec!["--no-color"], None, None, false),
        (vec!["--force-color", "--no-color"], None, None, true),
        (vec![], Some("1"), Some("1"), true),
        (vec![], Some(""), Some("1"), true),
        (vec![], Some("0"), Some(""), false),
        (vec![], Some("false"), Some("false"), false),
    ] {
        let mut command = fixture.command(&flags);
        command.env_remove("NO_COLOR");
        if let Some(value) = force {
            command.env("FORCE_COLOR", value);
        }
        if let Some(value) = no {
            command.env("NO_COLOR", value);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout.contains(&0x1b),
            colored,
            "{flags:?} {force:?} {no:?}"
        );
    }
}

#[test]
fn clap_help_lists_environment_flags_without_custom_block() {
    let fixture = Fixture::new("");
    for args in [
        vec!["--help"],
        vec!["show", "--help"],
        vec!["activity", "ls", "--help"],
        vec!["edit", "--help"],
    ] {
        let output = fixture.root_command().args(args).output().unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("--max-table-width"));
        assert!(help.contains("[env: MAX_TABLE_WIDTH="));
        assert!(help.contains("--editor"));
        assert!(help.contains("[env: EDITOR="));
        assert!(help.contains("--config-home"));
        assert!(help.contains("--data-home"));
        assert!(!help.contains("--xdg-"));
        assert!(!help.contains("--home"));
        assert!(!help.contains("Environment variables:"));
    }
}

#[cfg(unix)]
#[test]
fn global_editor_flag_overrides_env_and_keeps_edit_short_flag() {
    let fixture = Fixture::new("");
    for before in [true, false] {
        let mut command = fixture.root_command();
        command.env("EDITOR", "missing-timetrack-editor");
        if before {
            command.args(["--editor", "/bin/echo", "edit", "config"]);
        } else {
            command.args(["edit", "config", "-e", "/bin/echo"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            fixture.0.join("config.toml").to_str().unwrap()
        );
    }
}

#[test]
fn nested_activity_commands_use_global_paths_and_table_width() {
    let fixture = Fixture::new("");
    let config_home = fixture.0.join("selected-config");
    let description = "A description longer than the configured table width";
    let output = fixture
        .root_command()
        .args([
            "activity",
            "set",
            "team/task",
            "1234",
            "--description",
            description,
        ])
        .arg("--config-home")
        .arg(&config_home)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(config_home.join("activities").exists());
    assert!(!fixture.0.join("activities").exists());

    let output = fixture
        .root_command()
        .args(["activity", "ls", "--recursive", "--max-table-width", "40"])
        .arg("--config-home")
        .arg(&config_home)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let table = String::from_utf8(output.stdout).unwrap();
    assert!(table.contains("team/task"));
    assert!(table.contains("..."));
    assert!(
        table
            .lines()
            .all(|line| unicode_width::UnicodeWidthStr::width(line) == 40)
    );
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
        fixture.show(&["collapsed", "--range", "1", "-m"]),
        "2025-01-02\t0.80\tON\ta\ta\n"
    );
    let rows = fixture.show(&["collapsed", "--range", "3", "-m"]);
    assert_eq!(rows.lines().count(), 3);
    assert!(rows.contains("2025-01-01\t1.00\tON\told\told"));
    assert!(rows.contains("2025-01-02\t0.80\tON\ta\ta"));
    assert!(rows.contains("2025-01-02\t0.80\tON\tb\tb"));
    assert_eq!(
        fixture
            .show(&["collapsed", "-r", "1", "-m"])
            .lines()
            .count(),
        1
    );
    assert_eq!(
        fixture.show(&["collapsed", "-r", "0"]).trim(),
        "You are not tracking any activity"
    );
    let latest_attendance = fixture.show(&["attendance", "-r", "1", "-m"]);
    assert!(latest_attendance.starts_with("2025-01-02 09:00:00"));
    assert!(latest_attendance.contains("2025-01-02 10:36:00"));
    assert_eq!(
        fixture
            .show(&["attendance", "-r", "2", "-m"])
            .lines()
            .count(),
        2
    );
    assert_eq!(
        fixture.show(&["entries", "-r", "3", "-m"]).lines().count(),
        3
    );
    assert_eq!(fixture.show(&["time", "-r", "1", "-m"]), "60.00\n");
    assert_eq!(fixture.show(&["time", "-r", "3", "-m"]), "180.00\n");
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
        let panel = fixture.show(&[mode, "-r", "1"]);
        assert!(!panel.contains("Tracking activity"));
        assert!(!panel.is_empty());
    }
    let multi = fixture.show(&["collapsed", "attendance", "time"]);
    for header in [":Collapsed", ":Attendance", ":Time"] {
        assert!(multi.contains(header));
    }
}

#[test]
fn count_ranges_include_the_active_result_in_addition_to_completed_results() {
    let now = Utc::now();
    let mut entries = String::new();
    for (days_ago, key) in [(4, "a"), (3, "b"), (2, "c")] {
        let started = now - TimeDelta::days(days_ago);
        entries += &start(started, key, "ON");
        entries += &end(started + TimeDelta::hours(1));
    }
    entries += &start(now - TimeDelta::hours(1), "active", "ON");
    let fixture = Fixture::new(&entries);

    assert_eq!(
        fixture.show(&["entries", "-r", "2", "-m"]).lines().count(),
        3
    );
    assert_eq!(
        fixture
            .show(&["collapsed", "-r", "3", "-m"])
            .lines()
            .count(),
        4
    );
    assert_eq!(
        fixture
            .show(&["collapsed", "-r", "0", "-m"])
            .lines()
            .count(),
        1
    );
    assert!(
        fixture
            .show(&["entries", "-r", "0"])
            .contains("Tracking activity 'active'")
    );
    assert_eq!(
        fixture
            .show(&["attendance", "-r", "0", "-m"])
            .lines()
            .count(),
        1
    );
    let active_seconds = fixture
        .show(&["time", "-r", "0", "-m"])
        .trim()
        .parse::<f64>()
        .unwrap();
    assert!(
        (3_590.0..3_610.0).contains(&active_seconds),
        "{active_seconds}"
    );
    assert_eq!(
        fixture
            .show(&["attendance", "-r", "2", "-m"])
            .lines()
            .count(),
        3
    );
    let seconds = fixture
        .show(&["time", "-r", "2", "-m"])
        .trim()
        .parse::<f64>()
        .unwrap();
    assert!((10_790.0..10_810.0).contains(&seconds), "{seconds}");
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

    let rows = fixture.show(&["--range", "2025-01..2025-02-20", "-m"]);
    assert_eq!(rows.lines().count(), 2);
    assert!(rows.starts_with("2025-01-01 00:00:00"));
    assert!(rows.contains("2025-02-20 23:59:59.999999999"));
    assert!(!rows.contains("2025-02-21"));
    assert!(!rows.contains("2025-03-01"));
    assert_eq!(
        fixture.show(&["time", "--range", "2025-01", "-m"]),
        "3600.00\n"
    );
    assert_eq!(fixture.show(&["time", "--range", "..", "-m"]), "18000.00\n");
    assert_eq!(
        fixture.show(&["time", "--range", "..2025-01", "-m"]),
        "7200.00\n"
    );
    assert_eq!(
        fixture.show(&["time", "--range", "2025-03..", "-m"]),
        "3600.00\n"
    );
    assert!(
        fixture
            .show(&["--range", "..2000"])
            .contains("in the requested timeframe")
    );
    assert!(
        fixture
            .show(&["collapsed", "--range", "2025-02"])
            .contains("1.00")
    );
    assert!(
        fixture
            .show(&["attendance", "--range", "2025-02-20"])
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

    assert_eq!(fixture.show(&["time", "--range", "1m", "-m"]), "18000.00\n");
    assert_eq!(
        fixture.show(&["time", "--range", "1m..1m", "-m"]),
        "10800.00\n"
    );
    assert_eq!(fixture.show(&["time", "-r", "1", "-m"]), "7200.00\n");
    // A future range end must never extend an ongoing activity beyond now.
    fixture.write(&start(now - TimeDelta::minutes(10), "ongoing", "ON"));
    let seconds = fixture
        .show(&["time", "--range", "..2099", "-m"])
        .trim()
        .parse::<f64>()
        .unwrap();
    assert!((600.0..610.0).contains(&seconds), "{seconds}");
    let ongoing_row = fixture.show(&["entries", "--range", "..2099", "-m"]);
    assert!(ongoing_row.ends_with("\tongoing\tON\tongoing\tongoing\n"));
    let raw = fixture.show(&["-m"]);
    assert_eq!(raw.split('\t').count(), 5);
    assert!(fixture.show(&[]).contains("Tracking activity 'ongoing'"));
}

#[test]
fn current_time_includes_the_whole_overnight_activity() {
    let now = Utc::now();
    let fixture = Fixture::new(&start(now - TimeDelta::hours(49), "overnight", "ON"));
    let seconds = fixture
        .show(&["time", "-r", "0", "-m"])
        .trim()
        .parse::<f64>()
        .unwrap();
    assert!((176_400.0..176_410.0).contains(&seconds), "{seconds}");
    assert_eq!(
        fixture.show(&["entries", "-r", "1", "-m"]).lines().count(),
        2
    );
}

#[test]
fn cli_rejects_old_flags_and_invalid_ranges_and_handles_empty_data() {
    let fixture = Fixture::new("");
    for args in [
        vec!["--last", "1"],
        vec!["--from", "2025"],
        vec!["--to", "2025"],
        vec!["--in", "2025"],
        vec!["--range", "2025-01-00"],
        vec!["--range", "-1"],
        vec!["--range", "2025-03..2025-01"],
        vec!["--range", "2099-01"],
        vec!["--range", "3d..2025"],
        vec!["--range", "2..1"],
    ] {
        assert!(!fixture.run(&args).status.success(), "{args:?}");
    }
    for mode in ["entries", "collapsed", "attendance", "time"] {
        assert_eq!(
            fixture.show(&[mode]).trim(),
            "You have not recorded any data yet"
        );
        assert_eq!(
            fixture.show(&[mode, "--range", "3"]).trim(),
            "You have not recorded any data yet"
        );
    }

    let now = Utc::now();
    let mut entries = start(now - TimeDelta::hours(1), "done", "ON");
    entries += &end(now);
    let idle = Fixture::new(&entries);
    for mode in ["entries", "collapsed", "attendance", "time"] {
        assert_eq!(
            idle.show(&[mode, "-r", "0"]).trim(),
            "You are not tracking any activity"
        );
    }

    // Keep the test process's timezone unchanged; only the child uses UTC.
    let _ = Local::now();
}
