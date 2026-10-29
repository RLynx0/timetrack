use super::*;

#[test]
fn widths_shrink_the_widest_columns_and_distribute_the_remainder() {
    assert_eq!(fit_widths(&[5, 20, 10], 31), Some(vec![5, 8, 8]));
    assert_eq!(fit_widths(&[5, 20, 10], 30), Some(vec![5, 8, 7]));
    assert_eq!(fit_widths(&[20], 12), Some(vec![8]));
    assert_eq!(fit_widths(&[20, 2, 0], 16), Some(vec![4, 2, 0]));
    assert_eq!(fit_widths(&[5, 20, 10], 45), Some(vec![5, 20, 10]));
    assert_eq!(fit_widths(&[5, 20, 10], 80), Some(vec![5, 20, 10]));
}

#[test]
fn impossible_widths_fall_back_instead_of_overflowing() {
    assert_eq!(fit_widths(&[20, 2, 0], 15), None);
    assert_eq!(fit_widths(&[20], 0), None);
    assert_eq!(fit_widths(&[0, 0], 6), None);
    assert_eq!(fit_widths(&[], 0), None);
    assert_eq!(fit_widths(&[], 1), Some(vec![]));
}

#[test]
fn fitted_widths_fill_every_feasible_budget() {
    for natural in [vec![5, 20, 10], vec![0, 1, 30], vec![4, 4], vec![]] {
        let overhead = 3 * natural.len() + 1;
        let minimum = overhead + natural.iter().map(|&w| w.min(4)).sum::<usize>();
        let full = overhead + natural.iter().sum::<usize>();
        for limit in 0..=full + 2 {
            let fitted = fit_widths(&natural, limit);
            if limit < minimum {
                assert!(fitted.is_none());
            } else {
                let fitted = fitted.unwrap();
                assert_eq!(fitted.iter().sum::<usize>() + overhead, limit.min(full));
                for (&width, &original) in fitted.iter().zip(&natural) {
                    assert!((original.min(4)..=original).contains(&width));
                }
            }
        }
    }
}

#[test]
fn truncation_uses_display_width_and_preserves_fitting_text() {
    assert!(matches!(
        truncate_to_width("Hello", 5),
        Cow::Borrowed("Hello")
    ));
    assert_eq!(truncate_to_width("Hello world", 5), "He...");
    assert_eq!(truncate_to_width("Hello world", 4), "H...");
    assert_eq!(truncate_to_width("Hello", 2), "..");
    assert_eq!(truncate_to_width("Hello", 0), "");
    // Wide and combining characters must not be measured as one column each.
    assert_eq!(truncate_to_width("界界界界", 5), "界...");
    assert_eq!(fit_cell("界界界界", 6), (Cow::from("界..."), " ".into()));
    assert_eq!(truncate_to_width("😀😀😀😀", 5), "😀...");
    assert_eq!(truncate_to_width("e\u{301}long text", 4), "e\u{301}...");
}

fn render(table: &Table<&str, &str>) -> String {
    owo_colors::set_override(false);
    let mut output = Vec::new();
    table
        .render(&mut output, table.options.width.resolve())
        .unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn headers_rows_and_borders_use_the_same_fitted_width() {
    for chars in [
        TableCharOptions::rounded(),
        TableCharOptions::_sharp(),
        TableCharOptions::ascii_markdown(),
    ] {
        let mut table = Table::from([
            ("Long header", ["Hello world", "界界界界"]),
            ("Key", ["short", "value"]),
        ]);
        table.with_options(TablePrintOptions {
            chars,
            width: TableWidth::Fixed(19),
        });
        let fitted = render(&table);
        assert!(fitted.contains("Long..."));
        assert!(fitted.contains("Hell..."));
        assert!(fitted.contains("界界..."));
        assert!(fitted.lines().all(|line| display_width(line) == 19));

        table.options.width = TableWidth::Unbounded;
        let full = render(&table);
        assert!(full.contains("Long header"));
        assert!(full.contains("Hello world"));
        assert!(full.lines().all(|line| display_width(line) == 23));
        table.options.width = TableWidth::Fixed(14);
        assert_eq!(render(&table), full);
        table.options.width = TableWidth::Fixed(80);
        assert_eq!(render(&table), full);
    }
}
