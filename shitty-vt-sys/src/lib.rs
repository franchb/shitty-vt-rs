//! Raw FFI declarations for `lib/embed/shitty_vt.h`.
//!
//! Hand-written rather than generated: the header is small and stable, and
//! bindgen would put libclang on every consumer's build.
//! The declarations below are a literal transcription — keep them that way.

#![allow(non_camel_case_types)]

use std::ffi::{c_double, c_int, c_void};

/// Opaque terminal instance.
#[repr(C)]
pub struct shitty_vt {
    _private: [u8; 0],
}

// shitty_vt_cell.attributes bits
pub const SHITTY_VT_ATTR_BOLD: u16 = 1 << 0;
pub const SHITTY_VT_ATTR_FAINT: u16 = 1 << 1;
pub const SHITTY_VT_ATTR_ITALIC: u16 = 1 << 2;
pub const SHITTY_VT_ATTR_BLINK: u16 = 1 << 3;
pub const SHITTY_VT_ATTR_INVERSE: u16 = 1 << 4;
pub const SHITTY_VT_ATTR_CONCEAL: u16 = 1 << 5;
pub const SHITTY_VT_ATTR_STRIKE: u16 = 1 << 6;
pub const SHITTY_VT_ATTR_OVERLINE: u16 = 1 << 7;

// shitty_vt_cell.*_source kinds, in the low byte of the field.
pub const SHITTY_VT_COLOR_DEFAULT_FOREGROUND: u16 = 0;
pub const SHITTY_VT_COLOR_DEFAULT_BACKGROUND: u16 = 1;
pub const SHITTY_VT_COLOR_INDEXED: u16 = 2;
pub const SHITTY_VT_COLOR_DIRECT: u16 = 3;

/// `SHITTY_VT_COLOR_KIND`, which the header spells as a macro.
pub const fn shitty_vt_color_kind(source: u16) -> u16 {
    source & 0xff
}

/// `SHITTY_VT_COLOR_INDEX`: the palette entry, valid only when the kind is
/// [`SHITTY_VT_COLOR_INDEXED`].
pub const fn shitty_vt_color_index(source: u16) -> u8 {
    ((source >> 8) & 0xff) as u8
}

// shitty_vt_modes() bits
pub const SHITTY_VT_MODE_ALT_SCREEN: u32 = 1 << 0;
pub const SHITTY_VT_MODE_BRACKETED_PASTE: u32 = 1 << 1;
pub const SHITTY_VT_MODE_APP_CURSOR_KEYS: u32 = 1 << 2;
pub const SHITTY_VT_MODE_APP_KEYPAD: u32 = 1 << 3;
pub const SHITTY_VT_MODE_FOCUS_EVENTS: u32 = 1 << 4;
pub const SHITTY_VT_MODE_AUTO_WRAP: u32 = 1 << 5;
pub const SHITTY_VT_MODE_ORIGIN: u32 = 1 << 6;
pub const SHITTY_VT_MODE_INSERT: u32 = 1 << 7;
pub const SHITTY_VT_MODE_CURSOR_VISIBLE: u32 = 1 << 8;
pub const SHITTY_VT_MODE_SCREEN_REVERSE: u32 = 1 << 9;
pub const SHITTY_VT_MODE_SYNCHRONIZED_OUTPUT: u32 = 1 << 10;
pub const SHITTY_VT_MODE_MOUSE_CLICK: u32 = 1 << 11;
pub const SHITTY_VT_MODE_MOUSE_DRAG: u32 = 1 << 12;
pub const SHITTY_VT_MODE_MOUSE_MOTION: u32 = 1 << 13;
pub const SHITTY_VT_MODE_MOUSE_SGR: u32 = 1 << 14;
pub const SHITTY_VT_MODE_ALTERNATE_SCROLL: u32 = 1 << 15;

