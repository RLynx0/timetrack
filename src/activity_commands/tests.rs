use super::*;

fn parse(lines: &[&str]) -> Vec<Activity> {
    lines
        .iter()
        .map(|line| Activity::from_str(line).unwrap())
        .collect()
}

#[test]
fn collapsed_category_path_joins_single_branch_chain() {
    let activities = parse(&["team/project/feature\t1", "team/project/maintenance\t2"]);
    let hierarchy = ActivityCategory::from(activities);

    assert_eq!(
        collapsed_category_item("team", &hierarchy.branches["team"])
            .display_name()
            .as_ref(),
        "team/project/"
    );
}

#[test]
fn collapsed_category_path_stops_at_fork() {
    let activities = parse(&["team/project/feature\t1", "team/support\t2"]);
    let hierarchy = ActivityCategory::from(activities);

    assert_eq!(
        collapsed_category_item("team", &hierarchy.branches["team"])
            .display_name()
            .as_ref(),
        "team/"
    );
}

#[test]
fn collapsed_category_includes_single_leaf_and_metadata() {
    for path in ["team/feature", "team/project/feature"] {
        let activity = Activity::from_str(&format!("{path}\t1\tFeature work")).unwrap();
        let hierarchy = ActivityCategory::from([activity]);
        let item = collapsed_category_item("team", &hierarchy.branches["team"]);

        assert_eq!(item.display_name().as_ref(), path);
        assert_eq!(item.wbs(), Some("1"));
        assert_eq!(item.description(), Some("Feature work"));
        assert_eq!(item.to_string(), format!("{path}\t1\tFeature work"));
    }
}

#[test]
fn collapsed_category_stops_at_multiple_branches() {
    let activities = parse(&["team/project/feature\t1", "team/support/help\t2"]);
    let hierarchy = ActivityCategory::from(activities);
    let item = collapsed_category_item("team", &hierarchy.branches["team"]);

    assert_eq!(item.display_name().as_ref(), "team/");
    assert_eq!(item.to_string(), "team/\t\t");
}

#[test]
fn set_appends_and_force_updates_in_place() {
    let original = parse(&["team/first\t1\tFirst", "other\t2"]);
    let added = Activity::new(&["team", "last"], "3", None).unwrap();
    let (activities, outcome) = apply_set(original, added, false).unwrap();
    assert_eq!(outcome, SetOutcome::Added);
    assert_eq!(
        activities
            .iter()
            .map(Activity::full_path)
            .collect::<Vec<_>>(),
        ["team/first", "other", "team/last"]
    );

    let replacement = Activity::new(&["team", "first"], "4", Some("Changed")).unwrap();
    let (activities, outcome) = apply_set(activities, replacement, true).unwrap();
    assert_eq!(outcome, SetOutcome::Updated);
    assert_eq!(
        activities
            .iter()
            .map(Activity::full_path)
            .collect::<Vec<_>>(),
        ["team/first", "other", "team/last"]
    );
    assert_eq!(activities[0].to_string(), "team/first\t4\tChanged");
}

#[test]
fn set_rejects_conflicts_and_protects_idle() {
    let original = parse(&["team/task\t1", "other\t2"]);
    for (path, force) in [
        ("team/task", false),
        ("team", false),
        ("team", true),
        ("other/task", true),
        ("idle", true),
        ("idle/task", true),
    ] {
        let activity = Activity::new(&split_path(path), "3", None).unwrap();
        assert!(
            apply_set(original.clone(), activity, force).is_err(),
            "{path}"
        );
    }
}

#[test]
fn remove_leaf_or_subtree_preserves_other_activities() {
    let original = parse(&["team/task\t1", "team/deep/task\t2", "teammate/task\t3"]);
    let (activities, count) = apply_remove(original.clone(), &["team", "task"], false).unwrap();
    assert_eq!(count, 1);
    assert_eq!(activities[0].full_path(), "team/deep/task");

    let (activities, count) = apply_remove(original, &["team"], true).unwrap();
    assert_eq!(count, 2);
    assert_eq!(activities[0].full_path(), "teammate/task");
}

#[test]
fn remove_rejects_categories_without_recursive_missing_paths_and_idle() {
    let original = parse(&["team/task\t1"]);
    for (path, recursive) in [
        ("team", false),
        ("missing", true),
        ("team/missing", false),
        ("idle", true),
        ("", true),
    ] {
        assert!(
            apply_remove(original.clone(), &split_path(path), recursive).is_err(),
            "{path}"
        );
    }
}

