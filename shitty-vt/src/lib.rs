//! A safe wrapper over shitty's embeddable VT core.
//!
//! Feed bytes in, read a grid out. The terminal owns no pty and spawns no
//! child: replies it generates (DA, DSR, ...) queue up until [`Terminal::take_replies`]
//! drains them, and it is the caller's job to forward those to whatever is on
//! the other end.
//!
//! ```no_run
//! # use shitty_vt::Terminal;
//! let mut term = Terminal::new(80, 24, 1000);
//! term.feed(b"\x1b[31mhello\x1b[0m");
//! term.for_each_cell(|row, col, cell| {
//!     println!("{row},{col}: {}", cell.text());
//! });
//! ```

use std::ffi::{c_int, c_void};

use shitty_vt_sys as sys;

/// Text attributes on a cell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Attributes(pub u16);

impl Attributes {
    pub fn bold(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_BOLD != 0
    }
    pub fn faint(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_FAINT != 0
    }
    pub fn italic(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_ITALIC != 0
    }
    pub fn blink(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_BLINK != 0
    }
    pub fn inverse(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_INVERSE != 0
    }
    pub fn conceal(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_CONCEAL != 0
    }
    pub fn strike(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_STRIKE != 0
    }
    pub fn overline(self) -> bool {
        self.0 & sys::SHITTY_VT_ATTR_OVERLINE != 0
    }
}

/// An 8-bit-per-channel colour, already resolved through the palette.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// The facade hands out `0x00BBGGRR` — the little-endian view of
    /// `struct { uint8_t r, g, b; }`.
    fn from_packed(packed: u32) -> Self {
        Rgb {
            r: (packed & 0xff) as u8,
            g: ((packed >> 8) & 0xff) as u8,
            b: ((packed >> 16) & 0xff) as u8,
        }
    }
}

/// One visible cell. Borrowed for the duration of the visit callback.
#[derive(Clone, Copy, Debug)]
pub struct Cell<'a> {
    /// The cell's grapheme cluster as codepoints. Empty for a blank cell.
    pub grapheme: &'a [u32],
    pub foreground: Rgb,
    pub background: Rgb,
    pub underline_color: Rgb,
    pub attributes: Attributes,
    /// 0 none, 1 straight, 2 double, 3 curly, 4 dotted, 5 dashed
    pub underline_style: u8,
    /// 1 or 2. The continuation column of a wide cell is never visited.
    pub width: u8,
}