// Key codes for `shitty_vt_key_event.key`. Pinned ABI: the facade
// static_asserts each against its input-layer value.
pub const SHITTY_VT_KEY_UNKNOWN: u16 = 0;
pub const SHITTY_VT_KEY_PRINTABLE: u16 = 1;
pub const SHITTY_VT_KEY_SPACE: u16 = 2;
pub const SHITTY_VT_KEY_ESCAPE: u16 = 3;
pub const SHITTY_VT_KEY_ENTER: u16 = 4;
pub const SHITTY_VT_KEY_BACKSPACE: u16 = 5;
pub const SHITTY_VT_KEY_TAB: u16 = 6;
pub const SHITTY_VT_KEY_INSERT: u16 = 7;
pub const SHITTY_VT_KEY_DELETE: u16 = 8;
pub const SHITTY_VT_KEY_HOME: u16 = 9;
pub const SHITTY_VT_KEY_END: u16 = 10;
pub const SHITTY_VT_KEY_UP: u16 = 11;
pub const SHITTY_VT_KEY_DOWN: u16 = 12;
pub const SHITTY_VT_KEY_LEFT: u16 = 13;
pub const SHITTY_VT_KEY_RIGHT: u16 = 14;
pub const SHITTY_VT_KEY_PAGE_UP: u16 = 15;
pub const SHITTY_VT_KEY_PAGE_DOWN: u16 = 16;
pub const SHITTY_VT_KEY_CLEAR: u16 = 17;
pub const SHITTY_VT_KEY_F1: u16 = 18;
pub const SHITTY_VT_KEY_F2: u16 = 19;
pub const SHITTY_VT_KEY_F3: u16 = 20;
pub const SHITTY_VT_KEY_F4: u16 = 21;
pub const SHITTY_VT_KEY_F5: u16 = 22;
pub const SHITTY_VT_KEY_F6: u16 = 23;
pub const SHITTY_VT_KEY_F7: u16 = 24;
pub const SHITTY_VT_KEY_F8: u16 = 25;
pub const SHITTY_VT_KEY_F9: u16 = 26;
pub const SHITTY_VT_KEY_F10: u16 = 27;
pub const SHITTY_VT_KEY_F11: u16 = 28;
pub const SHITTY_VT_KEY_F12: u16 = 29;
pub const SHITTY_VT_KEY_F13: u16 = 30;
pub const SHITTY_VT_KEY_F14: u16 = 31;
pub const SHITTY_VT_KEY_F15: u16 = 32;
pub const SHITTY_VT_KEY_F16: u16 = 33;
pub const SHITTY_VT_KEY_F17: u16 = 34;
pub const SHITTY_VT_KEY_F18: u16 = 35;
pub const SHITTY_VT_KEY_F19: u16 = 36;
pub const SHITTY_VT_KEY_F20: u16 = 37;
pub const SHITTY_VT_KEY_F21: u16 = 38;
pub const SHITTY_VT_KEY_F22: u16 = 39;
pub const SHITTY_VT_KEY_F23: u16 = 40;
pub const SHITTY_VT_KEY_F24: u16 = 41;
pub const SHITTY_VT_KEY_F25: u16 = 42;
pub const SHITTY_VT_KEY_F26: u16 = 43;
pub const SHITTY_VT_KEY_F27: u16 = 44;
pub const SHITTY_VT_KEY_F28: u16 = 45;
pub const SHITTY_VT_KEY_F29: u16 = 46;
pub const SHITTY_VT_KEY_F30: u16 = 47;
pub const SHITTY_VT_KEY_F31: u16 = 48;
pub const SHITTY_VT_KEY_F32: u16 = 49;
pub const SHITTY_VT_KEY_F33: u16 = 50;
pub const SHITTY_VT_KEY_F34: u16 = 51;
pub const SHITTY_VT_KEY_F35: u16 = 52;
pub const SHITTY_VT_KEY_KEYPAD_0: u16 = 53;
pub const SHITTY_VT_KEY_KEYPAD_1: u16 = 54;
pub const SHITTY_VT_KEY_KEYPAD_2: u16 = 55;
pub const SHITTY_VT_KEY_KEYPAD_3: u16 = 56;
pub const SHITTY_VT_KEY_KEYPAD_4: u16 = 57;
pub const SHITTY_VT_KEY_KEYPAD_5: u16 = 58;
pub const SHITTY_VT_KEY_KEYPAD_6: u16 = 59;
pub const SHITTY_VT_KEY_KEYPAD_7: u16 = 60;
pub const SHITTY_VT_KEY_KEYPAD_8: u16 = 61;
pub const SHITTY_VT_KEY_KEYPAD_9: u16 = 62;
pub const SHITTY_VT_KEY_KEYPAD_DECIMAL: u16 = 63;
pub const SHITTY_VT_KEY_KEYPAD_DIVIDE: u16 = 64;
pub const SHITTY_VT_KEY_KEYPAD_MULTIPLY: u16 = 65;
pub const SHITTY_VT_KEY_KEYPAD_SUBTRACT: u16 = 66;
pub const SHITTY_VT_KEY_KEYPAD_ADD: u16 = 67;
pub const SHITTY_VT_KEY_KEYPAD_ENTER: u16 = 68;
pub const SHITTY_VT_KEY_KEYPAD_EQUAL: u16 = 69;
pub const SHITTY_VT_KEY_KEYPAD_SEPARATOR: u16 = 70;
pub const SHITTY_VT_KEY_KEYPAD_F1: u16 = 71;
pub const SHITTY_VT_KEY_KEYPAD_F2: u16 = 72;
pub const SHITTY_VT_KEY_KEYPAD_F3: u16 = 73;
pub const SHITTY_VT_KEY_KEYPAD_F4: u16 = 74;
pub const SHITTY_VT_KEY_KEYPAD_INSERT: u16 = 75;
pub const SHITTY_VT_KEY_KEYPAD_DELETE: u16 = 76;
pub const SHITTY_VT_KEY_KEYPAD_UP: u16 = 77;
pub const SHITTY_VT_KEY_KEYPAD_DOWN: u16 = 78;
pub const SHITTY_VT_KEY_KEYPAD_LEFT: u16 = 79;
pub const SHITTY_VT_KEY_KEYPAD_RIGHT: u16 = 80;
pub const SHITTY_VT_KEY_KEYPAD_HOME: u16 = 81;
pub const SHITTY_VT_KEY_KEYPAD_END: u16 = 82;
pub const SHITTY_VT_KEY_KEYPAD_PAGE_UP: u16 = 83;
pub const SHITTY_VT_KEY_KEYPAD_PAGE_DOWN: u16 = 84;
pub const SHITTY_VT_KEY_KEYPAD_BEGIN: u16 = 85;
pub const SHITTY_VT_KEY_KEYPAD_SPACE: u16 = 86;
pub const SHITTY_VT_KEY_KEYPAD_TAB: u16 = 87;
pub const SHITTY_VT_KEY_CAPS_LOCK: u16 = 88;
pub const SHITTY_VT_KEY_SCROLL_LOCK: u16 = 89;
pub const SHITTY_VT_KEY_NUM_LOCK: u16 = 90;
pub const SHITTY_VT_KEY_PRINT_SCREEN: u16 = 91;
pub const SHITTY_VT_KEY_PAUSE: u16 = 92;
pub const SHITTY_VT_KEY_MENU: u16 = 93;
pub const SHITTY_VT_KEY_LEFT_SHIFT: u16 = 94;
pub const SHITTY_VT_KEY_LEFT_CONTROL: u16 = 95;
pub const SHITTY_VT_KEY_LEFT_ALT: u16 = 96;
pub const SHITTY_VT_KEY_LEFT_SUPER: u16 = 97;
pub const SHITTY_VT_KEY_RIGHT_SHIFT: u16 = 98;
pub const SHITTY_VT_KEY_RIGHT_CONTROL: u16 = 99;
pub const SHITTY_VT_KEY_RIGHT_ALT: u16 = 100;
pub const SHITTY_VT_KEY_RIGHT_SUPER: u16 = 101;
pub const SHITTY_VT_KEY_MEDIA_PLAY: u16 = 102;
pub const SHITTY_VT_KEY_MEDIA_PAUSE: u16 = 103;
pub const SHITTY_VT_KEY_MEDIA_PLAY_PAUSE: u16 = 104;
pub const SHITTY_VT_KEY_MEDIA_REVERSE: u16 = 105;
pub const SHITTY_VT_KEY_MEDIA_STOP: u16 = 106;
pub const SHITTY_VT_KEY_MEDIA_FAST_FORWARD: u16 = 107;
pub const SHITTY_VT_KEY_MEDIA_REWIND: u16 = 108;
pub const SHITTY_VT_KEY_MEDIA_TRACK_NEXT: u16 = 109;
pub const SHITTY_VT_KEY_MEDIA_TRACK_PREVIOUS: u16 = 110;
pub const SHITTY_VT_KEY_MEDIA_RECORD: u16 = 111;
pub const SHITTY_VT_KEY_VOLUME_DOWN: u16 = 112;
pub const SHITTY_VT_KEY_VOLUME_UP: u16 = 113;
pub const SHITTY_VT_KEY_VOLUME_MUTE: u16 = 114;
/// One past the last key code.
pub const SHITTY_VT_KEY_COUNT: u16 = 115;

