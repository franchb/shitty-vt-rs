//! A safe wrapper over shitty's embeddable VT core.
//!
//! Feed bytes in, read a grid out. The terminal owns no pty and spawns no
//! child: replies it generates (DA, DSR, ...) queue up until [`Terminal::take_replies`]
//! drains them, and it is the caller's job to forward those to whatever is on
//! the other end.
//!
//! Input goes the same way. Report events - [`Terminal::key`],
//! [`Terminal::text`], the mouse entry points, [`Terminal::paste`] - and the
//! terminal encodes them by whatever protocol the application negotiated,
//! queueing the bytes in the same reply buffer. The embedder never encodes,
//! because the state that decides the encoding (kitty flags, modifyOtherKeys,
//! the mouse modes) lives inside the terminal.
//!
//! ```no_run
//! # use shitty_vt::Terminal;
//! let mut term = Terminal::new(80, 24, 1000);
//! term.feed(b"\x1b[31mhello\x1b[0m");
//! term.for_each_cell(|row, col, cell| {
//!     println!("{row},{col}: {}", cell.text());
//! });
//! ```

use std::cell::UnsafeCell;
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

    /// DECSET 1007. While [`Modes::alt_screen`] is also set, wheel input
    /// belongs to the application as arrow keys rather than moving a history
    /// the alternate screen does not keep.
    pub fn alternate_scroll(self) -> bool {
        self.0 & sys::SHITTY_VT_MODE_ALTERNATE_SCROLL != 0
    }
}

/// What the terminal is spending on its grid and history.
///
/// Cells only. Grapheme clusters, hyperlinks and sixel patches live in a
/// separate store this does not count, so treat it as the floor of the real
/// cost rather than the whole of it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Memory {
    /// Row slots actually backed by cells. The ring behind them is rounded to
    /// a power of two, so this can exceed [`Memory::capacity_rows`]: it is
    /// what the screen costs, not what it is allowed to hold.
    pub allocated_rows: u32,
    /// Rows the terminal will keep: the visible grid plus `save_lines`.
    pub capacity_rows: u32,
    pub columns: u32,
    pub cell_size: u32,
    /// `allocated_rows * columns * cell_size`.
    pub cell_bytes: u64,
}

/// A physical key, mirroring the facade's pinned codes.
///
/// [`Key::Printable`] covers every character key: which character it is
/// travels in the event's codepoints, not in the code.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Key {
    Unknown = 0,
    Printable,
    Space,
    Escape,
    Enter,
    Backspace,
    Tab,
    Insert,
    Delete,
    Home,
    End,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Clear,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    KeypadDecimal,
    KeypadDivide,
    KeypadMultiply,
    KeypadSubtract,
    KeypadAdd,
    KeypadEnter,
    KeypadEqual,
    KeypadSeparator,
    KeypadF1,
    KeypadF2,
    KeypadF3,
    KeypadF4,
    KeypadInsert,
    KeypadDelete,
    KeypadUp,
    KeypadDown,
    KeypadLeft,
    KeypadRight,
    KeypadHome,
    KeypadEnd,
    KeypadPageUp,
    KeypadPageDown,
    KeypadBegin,
    KeypadSpace,
    KeypadTab,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    MediaPlay,
    MediaPause,
    MediaPlayPause,
    MediaReverse,
    MediaStop,
    MediaFastForward,
    MediaRewind,
    MediaTrackNext,
    MediaTrackPrevious,
    MediaRecord,
    VolumeDown,
    VolumeUp,
    VolumeMute,
}

impl Key {
    /// One past the last key code: every code below this names a key.
    pub const COUNT: u16 = sys::SHITTY_VT_KEY_COUNT;

    /// The pinned C code.
    pub fn code(self) -> u16 {
        self as u16
    }

    /// The key a code names, or `None` for a code this build does not know.
    pub fn from_code(code: u16) -> Option<Key> {
        // SAFETY: the codes run 0..COUNT with no gaps and each one is a
        // variant above - `key_codes_match_the_facade` asserts all 115 of
        // them against the header, one by one.
        (code < sys::SHITTY_VT_KEY_COUNT).then(|| unsafe { std::mem::transmute::<u16, Key>(code) })
    }
}