#[test]
fn move_renames_leaf_or_category_preserving_order_and_metadata() {
    let original = parse(&["team/task\t1\tTask", "other\t2", "team/deep/task\t3"]);
    let (activities, destination) =
        apply_move(original.clone(), &["team", "task"], &["new", "name"]).unwrap();
    assert_eq!(destination, "new/name");
    assert_eq!(activities[0].to_string(), "new/name\t1\tTask");
    assert_eq!(activities[1].full_path(), "other");

    let (activities, destination) = apply_move(original, &["team"], &["new"]).unwrap();
    assert_eq!(destination, "new");
    assert_eq!(
        activities
            .iter()
            .map(Activity::full_path)
            .collect::<Vec<_>>(),
        ["new/task", "other", "new/deep/task"]
    );
    assert_eq!(activities[0].description(), Some("Task"));
}

#[test]
fn move_into_category_or_root_uses_source_basename() {
    let original = parse(&["team/task\t1", "destination/other\t2"]);
    let (activities, destination) =
        apply_move(original.clone(), &["team", "task"], &["destination"]).unwrap();
    assert_eq!(destination, "destination/task");
    assert_eq!(activities[0].full_path(), destination);

    let (activities, destination) =
        apply_move(original.clone(), &["team"], &["destination"]).unwrap();
    assert_eq!(destination, "destination/team");
    assert_eq!(activities[0].full_path(), "destination/team/task");

    let (activities, destination) = apply_move(original, &["team", "task"], &[]).unwrap();
    assert_eq!(destination, "task");
    assert_eq!(activities[0].full_path(), "task");
}

#[test]
fn move_rejects_collisions_self_moves_missing_sources_and_idle() {
    let original = parse(&[
        "team/task\t1",
        "destination/task\t2",
        "destination/team/existing\t3",
        "other\t4",
    ]);
    for (from, to) in [
        ("team/task", "other"),
        ("team/task", "destination"),
        ("team", "destination"),
        ("team", "team"),
        ("team", "team/new"),
        ("team/task", "team"),
        ("team/task", "other/nested"),
        ("missing", "new"),
        ("", "new"),
        ("idle", "new"),
        ("team/task", "idle"),
        ("team/task", "idle/nested"),
        ("team/task", "invalid\tname"),
    ] {
        assert!(
            apply_move(original.clone(), &split_path(from), &split_path(to)).is_err(),
            "{from} -> {to}"
        );
    }
}

#[test]
fn activity_constructor_validates_fields_and_sanitizes_description() {
    for path in [
        vec![],
        vec![""],
        vec!["team", ""],
        vec!["bad/name"],
        vec!["bad\tname"],
        vec!["bad\nname"],
        vec!["bad\rname"],
    ] {
        assert!(Activity::new(&path, "1", None).is_err(), "{path:?}");
    }
    for wbs in ["", "bad\twbs", "bad\nwbs", "bad\rwbs"] {
        assert!(Activity::new(&["task"], wbs, None).is_err(), "{wbs:?}");
    }
    let activity = Activity::new(&["team", "task"], "1", Some("first\tpart\nsecond")).unwrap();
    assert_eq!(
        activity.to_string(),
        "team/task\t1\tfirst    part -- second"
    );
    assert_eq!(
        Activity::from_str(&activity.to_string())
            .unwrap()
            .description(),
        activity.description()
    );
}

#[test]
fn relocation_matches_complete_path_segments() {
    let activity = Activity::from_str("team/deep/task\t1\tTask").unwrap();
    assert!(activity.relocated(&["tea"], &["new"]).unwrap().is_none());
    let relocated = activity.relocated(&["team"], &["new"]).unwrap().unwrap();
    assert_eq!(relocated.to_string(), "new/deep/task\t1\tTask");
}

#[test]
fn lookup_reports_the_offending_parent_without_newline() {
    let hierarchy = ActivityCategory::from(parse(&["team/task\t1"]));
    let error = hierarchy
        .get_item_at(&["team", "task", "child"])
        .unwrap_err();
    assert_eq!(error.to_string(), "team/task is not an activity category");
}

#[test]
fn atomic_writer_creates_parent_and_replaces_contents() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("timetrack-test-{}-{timestamp}", std::process::id()));
    let path = directory.join("config/activities");
    write_activities_to(&path, &parse(&["team/task\t1\tTask"])).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "team/task\t1\tTask\n");
    write_activities_to(&path, &[]).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "");
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_atomic_write_cleans_up_temporary_file() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "timetrack-failure-test-{}-{timestamp}",
        std::process::id()
    ));
    let path = directory.join("activities");
    fs::create_dir_all(&path).unwrap();
    assert!(write_activities_to(&path, &parse(&["task\t1"])).is_err());
    assert!(path.is_dir());
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_dir_all(directory).unwrap();
}