// shitty_vt_key_event.modifiers bits
pub const SHITTY_VT_MOD_SHIFT: u16 = 1 << 0;
pub const SHITTY_VT_MOD_CONTROL: u16 = 1 << 1;
pub const SHITTY_VT_MOD_ALT: u16 = 1 << 2;
pub const SHITTY_VT_MOD_SUPER: u16 = 1 << 3;
pub const SHITTY_VT_MOD_CAPS_LOCK: u16 = 1 << 4;
pub const SHITTY_VT_MOD_NUM_LOCK: u16 = 1 << 5;
pub const SHITTY_VT_MOD_ALT_GRAPH: u16 = 1 << 6;

// shitty_vt_key_event.action values
pub const SHITTY_VT_KEY_PRESS: u8 = 0;
pub const SHITTY_VT_KEY_REPEAT: u8 = 1;
pub const SHITTY_VT_KEY_RELEASE: u8 = 2;

// Mouse buttons
pub const SHITTY_VT_MOUSE_LEFT: c_int = 0;
pub const SHITTY_VT_MOUSE_RIGHT: c_int = 1;
pub const SHITTY_VT_MOUSE_MIDDLE: c_int = 2;
pub const SHITTY_VT_MOUSE_AUX1: c_int = 3;
pub const SHITTY_VT_MOUSE_AUX2: c_int = 4;
pub const SHITTY_VT_MOUSE_AUX3: c_int = 5;
pub const SHITTY_VT_MOUSE_AUX4: c_int = 6;
pub const SHITTY_VT_MOUSE_AUX5: c_int = 7;