/// Modifiers held when an event happened.
///
/// The lock states are not decoration: NumLock chooses between the two
/// identities of a keypad key, and the kitty protocol reports both locks when
/// the application asks it to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Modifiers(pub u16);

impl Modifiers {
    pub const NONE: Modifiers = Modifiers(0);
    pub const SHIFT: Modifiers = Modifiers(sys::SHITTY_VT_MOD_SHIFT);
    pub const CONTROL: Modifiers = Modifiers(sys::SHITTY_VT_MOD_CONTROL);
    pub const ALT: Modifiers = Modifiers(sys::SHITTY_VT_MOD_ALT);
    pub const SUPER: Modifiers = Modifiers(sys::SHITTY_VT_MOD_SUPER);
    pub const CAPS_LOCK: Modifiers = Modifiers(sys::SHITTY_VT_MOD_CAPS_LOCK);
    pub const NUM_LOCK: Modifiers = Modifiers(sys::SHITTY_VT_MOD_NUM_LOCK);
    pub const ALT_GRAPH: Modifiers = Modifiers(sys::SHITTY_VT_MOD_ALT_GRAPH);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every modifier in `other` is held.
    pub fn contains(self, other: Modifiers) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Modifiers;

    fn bitor(self, other: Modifiers) -> Modifiers {
        Modifiers(self.0 | other.0)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, other: Modifiers) {
        self.0 |= other.0;
    }
}

/// What happened to a key. Repeat and release exist for the kitty protocol; a
/// legacy application sees presses and repeats alike and releases not at all.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum KeyAction {
    #[default]
    Press = 0,
    Repeat = 1,
    Release = 2,
}

/// One physical key event.
///
/// The three codepoints are the key's unicode identity in the active layout
/// unshifted, in the base (ASCII) layout, and with Shift in the active
/// layout. They feed chord encoding and the kitty protocol's alternate keys.
/// Leaving them `None` is well-formed: a named key has none, and an embedder
/// that knows only the layout character sets only that one.
///
/// Fields are public and the type is `Copy`, so the constructors below are a
/// starting point rather than a wall:
///
/// ```
/// # use shitty_vt::{Key, KeyEvent, Modifiers};
/// let ctrl_c = KeyEvent {
///     modifiers: Modifiers::CONTROL,
///     shifted: Some('C'),
///     ..KeyEvent::printable('c')
/// };
/// # let _ = ctrl_c;
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyEvent {
    pub key: Key,
    pub action: KeyAction,
    pub modifiers: Modifiers,
    pub layout: Option<char>,
    pub base: Option<char>,
    pub shifted: Option<char>,
}

impl KeyEvent {
    /// A press of a named key, unmodified.
    pub fn press(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            action: KeyAction::Press,
            modifiers: Modifiers::NONE,
            layout: None,
            base: None,
            shifted: None,
        }
    }

    /// A repeat of a held key.
    pub fn repeat(key: Key) -> KeyEvent {
        KeyEvent {
            action: KeyAction::Repeat,
            ..KeyEvent::press(key)
        }
    }

    /// A release. Only the kitty protocol reports these; under any other
    /// encoding the terminal drops them.
    pub fn release(key: Key) -> KeyEvent {
        KeyEvent {
            action: KeyAction::Release,
            ..KeyEvent::press(key)
        }
    }

    /// A press of a character key, with `ch` as both the layout and the base
    /// identity. That is right for a latin layout; on any other, set
    /// [`KeyEvent::base`] to the character the same physical key types in the
    /// ASCII layout, which is what a chord like Ctrl+C is encoded from.
    pub fn printable(ch: char) -> KeyEvent {
        KeyEvent {
            layout: Some(ch),
            base: Some(ch),
            ..KeyEvent::press(Key::Printable)
        }
    }

    /// The same event with `modifiers` held.
    pub fn with_modifiers(self, modifiers: Modifiers) -> KeyEvent {
        KeyEvent { modifiers, ..self }
    }

    fn to_raw(self) -> sys::shitty_vt_key_event {
        sys::shitty_vt_key_event {
            key: self.key.code(),
            action: self.action as u8,
            modifiers: self.modifiers.0,
            layout_codepoint: self.layout.map_or(0, u32::from),
            base_codepoint: self.base.map_or(0, u32::from),
            shifted_codepoint: self.shifted.map_or(0, u32::from),
        }
    }
}

