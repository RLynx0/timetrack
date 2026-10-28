use super::*;

#[test]
fn collapsed_category_path_joins_single_branch_chain() {
    let activities = ["team/project/feature\t1", "team/project/maintenance\t2"]
        .into_iter()
        .map(Activity::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
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
    let activities = ["team/project/feature\t1", "team/support\t2"]
        .into_iter()
        .map(Activity::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
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
    let activities = ["team/project/feature\t1", "team/support/help\t2"]
        .into_iter()
        .map(Activity::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    let hierarchy = ActivityCategory::from(activities);
    let item = collapsed_category_item("team", &hierarchy.branches["team"]);

    assert_eq!(item.display_name().as_ref(), "team/");
    assert_eq!(item.to_string(), "team/\t\t");
}