/// One readable cell. Colours are `0x00BBGGRR`. `grapheme` is valid only for
/// the duration of the [`shitty_vt_cell_fn`] callback.
///
/// The `*_source` fields say where each resolved colour came from, so an
/// embedder with a palette of its own can honour the request rather than the
/// value this terminal would have painted. Read them with
/// [`shitty_vt_color_kind`] and [`shitty_vt_color_index`].
#[repr(C)]
pub struct shitty_vt_cell {
    pub grapheme: *const u32,
    pub grapheme_len: usize,
    pub foreground: u32,
    pub background: u32,
    pub underline_color: u32,
    pub attributes: u16,
    /// 0 none, 1 straight, 2 double, 3 curly, 4 dotted, 5 dashed
    pub underline_style: u8,
    /// 1 or 2; the continuation of a wide cell is not reported
    pub width: u8,
    pub foreground_source: u16,
    pub background_source: u16,
    pub underline_source: u16,
}

/// What the terminal spends on its grid and history. Cells only: the
/// grapheme, hyperlink and sixel stores are not counted.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct shitty_vt_memory {
    /// Row slots actually backed by cells. The ring is rounded to a power of
    /// two, so this can exceed `capacity_rows`.
    pub allocated_rows: u32,
    /// Rows the terminal will keep: rows + save_lines.
    pub capacity_rows: u32,
    pub columns: u32,
    pub cell_size: u32,
    pub cell_bytes: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct shitty_vt_cursor {
    pub column: u16,
    pub row: u16,
    /// 0 hidden, 1 filled block, 2 hollow block, 3 underline, 4 bar
    pub style: u8,
    pub visible: u8,
}

/// One physical key event. Zero codepoints mean unknown; a named key needs
/// none of them.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct shitty_vt_key_event {
    /// A `SHITTY_VT_KEY_*` code.
    pub key: u16,
    /// `SHITTY_VT_KEY_PRESS`, `_REPEAT` or `_RELEASE`.
    pub action: u8,
    /// `SHITTY_VT_MOD_*` bits.
    pub modifiers: u16,
    /// The key's identity in the active layout unshifted,
    pub layout_codepoint: u32,
    /// in the base (ASCII) layout,
    pub base_codepoint: u32,
    /// and with Shift in the active layout.
    pub shifted_codepoint: u32,
}

