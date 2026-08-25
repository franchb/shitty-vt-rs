//! Raw FFI declarations for `lib/embed/shitty_vt.h`.
//!
//! Hand-written rather than generated: the header is under 120 lines and
//! stable, and bindgen would put libclang on every consumer's build.
//! The declarations below are a literal transcription — keep them that way.

#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_void};

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

/// One readable cell. Colours are `0x00BBGGRR`. `grapheme` is valid only for
/// the duration of the [`shitty_vt_cell_fn`] callback.
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
    pub fn shitty_vt_cursor_state(vt: *const shitty_vt) -> shitty_vt_cursor;
    pub fn shitty_vt_modes(vt: *const shitty_vt) -> u32;
}
