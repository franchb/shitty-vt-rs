//! Cell-level behaviour of the shitty VT core, in the same dump format the
//! Luvus `VtEngine` conformance tests use, so the two engines can be diffed
//! directly rather than by eye.

use shitty_vt::Terminal;

/// Visible grid as one `"col:CODEPOINT+CODEPOINT"` token per occupied cell.
/// Blanks are omitted, so a cluster that took two columns shows as a gap.
fn cell_dump(input: &[u8], cols: u16, rows: u16) -> Vec<String> {
    let mut term = Terminal::new(cols, rows, 100);
    term.feed(input);
    let mut out: Vec<Vec<String>> = vec![Vec::new(); rows as usize];
    term.for_each_cell(|row, col, cell| {
        let text = cell.text();
        if text.is_empty() || text == " " {
            return;
        }
        let points: Vec<String> = cell.grapheme.iter().map(|p| format!("{p:X}")).collect();
        out[row as usize].push(format!("{}:{}", col, points.join("+")));
    });
    out.into_iter().map(|row| row.join(" ")).collect()
}

#[test]
fn ascii_lands_one_cell_per_column() {
    assert_eq!(cell_dump(b"ab", 4, 3)[0], "0:61 1:62");
}

#[test]
fn wide_characters_occupy_two_columns() {
    assert_eq!(
        cell_dump("\u{65E5}\u{672C}".as_bytes(), 4, 3)[0],
        "0:65E5 2:672C"
    );
}

#[test]
fn combining_marks_stay_with_their_base_cell() {
    assert_eq!(cell_dump("e\u{301}x".as_bytes(), 4, 3)[0], "0:65+301 1:78");
}

#[test]
fn variation_selector_16_is_wide() {
    // Differs from alacritty_terminal, which gives U+2764 U+FE0F one column.
    // UTS #51 has VS16 select emoji presentation, which is wide.
    assert_eq!(
        cell_dump("\u{2764}\u{FE0F}x".as_bytes(), 4, 3)[0],
        "0:2764+FE0F 2:78"
    );
}

#[test]
fn emoji_zwj_sequence_is_one_wide_cell() {
    // Differs from alacritty_terminal, which splits this across two wide
    // cells for four columns total.
    let dump = cell_dump("\u{1F469}\u{200D}\u{1F4BB}x".as_bytes(), 4, 3);
    assert_eq!(dump[0], "0:1F469+200D+1F4BB 2:78");
}

#[test]
fn emoji_modifier_sequence_is_one_wide_cell() {
    let dump = cell_dump("\u{1F44D}\u{1F3FD}x".as_bytes(), 4, 3);
    assert_eq!(dump[0], "0:1F44D+1F3FD 2:78");
}

#[test]
fn text_soft_wraps_at_the_right_margin() {
    let dump = cell_dump(b"abcdefgh", 4, 3);
    assert_eq!(dump[0], "0:61 1:62 2:63 3:64");
    assert_eq!(dump[1], "0:65 1:66 2:67 3:68");
}

#[test]
fn sgr_colour_resolves_through_the_palette() {
    let mut term = Terminal::new(8, 2, 0);
    term.feed(b"\x1b[31mr\x1b[1;32mg\x1b[0m");
    let mut seen = Vec::new();
    term.for_each_cell(|_, col, cell| {
        if !cell.text().trim().is_empty() {
            seen.push((col, cell.foreground, cell.attributes.bold()));
        }
    });
    assert_eq!(seen.len(), 2, "expected two painted cells: {seen:?}");
    assert_ne!(seen[0].1, seen[1].1, "red and green must differ");
    assert!(!seen[0].2, "first cell is not bold");
    assert!(seen[1].2, "second cell is bold");
}

#[test]
fn device_attributes_query_produces_a_reply() {
    let mut term = Terminal::new(8, 2, 0);
    assert!(term.take_replies().is_empty());
    term.feed(b"\x1b[c");
    let reply = term.take_replies();
    assert!(
        reply.starts_with(b"\x1b["),
        "unexpected DA reply: {reply:?}"
    );
    assert!(term.take_replies().is_empty(), "replies must drain once");
}

#[test]
fn alternate_screen_and_bracketed_paste_report_through_modes() {
    let mut term = Terminal::new(8, 2, 0);
    assert!(!term.modes().alt_screen());
    assert!(!term.modes().bracketed_paste());
    term.feed(b"\x1b[?1049h\x1b[?2004h");
    assert!(term.modes().alt_screen());
    assert!(term.modes().bracketed_paste());
    term.feed(b"\x1b[?1049l");
    assert!(!term.modes().alt_screen());
}

