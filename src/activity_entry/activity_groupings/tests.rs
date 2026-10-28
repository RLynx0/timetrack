use super::*;
use crate::activity_entry::ActivityEntry;

fn activity(day: u32, hour: u32, key: &str, attendance: &str) -> TrackedActivity {
    let entry =
        format!("2025-01-{day:02} {hour:02}:00:00 +00:00\t{key}\t{attendance}\t{key}\t{key}")
            .parse::<ActivityEntry>()
            .unwrap();
    let ActivityEntry::Start(start) = entry else {
        panic!()
    };
    let end = *start.time_stamp() + TimeDelta::hours(1);
    TrackedActivity::new_completed(start, end)
}

#[test]
fn collapsed_latest_includes_the_whole_group_and_waits_for_complete_dates() {
    let activities = vec![
        activity(1, 8, "old", "A"),
        activity(2, 9, "a", "A"),
        activity(2, 10, "b", "A"),
        activity(2, 11, "a", "A"),
    ];
    let now = *activities.last().unwrap().end_time().unwrap();
    assert!(latest_collapsed(&activities[2..], 1, false, now).is_none());
    assert!(latest_collapsed(&activities[1..], 1, false, now).is_none());
    let rows = latest_collapsed(&activities, 1, false, now).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].wbs(), "a");
    assert_eq!(rows[0].duration(), TimeDelta::hours(2));
    let rows = latest_collapsed(&activities, 2, false, now).unwrap();
    assert_eq!(rows.iter().map(|r| r.wbs()).collect::<Vec<_>>(), ["a", "b"]);
    assert!(latest_collapsed(&activities, 3, false, now).is_none());
    assert_eq!(
        latest_collapsed(&activities, 3, true, now).unwrap().len(),
        3
    );
    assert_eq!(
        latest_collapsed(&activities, 10, true, now).unwrap().len(),
        3
    );
    assert!(latest_collapsed(&[], 1, true, now).unwrap().is_empty());
}

#[test]
fn attendance_waits_for_a_break_and_counts_ranges_not_activities() {
    let activities = vec![
        activity(1, 8, "a", "B"),
        activity(1, 9, "a", "A"),
        activity(1, 10, "b", "A"),
        activity(1, 12, "c", "A"),
    ];
    assert!(latest_attendance_ranges(&activities[1..3], 1, false).is_none());
    let ranges = latest_attendance_ranges(&activities[..3], 1, false).unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(*ranges[0].start_time(), *activities[1].start_time());
    assert_eq!(ranges[0].end_time(), activities[2].end_time());
    assert_eq!(
        latest_attendance_ranges(&activities, 2, false)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        latest_attendance_ranges(&activities, 10, true)
            .unwrap()
            .len(),
        3
    );
    assert!(latest_attendance_ranges(&[], 1, true).unwrap().is_empty());
}

#[test]
fn clipping_limits_both_sides_and_drops_non_overlapping_activities() {
    let activity = activity(1, 9, "a", "A");
    let start = *activity.start_time() + TimeDelta::minutes(10);
    let end = *activity.end_time().unwrap() - TimeDelta::minutes(10);
    let clipped = activity.clipped(Some(start), end, end).unwrap();
    assert_eq!(*clipped.start_time(), start);
    assert_eq!(clipped.end_time(), Some(&end));
    assert_eq!(clipped.wbs(), activity.wbs());
    assert!(
        activity
            .clipped(Some(*activity.end_time().unwrap()), end, end)
            .is_none()
    );
    assert!(
        activity
            .clipped(None, *activity.start_time(), end)
            .is_none()
    );
    let ongoing = TrackedActivity::new_ongoing(activity.start_entry.clone());
    assert_eq!(
        ongoing
            .clipped(None, end, end + TimeDelta::hours(1))
            .unwrap()
            .end_time(),
        Some(&end)
    );
    assert_eq!(ongoing.clipped(None, end, end).unwrap().end_time(), None);
}