/// A pointer button. The protocols' own numbering is the terminal's business.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
    Aux1 = 3,
    Aux2 = 4,
    Aux3 = 5,
    Aux4 = 6,
    Aux5 = 7,
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

/// Hands one C cell to a Rust closure. Shared by every walk, so the two
/// entry points cannot drift in how they decode a cell.
///
/// # Safety
/// `user` must point at a live `F` for the duration of the call, and `cell`
/// must be valid; both hold for the walks below, which pass a stack closure
/// and are synchronous.
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

/// An embedded terminal.
pub struct Terminal {
    raw: *mut sys::shitty_vt,
    // Raw pointers rather than `Box` fields, deliberately. The terminal
    // keeps the address of both for its whole life and writes through the
    // second from its callbacks; a `Box` field would be reborrowed by every
    // `&mut self` method, invalidating the pointer C still holds. The
    // `UnsafeCell` is what makes those writes legal while a `&self` method
    // is reading, since a callback can fire from inside a C call.
    callbacks: *mut sys::shitty_vt_callbacks,
    events: *mut UnsafeCell<Events>,
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
        let events: *mut UnsafeCell<Events> =
            Box::into_raw(Box::new(UnsafeCell::new(Events::default())));
        // SAFETY: `events` was just allocated and is not aliased yet.
        let user = unsafe { (*events).get() } as *mut c_void;
        let callbacks = Box::into_raw(Box::new(sys::shitty_vt_callbacks {
            user,
            title_changed: Some(on_title),
            bell: Some(on_bell),
            damaged: Some(on_damaged),
            open_uri: Some(on_open_uri),
            clipboard_set: Some(on_clipboard),
            resize_request: Some(on_resize_request),
        }));
        // SAFETY: both allocations outlive the terminal - `Drop` frees it
        // first and them after.
        let raw = unsafe { sys::shitty_vt_new(columns.max(1), rows.max(1), save_lines, callbacks) };
        if raw.is_null() {
            // SAFETY: the terminal never took ownership, so these are still
            // ours to release.
            unsafe {
                drop(Box::from_raw(callbacks));
                drop(Box::from_raw(events));
            }
            panic!("shitty_vt_new returned null");
        }
        Terminal {
            raw,
            callbacks,
            events,
        }
    }

    /// The callback state.
    ///
    /// # Safety
    /// The returned pointer is valid for the terminal's life. Do not hold a
    /// reference derived from it across a call into C, which may write
    /// through the same pointer from a callback.
    fn events(&self) -> *mut Events {
        // SAFETY: `events` is live for as long as `self` is.
        unsafe { (*self.events).get() }
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
        // SAFETY: `visit::<F>` is only ever called with `user` pointing at
        // `f`, and only for the duration of this call.
        unsafe {
            sys::shitty_vt_each_cell(self.raw, visit::<F>, &mut f as *mut F as *mut c_void);
        }
    }

    /// Rows addressable through [`Terminal::row_cells`]: the retained history
    /// followed by the visible grid.
    pub fn total_rows(&self) -> u32 {
        unsafe { sys::shitty_vt_total_rows(self.raw) }
    }

    /// Visits one row by absolute index, oldest first, without moving the
    /// view. Index 0 is the oldest retained row; the last index is the bottom
    /// of the live screen. An index past the end visits nothing.
    pub fn row_cells<F: FnMut(u16, u16, Cell<'_>)>(&self, index: u32, mut f: F) {
        // SAFETY: as in `for_each_cell`.
        unsafe {
            sys::shitty_vt_row_cells(self.raw, index, visit::<F>, &mut f as *mut F as *mut c_void);
        }
    }

    /// Moves the view through the scrollback: positive scrolls up into
    /// history, negative back toward the live bottom. Clamps to the retained
    /// history and is inert on the alternate screen. Returns the new offset.
    pub fn scroll(&mut self, rows: i32) -> u32 {
        unsafe { sys::shitty_vt_scroll(self.raw, rows) }
    }

    /// Places the view so `offset` rows of history sit above it; 0 is live.
    pub fn scroll_to(&mut self, offset: u32) -> u32 {
        unsafe { sys::shitty_vt_scroll_to(self.raw, offset) }
    }

    /// Rows of history above the live bottom the view currently shows.
    pub fn scroll_offset(&self) -> u32 {
        unsafe { sys::shitty_vt_scroll_offset(self.raw) }
    }

    /// Rows of scrollback retained; the largest offset [`Terminal::scroll_to`]
    /// will reach.
    pub fn history_rows(&self) -> u32 {
        unsafe { sys::shitty_vt_history_rows(self.raw) }
    }

    /// What the grid and history currently cost.
    pub fn memory_usage(&self) -> Memory {
        let mut raw = sys::shitty_vt_memory::default();
        // SAFETY: `raw` is a live, correctly typed output slot.
        unsafe { sys::shitty_vt_memory_usage(self.raw, &mut raw) };
        Memory {
            allocated_rows: raw.allocated_rows,
            capacity_rows: raw.capacity_rows,
            columns: raw.columns,
            cell_size: raw.cell_size,
            cell_bytes: raw.cell_bytes,
        }
    }

    /// Changes how many rows of scrollback the terminal keeps.
    ///
    /// Lowering it drops the oldest rows that no longer fit, at once rather
    /// than as the history is overwritten, and releases what they held.
    /// Raising it does not bring back rows already dropped. The visible grid
    /// is untouched either way.
    pub fn set_save_lines(&mut self, save_lines: u16) {
        unsafe { sys::shitty_vt_set_save_lines(self.raw, save_lines) }
    }

    /// The cursor.
    ///
    /// [`Cursor::row`] is a row of the *current view*, so while the view sits
    /// in the scrollback it can be at or past the last row — meaning the
    /// cursor is off screen and nothing should be drawn for it.
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

    /// Reports a physical key. Returns whether the terminal consumed it.
    ///
    /// The terminal does its own encoding: cursor and keypad modes,
    /// modifyOtherKeys and the kitty keyboard protocol are applied exactly as
    /// the application negotiated them, and the bytes land in
    /// [`Terminal::take_replies`] with everything else bound for the child.
    ///
    /// Deliver a keystroke the way a windowing layer does — the key, then the
    /// text it produced, then [`Terminal::input_flush`] to end the batch. The
    /// flush is not a formality: a key can be held back to learn whether text
    /// follows it (the kitty protocol's associated text), and is released
    /// either by that text or by the flush. [`Terminal::send_key`] does the
    /// three in order for callers with nothing to interleave.
    pub fn key(&mut self, event: KeyEvent) -> bool {
        let raw = event.to_raw();
        // SAFETY: `raw` outlives the call, which is all the facade asks.
        unsafe { sys::shitty_vt_key(self.raw, &raw) != 0 }
    }

    /// Reports the text a key produced, or that an input method committed,
    /// one character at a time. Text can arrive without a key event before it.
    pub fn text(&mut self, ch: char, modifiers: Modifiers) -> bool {
        unsafe { sys::shitty_vt_text(self.raw, ch as u32, modifiers.0) != 0 }
    }

    /// Ends the event batch, releasing a key that was waiting to see whether
    /// text followed it.
    pub fn input_flush(&mut self) {
        unsafe { sys::shitty_vt_input_flush(self.raw) }
    }

    /// A whole keystroke: the key, the text it produced if any, and the flush
    /// that ends the batch. Returns whether the key was consumed.
    ///
    /// ```
    /// use shitty_vt::{Key, KeyEvent, Modifiers, Terminal};
    ///
    /// let mut term = Terminal::new(80, 24, 1000);
    /// term.send_key(KeyEvent::printable('a'), Some('a'));
    /// term.send_key(KeyEvent::press(Key::Up), None);
    /// term.send_key(
    ///     KeyEvent::printable('c').with_modifiers(Modifiers::CONTROL),
    ///     None,
    /// );
    /// assert_eq!(term.take_replies(), b"a\x1b[A\x03");
    /// ```
    ///
    /// What the same three events encode to is the application's choice, not
    /// this caller's: after `\x1b[?1h` the arrow is `\x1bOA`, and after
    /// `\x1b[>1u` an Escape key is `\x1b[27u`.
    pub fn send_key(&mut self, event: KeyEvent, text: Option<char>) -> bool {
        let consumed = self.key(event);
        if let Some(ch) = text {
            self.text(ch, event.modifiers);
        }
        self.input_flush();
        consumed
    }

    /// A pointer button, at a cell 0-based from the top left.
    ///
    /// `time` is seconds on any monotonic clock and is what separates the
    /// clicks of a double or triple click. With no tracking mode capturing
    /// the pointer, an unshifted press and drag selects instead, and the
    /// finished selection arrives through
    /// [`Terminal::take_clipboard_writes`].
    pub fn mouse_button(
        &mut self,
        button: MouseButton,
        pressed: bool,
        column: i32,
        row: i32,
        modifiers: Modifiers,
        time: f64,
    ) -> bool {
        unsafe {
            sys::shitty_vt_mouse_button(
                self.raw,
                button as c_int,
                pressed as c_int,
                column,
                row,
                modifiers.0,
                time,
            ) != 0
        }
    }

    /// Pointer motion. During a drag the cell may be outside the grid, which
    /// is why the coordinates are signed.
    pub fn mouse_motion(&mut self, column: i32, row: i32, modifiers: Modifiers) -> bool {
        unsafe { sys::shitty_vt_mouse_motion(self.raw, column, row, modifiers.0) != 0 }
    }

    /// Wheel or trackpad scroll, in wheel lines, positive up and right;
    /// fractions accumulate across calls.
    ///
    /// Goes to the application when a mouse mode captures the wheel —
    /// including the alternate screen's wheel-to-arrows — and moves the view
    /// through the scrollback otherwise.
    pub fn mouse_scroll(
        &mut self,
        dx: f64,
        dy: f64,
        column: i32,
        row: i32,
        modifiers: Modifiers,
    ) -> bool {
        unsafe { sys::shitty_vt_mouse_scroll(self.raw, dx, dy, column, row, modifiers.0) != 0 }
    }

    /// Pastes through the terminal's own paste path: the payload is sanitized
    /// and, when the application turned bracketed paste on, wrapped in its
    /// markers. Capped at 16 MiB.
    pub fn paste(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        unsafe { sys::shitty_vt_paste(self.raw, bytes.as_ptr(), bytes.len()) }
    }

    /// Keyboard focus, reported to applications that asked for focus events.
    /// A fresh terminal is focused.
    pub fn set_focus(&mut self, focused: bool) {
        unsafe { sys::shitty_vt_focus(self.raw, focused as c_int) }
    }

    /// The most recent title the application set, or `None` while it has set
    /// none: the facade stays silent until the first one arrives.
    ///
    /// `Some("")` is therefore real activity rather than a starting state —
    /// a reset the application sends (RIS) publishes the cleared title.
    pub fn title(&self) -> Option<String> {
        // SAFETY: no call into C happens while this reference is alive.
        unsafe { &*self.events() }
            .title
            .as_ref()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    /// Number of bells since the terminal was created.
    pub fn bells(&self) -> u64 {
        // SAFETY: as in `title`.
        unsafe { (*self.events()).bells }
    }

    /// Whether the presentation moved since [`Terminal::clear_damage`].
    pub fn damaged(&self) -> bool {
        // SAFETY: as in `title`.
        unsafe { (*self.events()).damaged }
    }

    pub fn clear_damage(&mut self) {
        // SAFETY: as in `title`.
        unsafe { (*self.events()).damaged = false }
    }

    /// The grid size the application last asked for through XTWINOPS, if any.
    /// Answering it is the embedder's choice.
    pub fn take_resize_request(&mut self) -> Option<(u16, u16)> {
        // SAFETY: as in `title`.
        unsafe { (*self.events()).resize_request.take() }
    }

    /// OSC 52 selections the application set: `(0 primary | 1 clipboard, bytes)`.
    pub fn take_clipboard_writes(&mut self) -> Vec<(i32, Vec<u8>)> {
        // SAFETY: as in `title`.
        unsafe { std::mem::take(&mut (*self.events()).clipboard) }
    }

    /// Hyperlinks the application asked to open.
    pub fn take_open_uris(&mut self) -> Vec<Vec<u8>> {
        // SAFETY: as in `title`.
        unsafe { std::mem::take(&mut (*self.events()).open_uri) }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Order matters: the terminal holds both pointers, so it goes first
        // and the allocations it was pointing at go after.
        // SAFETY: each is owned by this `Terminal` and freed exactly once.
        unsafe {
            sys::shitty_vt_free(self.raw);
            drop(Box::from_raw(self.callbacks));
            drop(Box::from_raw(self.events));
        }
    }
}
