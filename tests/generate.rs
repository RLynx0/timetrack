use std::{fs, process::Output};

use chrono::{DateTime, TimeDelta, Utc};

mod common;

struct Fixture(common::Fixture);

impl Fixture {
    fn new(entries: &str, range: Option<&str>) -> Self {
        let fixture = common::Fixture::new("timetrack-generate");
        let mut config: toml::Value =
            toml::from_str(include_str!("../assets/default_config.toml")).unwrap();
        let generate = config["generate"].as_table_mut().unwrap();
        match range {
            Some(range) => {
                generate.insert("range".into(), range.into());
            }
            None => {
                generate.remove("range");
            }
        }
        generate.insert(
            "file_name_format".into(),
            "${year}-${month}-${day}.txt".into(),
        );
        fs::write(
            fixture.path.join("config.toml"),
            toml::to_string(&config).unwrap(),
        )
        .unwrap();
        fs::write(fixture.path.join("entries"), entries).unwrap();
        Self(fixture)
    }

    fn run(&self, args: &[&str]) -> Output {
        self.0.command().args(args).output().unwrap()
    }

    fn success(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "args: {args:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn rows(&self, args: &[&str]) -> Vec<String> {
        let mut command = vec!["generate", "--stdout"];
        command.extend_from_slice(args);
        self.success(&command)
            .lines()
            .skip(1)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

fn time(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input).unwrap().to_utc()
}

fn start(time: DateTime<Utc>, key: &str) -> String {
    format!("{time}\t{key}\t0800\t{key}\t{key}\n")
}

fn completed(from: &str, to: &str, key: &str) -> String {
    format!("{}{end}\t__END\n\n", start(time(from), key), end = time(to))
}

#[test]
fn configured_window_clips_both_boundaries_and_cli_overrides_it() {
    let entries = completed("2024-12-31T23:00:00Z", "2025-01-01T01:00:00Z", "first")
        + &completed("2025-01-31T23:00:00Z", "2025-02-01T01:00:00Z", "last");
    let fixture = Fixture::new(&entries, Some("2025-01"));
    assert_eq!(
        fixture.rows(&[]),
        [
            "01/01/2025\tfirst\t0800\t1.00\tfirst",
            "31/01/2025\tlast\t0800\t1.00\tlast",
        ]
    );
    assert_eq!(
        fixture.rows(&["--range", "2025-02-01..2025-02-02"]),
        ["01/02/2025\tlast\t0800\t1.00\tlast"]
    );
}

#[test]
fn counts_select_whole_collapsed_groups_and_include_active_result() {
    let entries = completed("2025-01-02T09:00:00Z", "2025-01-02T10:00:00Z", "a")
        + &completed("2025-01-02T10:00:00Z", "2025-01-02T11:00:00Z", "b")
        + &completed("2025-01-02T11:00:00Z", "2025-01-02T12:00:00Z", "a");
    let fixture = Fixture::new(&entries, Some("1"));
    assert_eq!(fixture.rows(&[]), ["02/01/2025\ta\t0800\t2.00\ta"]);
    assert!(fixture.rows(&["-r", "0"]).is_empty());

    let ongoing = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    fs::write(
        fixture.0.path.join("entries"),
        entries + &start(ongoing, "active"),
    )
    .unwrap();
    let rows = fixture.rows(&["-r", "1"]);
    assert_eq!(rows[0], "02/01/2025\ta\t0800\t2.00\ta");
    assert!(rows[1..].iter().all(|row| row.ends_with("\tactive")));
    assert!(rows.len() > 1);
    let rows = fixture.rows(&["-r", "0"]);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| row.ends_with("\tactive")));
}

#[test]
fn invalid_config_range_is_lazy_and_can_be_overridden() {
    let fixture = Fixture::new("", Some("not-a-range"));
    let output = fixture.run(&["generate", "--stdout"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("generate.range"));
    fixture.success(&["show", "collapsed", "-r", "2025-01"]);
    fixture.success(&["list-attendance-types"]);
    assert!(fixture.rows(&["-r", "2025-01"]).is_empty());
}

#[test]
fn missing_config_range_defaults_to_current_month() {
    let now = Utc::now();
    let month_start = now.date_naive().format("%Y-%m-01T00:00:00Z").to_string();
    let before = time(&month_start) - TimeDelta::days(1);
    let entries = format!(
        "{}{}\t__END\n\n",
        start(before, "previous"),
        before + TimeDelta::hours(1)
    ) + &start(now - TimeDelta::minutes(1), "active");
    let fixture = Fixture::new(&entries, None);
    assert_eq!(fixture.rows(&[]), fixture.rows(&["-r", "month"]));
    assert!(!fixture.rows(&[]).iter().any(|row| row.contains("previous")));
}

#[test]
fn file_name_uses_range_start_then_first_row_then_today() {
    let entries = completed("2025-01-15T09:00:00Z", "2025-01-15T10:00:00Z", "a");
    let fixture = Fixture::new(&entries, Some("2025-01"));
    let directory = fixture.0.path.to_str().unwrap();
    fixture.success(&["generate", "-f", directory]);
    let contents = fs::read_to_string(fixture.0.path.join("2025-01-01.txt")).unwrap();
    assert!(contents.contains("15/01/2025\ta\t0800\t1.00\ta"));
    fixture.success(&["generate", "-f", directory, "-r", ".."]);
    assert!(fixture.0.path.join("2025-01-15.txt").exists());
    fixture.success(&["generate", "-f", directory, "-r", "2024-12"]);
    assert!(fixture.0.path.join("2024-12-01.txt").exists());

    fs::write(fixture.0.path.join("entries"), "").unwrap();
    let today_before = Utc::now().format("%Y-%m-%d.txt").to_string();
    fixture.success(&["generate", "-f", directory, "-r", ".."]);
    let today_after = Utc::now().format("%Y-%m-%d.txt").to_string();
    assert!(
        fixture.0.path.join(today_before).exists() || fixture.0.path.join(today_after).exists()
    );
}

#[test]
fn generated_output_uses_crlf_for_stdout_and_files() {
    let entries = completed("2025-01-15T09:00:00Z", "2025-01-15T10:00:00Z", "a");
    let fixture = Fixture::new(&entries, Some("2025-01"));
    let output = fixture.success(&["generate", "--stdout", "--range", "2025-01"]);
    assert!(output.as_bytes().ends_with(b"\r\n"));
    assert!(
        output
            .as_bytes()
            .split_inclusive(|byte| *byte == b'\n')
            .all(|line| line.ends_with(b"\r\n"))
    );

    let output_path = fixture.0.path.join("generated.txt");
    fixture.success(&[
        "generate",
        "--file-path",
        output_path.to_str().unwrap(),
        "--range",
        "2025-01",
    ]);
    let file = fs::read(output_path).unwrap();
    assert!(file.ends_with(b"\r\n"));
    assert!(
        file.split_inclusive(|byte| *byte == b'\n')
            .all(|line| line.ends_with(b"\r\n"))
    );

    let empty = Fixture::new("", Some("2025-01"));
    let output = empty.success(&["generate", "--stdout", "--range", "2025-01"]);
    assert!(output.ends_with("\r\n"));
    assert!(!output.contains('\n') || output.matches('\n').count() == 1);
}