impl Cell<'_> {
    /// The cluster as a `String`. Allocates; for hot paths read
    /// [`Cell::grapheme`] directly.
    pub fn text(&self) -> String {
        self.grapheme
            .iter()
            .filter_map(|p| char::from_u32(*p))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Cursor {
    pub column: u16,
    pub row: u16,
    /// 0 hidden, 1 filled block, 2 hollow block, 3 underline, 4 bar
    pub style: u8,
    pub visible: bool,
}

/// Mode flags the application has set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modes(pub u32);

macro_rules! mode {
    ($name:ident, $bit:ident) => {
        pub fn $name(self) -> bool {
            self.0 & sys::$bit != 0
        }
    };
}

impl Modes {
    mode!(alt_screen, SHITTY_VT_MODE_ALT_SCREEN);
    mode!(bracketed_paste, SHITTY_VT_MODE_BRACKETED_PASTE);
    mode!(app_cursor_keys, SHITTY_VT_MODE_APP_CURSOR_KEYS);
    mode!(app_keypad, SHITTY_VT_MODE_APP_KEYPAD);
    mode!(focus_events, SHITTY_VT_MODE_FOCUS_EVENTS);
    mode!(auto_wrap, SHITTY_VT_MODE_AUTO_WRAP);
    mode!(origin, SHITTY_VT_MODE_ORIGIN);
    mode!(insert, SHITTY_VT_MODE_INSERT);
    mode!(cursor_visible, SHITTY_VT_MODE_CURSOR_VISIBLE);
    mode!(screen_reverse, SHITTY_VT_MODE_SCREEN_REVERSE);
    mode!(synchronized_output, SHITTY_VT_MODE_SYNCHRONIZED_OUTPUT);
    mode!(mouse_click, SHITTY_VT_MODE_MOUSE_CLICK);
    mode!(mouse_drag, SHITTY_VT_MODE_MOUSE_DRAG);
    mode!(mouse_motion, SHITTY_VT_MODE_MOUSE_MOTION);
    mode!(mouse_sgr, SHITTY_VT_MODE_MOUSE_SGR);
}

/// What the terminal reported through its callbacks since the last read.
#[derive(Clone, Debug, Default)]
struct Events {
    title: Option<Vec<u8>>,
    bells: u64,
    damaged: bool,
    open_uri: Vec<Vec<u8>>,
    clipboard: Vec<(i32, Vec<u8>)>,
    resize_request: Option<(u16, u16)>,
}

/// An embedded terminal.
pub struct Terminal {
    raw: *mut sys::shitty_vt,
    // Boxed for a stable address: the terminal retains the pointer to the
    // callbacks struct, and the callbacks carry a pointer to `events`.
    // Declared after `raw` only for clarity; drop order is handled explicitly.
    _callbacks: Box<sys::shitty_vt_callbacks>,
    events: Box<Events>,
}

// The facade hands out one opaque instance with no shared global state, and
// every entry point takes that instance. Sending one between threads is
// therefore fine; using it from two at once is not, which is why there is no
// `Sync`.
unsafe impl Send for Terminal {}

unsafe extern "C" fn on_title(user: *mut c_void, bytes: *const u8, len: usize) {
    let events = &mut *(user as *mut Events);
    events.title = Some(std::slice::from_raw_parts(bytes, len).to_vec());
}

unsafe extern "C" fn on_bell(user: *mut c_void) {
    (*(user as *mut Events)).bells += 1;
}

unsafe extern "C" fn on_damaged(user: *mut c_void) {
    (*(user as *mut Events)).damaged = true;
}

unsafe extern "C" fn on_open_uri(user: *mut c_void, bytes: *const u8, len: usize) {
    let events = &mut *(user as *mut Events);
    events
        .open_uri
        .push(std::slice::from_raw_parts(bytes, len).to_vec());
}

unsafe extern "C" fn on_clipboard(user: *mut c_void, which: c_int, bytes: *const u8, len: usize) {
    let events = &mut *(user as *mut Events);
    events
        .clipboard
        .push((which, std::slice::from_raw_parts(bytes, len).to_vec()));
}

unsafe extern "C" fn on_resize_request(user: *mut c_void, columns: u16, rows: u16) {
    (*(user as *mut Events)).resize_request = Some((columns, rows));
}

impl Terminal {
    /// Builds a terminal with `columns` x `rows` visible and `save_lines`
    /// rows of scrollback retained.
    pub fn new(columns: u16, rows: u16, save_lines: u16) -> Terminal {
        let mut events = Box::new(Events::default());
        let user = (&mut *events) as *mut Events as *mut c_void;
        let callbacks = Box::new(sys::shitty_vt_callbacks {
            user,
            title_changed: Some(on_title),
            bell: Some(on_bell),
            damaged: Some(on_damaged),
            open_uri: Some(on_open_uri),
            clipboard_set: Some(on_clipboard),
            resize_request: Some(on_resize_request),
        });
        // SAFETY: `callbacks` outlives the terminal - both are owned here and
        // the terminal is freed first in `Drop`.
        let raw = unsafe {
            sys::shitty_vt_new(
                columns.max(1),
                rows.max(1),
                save_lines,
                &*callbacks as *const _,
            )
        };
        assert!(!raw.is_null(), "shitty_vt_new returned null");
        Terminal {
            raw,
            _callbacks: callbacks,
            events,
        }
    }

    /// Parses `bytes` into the grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        unsafe { sys::shitty_vt_feed(self.raw, bytes.as_ptr(), bytes.len()) }
    }

    pub fn resize(&mut self, columns: u16, rows: u16) {
        unsafe { sys::shitty_vt_resize(self.raw, columns.max(1), rows.max(1)) }
    }

    /// Drains terminal-generated replies. Forward these to the child.
    pub fn take_replies(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            // SAFETY: `chunk` is writable for its whole length.
            let taken =
                unsafe { sys::shitty_vt_take_replies(self.raw, chunk.as_mut_ptr(), chunk.len()) };
            if taken == 0 {
                break;
            }
            out.extend_from_slice(&chunk[..taken]);
            if taken < chunk.len() {
                break;
            }
        }
        out
    }

    /// Visits every visible cell row-major. Wide-cell continuations are
    /// skipped, so a `width == 2` cell is followed by a column gap.
    pub fn for_each_cell<F: FnMut(u16, u16, Cell<'_>)>(&self, mut f: F) {
        unsafe extern "C" fn visit<F: FnMut(u16, u16, Cell<'_>)>(
            user: *mut c_void,
            row: u16,
            column: u16,
            cell: *const sys::shitty_vt_cell,
        ) {
            let cell = &*cell;
            let grapheme = if cell.grapheme.is_null() || cell.grapheme_len == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(cell.grapheme, cell.grapheme_len)
            };
            (*(user as *mut F))(
                row,
                column,
                Cell {
                    grapheme,
                    foreground: Rgb::from_packed(cell.foreground),
                    background: Rgb::from_packed(cell.background),
                    underline_color: Rgb::from_packed(cell.underline_color),
                    attributes: Attributes(cell.attributes),
                    underline_style: cell.underline_style,
                    width: cell.width,
                },
            );
        }
        // SAFETY: `visit::<F>` is only ever called with `user` pointing at
        // `f`, and only for the duration of this call.
        unsafe {
            sys::shitty_vt_each_cell(self.raw, visit::<F>, &mut f as *mut F as *mut c_void);
        }
    }

    pub fn cursor(&self) -> Cursor {
        // SAFETY: `raw` is valid for the terminal's lifetime.
        let cursor = unsafe { sys::shitty_vt_cursor_state(self.raw) };
        Cursor {
            column: cursor.column,
            row: cursor.row,
            style: cursor.style,
            visible: cursor.visible != 0,
        }
    }

    pub fn modes(&self) -> Modes {
        Modes(unsafe { sys::shitty_vt_modes(self.raw) })
    }

    /// The most recent title the application set.
    ///
    /// Note that the facade publishes an empty title once at construction, so
    /// this reads `Some("")` before the application has set anything. Treat
    /// emptiness, not `None`, as "no title yet".
    pub fn title(&self) -> Option<String> {
        self.events
            .title
            .as_ref()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    /// Number of bells since the terminal was created.
    pub fn bells(&self) -> u64 {
        self.events.bells
    }

    /// Whether the presentation moved since [`Terminal::clear_damage`].
    pub fn damaged(&self) -> bool {
        self.events.damaged
    }

    pub fn clear_damage(&mut self) {
        self.events.damaged = false;
    }

    /// The grid size the application last asked for through XTWINOPS, if any.
    /// Answering it is the embedder's choice.
    pub fn take_resize_request(&mut self) -> Option<(u16, u16)> {
        self.events.resize_request.take()
    }

    /// OSC 52 selections the application set: `(0 primary | 1 clipboard, bytes)`.
    pub fn take_clipboard_writes(&mut self) -> Vec<(i32, Vec<u8>)> {
        std::mem::take(&mut self.events.clipboard)
    }

    /// Hyperlinks the application asked to open.
    pub fn take_open_uris(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.events.open_uri)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // The terminal holds a pointer into `_callbacks` and `events`, so it
        // has to go first; the boxes are dropped after this returns.
        unsafe { sys::shitty_vt_free(self.raw) }
    }
}
