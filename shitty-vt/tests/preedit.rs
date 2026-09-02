//! The composition preview: an input method's uncommitted text, drawn over
//! the cursor row and belonging to no one else - not the grid, not the
//! scrollback, not the child.

use shitty_vt::{Cell, Terminal};

/// The preview as `(row, column, text)` per cell.
fn preview(term: &Terminal) -> Vec<(u16, u16, String)> {
    let mut cells = Vec::new();
    term.preedit_cells(|row, column, cell| cells.push((row, column, cell.text())));
    cells
}

/// The preview's text alone.
fn preview_text(term: &Terminal) -> String {
    preview(term).into_iter().map(|(_, _, text)| text).collect()
}

/// The visible grid's first row, trailing blanks trimmed.
fn first_row(term: &Terminal) -> String {
    let mut row = String::new();
    term.for_each_cell(|r, _, cell| {
        if r == 0 {
            row.push_str(&cell.text());
        }
    });
    row.trim_end().to_string()
}

fn composing(text: &str, cursor: Option<std::ops::Range<usize>>) -> Terminal {
    let mut term = Terminal::new(20, 6, 0);
    term.feed(b"hello");
    term.take_replies();
    term.set_preedit(text, cursor);
    term
}

#[test]
fn the_preview_is_drawn_from_the_cursor() {
    let term = composing("abc", Some(0..3));
    assert_eq!(
        preview(&term),
        vec![
            (0, 5, "a".to_string()),
            (0, 6, "b".to_string()),
            (0, 7, "c".to_string()),
        ]
    );
}

#[test]
fn the_preview_stays_out_of_the_grid_and_out_of_the_child() {
    let mut term = composing("abc", Some(0..3));
    // The row still holds what the application wrote, and the child hears
    // nothing until the input method commits.
    assert_eq!(first_row(&term), "hello");
    assert_eq!(term.take_replies(), b"");
}

#[test]
fn the_cursor_hides_and_anchors_the_candidate_window() {
    // An input method draws its candidate list at the cursor, so while
    // composing the cursor tracks the preview rather than the application.
    let term = composing("abc", Some(1..1));
    assert_eq!((term.cursor().column, term.cursor().row), (6, 0));
    assert!(!term.cursor().visible);
}

#[test]
fn an_empty_preview_clears_the_composition() {
    let mut term = composing("abc", Some(0..3));
    term.clear_preedit();

    assert!(preview(&term).is_empty());
    // And the application's cursor comes back where it was.
    assert_eq!((term.cursor().column, term.cursor().row), (5, 0));
    assert!(term.cursor().visible);
}

#[test]
fn a_wide_character_takes_two_columns_of_the_preview() {
    // Continuations are not reported, as everywhere else: two cells
    // covering four columns.
    let term = composing("\u{65E5}\u{672C}", Some(0..6));
    assert_eq!(
        preview(&term),
        vec![
            (0, 5, "\u{65E5}".to_string()),
            (0, 7, "\u{672C}".to_string()),
        ]
    );
}

#[test]
fn a_preview_too_long_for_the_row_keeps_the_fresh_end() {
    let mut term = Terminal::new(20, 6, 0);
    term.set_preedit("abcdefghijklmnopqrstuvwxyz", Some(0..26));

    let cells = preview(&term);
    assert_eq!(cells.len(), 20);
    assert_eq!(cells[0].1, 0, "the clipped preview starts at the margin");
    assert_eq!(preview_text(&term), "ghijklmnopqrstuvwxyz");
}

#[test]
fn a_cursor_range_past_the_text_is_clamped() {
    let term = composing("ab", Some(0..99));
    assert_eq!(preview_text(&term), "ab");
    assert_eq!((term.cursor().column, term.cursor().row), (5, 0));
}

#[test]
fn a_double_width_row_carries_no_preview() {
    let mut term = Terminal::new(20, 6, 0);
    term.feed(b"\x1b#6hello");
    term.set_preedit("abc", Some(0..3));

    assert!(preview(&term).is_empty());
}

#[test]
fn the_preview_underlines_itself_and_reverses_the_cursor_range() {
    // How the preview is styled is the terminal's business, and this is
    // what it decides: the composition underlined, the input method's own
    // cursor range in reverse video.
    let mut term = composing("abcd", Some(1..3));
    let mut styles = Vec::new();
    term.preedit_cells(|_, _, cell: Cell<'_>| {
        styles.push((cell.text(), cell.underline_style, cell.attributes.inverse()))
    });

    assert_eq!(
        styles,
        vec![
            ("a".to_string(), 1, false),
            ("b".to_string(), 0, true),
            ("c".to_string(), 0, true),
            ("d".to_string(), 1, false),
        ]
    );

    term.clear_preedit();
    assert!(preview(&term).is_empty());
}

#[test]
fn no_cursor_range_leaves_the_whole_preview_underlined() {
    let term = composing("ab", None);
    let mut styles = Vec::new();
    term.preedit_cells(|_, _, cell| styles.push((cell.underline_style, cell.attributes.inverse())));

    assert_eq!(styles, vec![(1, false), (1, false)]);
}

#[test]
fn a_cluster_renders_unlike_the_same_text_in_the_grid() {
    // Upstream pg83/shitty#109: the preview drops every zero-width
    // codepoint instead of joining it to the cell before it, so a
    // combining mark disappears and an emoji sequence splits. The grid
    // does neither. Pinned rather than worked around - the bindings hand
    // over the text they are given - so that a fix upstream shows up here
    // as a failure rather than as silence.
    let cases = [
        // (text, preview cells as (column, text), grid cells)
        ("e\u{301}", vec![(5, "e")], vec![(0, "e\u{301}")]),
        (
            "\u{2764}\u{FE0F}",
            vec![(5, "\u{2764}")],
            vec![(0, "\u{2764}\u{FE0F}")],
        ),
        (
            "\u{1F469}\u{200D}\u{1F4BB}",
            vec![(5, "\u{1F469}"), (7, "\u{1F4BB}")],
            vec![(0, "\u{1F469}\u{200D}\u{1F4BB}")],
        ),
    ];

    for (text, expected_preview, expected_grid) in cases {
        let term = composing(text, Some(0..text.len()));
        let shown: Vec<(u16, String)> = preview(&term)
            .into_iter()
            .map(|(_, column, cell)| (column, cell))
            .collect();
        let expected: Vec<(u16, String)> = expected_preview
            .into_iter()
            .map(|(column, cell)| (column, cell.to_string()))
            .collect();
        assert_eq!(shown, expected, "preview of {text:?}");

        let mut grid = Terminal::new(20, 6, 0);
        grid.feed(text.as_bytes());
        let mut cells = Vec::new();
        grid.for_each_cell(|row, column, cell| {
            if row == 0 && !cell.text().trim().is_empty() {
                cells.push((column, cell.text()));
            }
        });
        let expected: Vec<(u16, String)> = expected_grid
            .into_iter()
            .map(|(column, cell)| (column, cell.to_string()))
            .collect();
        assert_eq!(cells, expected, "grid of {text:?}");
    }
}

#[test]
fn a_single_codepoint_of_any_width_reaches_the_preview() {
    // The facade decodes the preview strictly and drops one that is not
    // UTF-8; `&str` cannot be that, so the Rust side has no such case.
    for text in ["\u{1F600}", "\u{10FFFF}", "\u{65E5}", "z"] {
        let term = composing(text, Some(0..text.len()));
        assert_eq!(preview_text(&term), text, "{text:?} should show whole");
    }
}
