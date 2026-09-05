//! Where a cell's colours came from, as opposed to what they resolved to.
//!
//! An embedder painting with this terminal's palette wants the resolved
//! values. One that owns a palette - a multiplexer inheriting the host
//! terminal's theme - wants the request, because a resolved colour has
//! already lost the difference between "the default foreground" and the
//! particular white this terminal was configured with.

use shitty_vt::{ColorSource, Rgb, Terminal};

/// Both halves of the top-left cell's colours, laid out like `Cell` itself.
struct Colors {
    foreground_source: ColorSource,
    foreground: Rgb,
    background_source: ColorSource,
    background: Rgb,
    underline_source: ColorSource,
    underline_color: Rgb,
    inverse: bool,
}

fn colors(input: &[u8]) -> Colors {
    let mut term = Terminal::new(10, 2, 0);
    term.feed(input);
    let mut found = None;
    term.for_each_cell(|row, column, cell| {
        if row == 0 && column == 0 {
            found = Some(Colors {
                foreground_source: cell.foreground_source,
                foreground: cell.foreground,
                background_source: cell.background_source,
                background: cell.background,
                underline_source: cell.underline_source,
                underline_color: cell.underline_color,
                inverse: cell.attributes.inverse(),
            });
        }
    });
    found.expect("the cell just written should be visited")
}

#[test]
fn plain_text_asks_for_the_defaults() {
    let cell = colors(b"a");

    assert_eq!(cell.foreground_source, ColorSource::DefaultForeground);
    assert_eq!(cell.background_source, ColorSource::DefaultBackground);
}

#[test]
fn an_ansi_request_stays_a_palette_entry() {
    assert_eq!(
        colors(b"\x1b[31ma").foreground_source,
        ColorSource::Indexed(1)
    );
    assert_eq!(
        colors(b"\x1b[41ma").background_source,
        ColorSource::Indexed(1)
    );
    // The bright half is the upper eight entries rather than a flag on the
    // lower ones, so an embedder can index a sixteen-colour palette with it
    // directly.
    assert_eq!(
        colors(b"\x1b[91ma").foreground_source,
        ColorSource::Indexed(9)
    );
}

#[test]
fn a_256_color_request_names_its_entry() {
    assert_eq!(
        colors(b"\x1b[38;5;200ma").foreground_source,
        ColorSource::Indexed(200)
    );
}

#[test]
fn a_direct_request_arrives_already_resolved() {
    let cell = colors(b"\x1b[38;2;1;2;3ma");

    assert_eq!(cell.foreground_source, ColorSource::Direct);
    assert_eq!(cell.foreground, Rgb { r: 1, g: 2, b: 3 });

    let cell = colors(b"\x1b[48;2;4;5;6ma");
    assert_eq!(cell.background_source, ColorSource::Direct);
    assert_eq!(cell.background, Rgb { r: 4, g: 5, b: 6 });
}

#[test]
fn a_reset_returns_to_the_defaults() {
    let cell = colors(b"\x1b[31m\x1b[39ma");

    assert_eq!(cell.foreground_source, ColorSource::DefaultForeground);
}

#[test]
fn a_redefined_palette_entry_is_still_that_entry() {
    // OSC 4 moves index 1 to blue. The resolved colour follows it and the
    // source does not: the application asked for ANSI red and still has,
    // whatever the palette now holds. This is the case that resolved RGB
    // alone cannot express, and the reason the field exists.
    let cell = colors(b"\x1b]4;1;rgb:00/00/ff\x07\x1b[31ma");

    assert_eq!(cell.foreground_source, ColorSource::Indexed(1));
    assert_eq!(cell.foreground, Rgb { r: 0, g: 0, b: 255 });
}

#[test]
fn the_underline_color_has_a_source_of_its_own() {
    let indexed = colors(b"\x1b[4;58;5;5ma");
    assert_eq!(indexed.underline_source, ColorSource::Indexed(5));
    assert_eq!(indexed.foreground_source, ColorSource::DefaultForeground);

    let direct = colors(b"\x1b[4;58;2;9;8;7ma");
    assert_eq!(direct.underline_source, ColorSource::Direct);
    assert_eq!(direct.underline_color, Rgb { r: 9, g: 8, b: 7 });
}

#[test]
fn an_unset_underline_color_follows_the_text() {
    // Nothing named an underline colour, so the cell underlines itself in
    // whatever it draws its text with - source and all.
    let cell = colors(b"\x1b[4;31ma");

    assert_eq!(cell.underline_source, ColorSource::Indexed(1));
    assert_eq!(cell.underline_color, cell.foreground);
}

#[test]
fn inverse_reports_the_colors_as_asked_for() {
    // Inverse is an attribute, not a pair of colours: the swap belongs to
    // whoever paints. Both sources describe the request, so an embedder
    // that swaps them itself gets its own defaults on both sides rather
    // than this terminal's black and white.
    let cell = colors(b"\x1b[7ma");

    assert!(cell.inverse);
    assert_eq!(cell.foreground_source, ColorSource::DefaultForeground);
    assert_eq!(cell.background_source, ColorSource::DefaultBackground);
}