pub type shitty_vt_cell_fn =
    unsafe extern "C" fn(user: *mut c_void, row: u16, column: u16, cell: *const shitty_vt_cell);

/// Everything the terminal may want from its embedder. Every field may be
/// null. The terminal keeps this pointer rather than copying, so the struct
/// must outlive the terminal.
#[repr(C)]
pub struct shitty_vt_callbacks {
    pub user: *mut c_void,
    pub title_changed: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize)>,
    pub bell: Option<unsafe extern "C" fn(*mut c_void)>,
    pub damaged: Option<unsafe extern "C" fn(*mut c_void)>,
    pub open_uri: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize)>,
    pub clipboard_set: Option<unsafe extern "C" fn(*mut c_void, c_int, *const u8, usize)>,
    pub resize_request: Option<unsafe extern "C" fn(*mut c_void, u16, u16)>,
}

extern "C" {
    pub fn shitty_vt_new(
        columns: u16,
        rows: u16,
        save_lines: u16,
        callbacks: *const shitty_vt_callbacks,
    ) -> *mut shitty_vt;
    pub fn shitty_vt_free(vt: *mut shitty_vt);
    pub fn shitty_vt_feed(vt: *mut shitty_vt, bytes: *const u8, len: usize);
    pub fn shitty_vt_resize(vt: *mut shitty_vt, columns: u16, rows: u16);
    pub fn shitty_vt_take_replies(vt: *mut shitty_vt, out: *mut u8, cap: usize) -> usize;
    pub fn shitty_vt_each_cell(vt: *mut shitty_vt, f: shitty_vt_cell_fn, user: *mut c_void);
    pub fn shitty_vt_scroll(vt: *mut shitty_vt, rows: i32) -> u32;
    pub fn shitty_vt_scroll_to(vt: *mut shitty_vt, offset: u32) -> u32;
    pub fn shitty_vt_scroll_offset(vt: *const shitty_vt) -> u32;
    pub fn shitty_vt_history_rows(vt: *const shitty_vt) -> u32;
    pub fn shitty_vt_memory_usage(vt: *const shitty_vt, out: *mut shitty_vt_memory);
    pub fn shitty_vt_set_save_lines(vt: *mut shitty_vt, save_lines: u16);
    pub fn shitty_vt_total_rows(vt: *const shitty_vt) -> u32;
    pub fn shitty_vt_row_cells(
        vt: *mut shitty_vt,
        index: u32,
        f: shitty_vt_cell_fn,
        user: *mut c_void,
    );
    pub fn shitty_vt_cursor_state(vt: *const shitty_vt) -> shitty_vt_cursor;
    pub fn shitty_vt_modes(vt: *const shitty_vt) -> u32;
    pub fn shitty_vt_key(vt: *mut shitty_vt, event: *const shitty_vt_key_event) -> c_int;
    pub fn shitty_vt_text(vt: *mut shitty_vt, codepoint: u32, modifiers: u16) -> c_int;
    pub fn shitty_vt_input_flush(vt: *mut shitty_vt);
    pub fn shitty_vt_mouse_button(
        vt: *mut shitty_vt,
        button: c_int,
        pressed: c_int,
        column: i32,
        row: i32,
        modifiers: u16,
        time: c_double,
    ) -> c_int;
    pub fn shitty_vt_mouse_motion(
        vt: *mut shitty_vt,
        column: i32,
        row: i32,
        modifiers: u16,
    ) -> c_int;
    pub fn shitty_vt_mouse_scroll(
        vt: *mut shitty_vt,
        dx: c_double,
        dy: c_double,
        column: i32,
        row: i32,
        modifiers: u16,
    ) -> c_int;
    pub fn shitty_vt_paste(vt: *mut shitty_vt, bytes: *const u8, len: usize);
    pub fn shitty_vt_focus(vt: *mut shitty_vt, focused: c_int);
    pub fn shitty_vt_preedit(
        vt: *mut shitty_vt,
        text: *const u8,
        len: usize,
        cursor_begin: i32,
        cursor_end: i32,
    );
    pub fn shitty_vt_preedit_cells(vt: *mut shitty_vt, f: shitty_vt_cell_fn, user: *mut c_void);
}