#[test]
fn osc_title_reaches_the_embedder() {
    let mut term = Terminal::new(8, 2, 0);
    // The facade publishes an empty title at construction rather than
    // staying silent until the application sets one - pinned so a change
    // upstream is visible here.
    assert_eq!(term.title().as_deref(), Some(""));
    term.feed(b"\x1b]0;a pane\x07");
    assert_eq!(term.title().as_deref(), Some("a pane"));
    term.feed(b"\x1b]2;second\x1b\\");
    assert_eq!(term.title().as_deref(), Some("second"));
}

#[test]
fn cursor_tracks_the_written_text() {
    let mut term = Terminal::new(8, 2, 0);
    term.feed(b"abc");
    let cursor = term.cursor();
    assert_eq!((cursor.row, cursor.column), (0, 3));
    assert!(cursor.visible);
}

#[test]
fn resize_reflows_the_grid() {
    let mut term = Terminal::new(4, 3, 100);
    term.feed(b"abcdefgh");
    term.resize(8, 3);
    let mut first = String::new();
    term.for_each_cell(|row, _, cell| {
        if row == 0 {
            first.push_str(&cell.text());
        }
    });
    assert_eq!(
        first.trim_end(),
        "abcdefgh",
        "reflow should rejoin the line"
    );
}

fn lines(term: &Terminal, rows: u16) -> Vec<String> {
    let mut out = vec![String::new(); rows as usize];
    term.for_each_cell(|row, _, cell| out[row as usize].push_str(&cell.text()));
    out.into_iter().map(|l| l.trim_end().to_string()).collect()
}

#[test]
fn history_holds_what_scrolled_off_the_grid() {
    let mut term = Terminal::new(20, 6, 100);
    for index in 0..10 {
        term.feed(format!("line{index}\r\n").as_bytes());
    }
    // Ten lines plus the trailing newline is eleven rows over a six-row
    // grid, so five went into the history.
    assert_eq!(term.history_rows(), 5);
    assert_eq!(term.scroll_offset(), 0);
    assert_eq!(lines(&term, 6)[0], "line5");
}

#[test]
fn scrolling_moves_the_view_and_clamps() {
    let mut term = Terminal::new(20, 6, 100);
    for index in 0..10 {
        term.feed(format!("line{index}\r\n").as_bytes());
    }
    assert_eq!(term.scroll(2), 2);
    assert_eq!(lines(&term, 6)[0], "line3");

    assert_eq!(term.scroll(99), 5, "clamps to the retained history");
    assert_eq!(lines(&term, 6)[0], "line0");

    assert_eq!(term.scroll_to(0), 0, "back to live");
    assert_eq!(lines(&term, 6)[0], "line5");
}

#[test]
fn a_terminal_keeping_no_lines_has_nothing_to_scroll() {
    let mut term = Terminal::new(20, 6, 0);
    for index in 0..10 {
        term.feed(format!("line{index}\r\n").as_bytes());
    }
    assert_eq!(term.history_rows(), 0);
    assert_eq!(term.scroll(5), 0);
}

#[test]
fn history_rows_are_addressable_without_moving_the_view() {
    let mut term = Terminal::new(20, 6, 100);
    for index in 0..10 {
        term.feed(format!("line{index}\r\n").as_bytes());
    }
    assert_eq!(term.total_rows(), 11);

    let row_text = |term: &Terminal, index: u32| {
        let mut text = String::new();
        term.row_cells(index, |_, _, cell| text.push_str(&cell.text()));
        text.trim_end().to_string()
    };
    assert_eq!(row_text(&term, 0), "line0", "oldest retained row");
    assert_eq!(row_text(&term, 9), "line9", "last written row");
    assert_eq!(
        row_text(&term, 10),
        "",
        "the row the trailing newline opened"
    );

    // Scrolling the view must not move what an index means.
    term.scroll(3);
    assert_eq!(term.scroll_offset(), 3);
    assert_eq!(row_text(&term, 0), "line0");
    assert_eq!(row_text(&term, 9), "line9");

    // Past the end visits nothing rather than misbehaving.
    let mut visited = 0;
    term.row_cells(11, |_, _, _| visited += 1);
    assert_eq!(visited, 0);
}
