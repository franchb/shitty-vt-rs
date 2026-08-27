//! Input encoding: events go in, and the terminal encodes them by whatever
//! protocol the stream negotiated. Every expectation here is a byte sequence
//! the terminal produced, not one the bindings built.

use shitty_vt::{Key, KeyAction, KeyEvent, Modifiers, MouseButton, Terminal};

/// A terminal that has already read `stream`, with its replies drained so
/// only what the input produces is left.
fn negotiated(stream: &[u8]) -> Terminal {
    let mut term = Terminal::new(80, 24, 100);
    term.feed(stream);
    term.take_replies();
    term
}

#[test]
fn an_arrow_key_follows_the_cursor_mode() {
    let mut term = negotiated(b"");
    term.send_key(KeyEvent::press(Key::Up), None);
    assert_eq!(term.take_replies(), b"\x1b[A");

    let mut term = negotiated(b"\x1b[?1h");
    term.send_key(KeyEvent::press(Key::Up), None);
    assert_eq!(term.take_replies(), b"\x1bOA");
}

#[test]
fn text_reaches_the_child_as_utf8() {
    let mut term = negotiated(b"");
    term.send_key(KeyEvent::printable('A'), Some('A'));
    assert_eq!(term.take_replies(), b"A");

    term.text('\u{442}', Modifiers::NONE);
    term.input_flush();
    assert_eq!(term.take_replies(), "\u{442}".as_bytes());
}

#[test]
fn a_control_chord_is_encoded_from_the_key_event() {
    // The chord is built from the codepoints on the event, so the terminal
    // needs no separate text event to know it was Ctrl+C.
    let mut term = negotiated(b"");
    term.send_key(
        KeyEvent::printable('c').with_modifiers(Modifiers::CONTROL),
        None,
    );
    assert_eq!(term.take_replies(), b"\x03");

    // And a fuller event, with the shifted identity a real layout would
    // carry, encodes the same.
    term.send_key(
        KeyEvent {
            modifiers: Modifiers::CONTROL,
            shifted: Some('C'),
            ..KeyEvent::printable('c')
        },
        None,
    );
    assert_eq!(term.take_replies(), b"\x03");
}

#[test]
fn the_kitty_protocol_changes_what_escape_sends() {
    let mut term = negotiated(b"");
    term.send_key(KeyEvent::press(Key::Escape), None);
    assert_eq!(term.take_replies(), b"\x1b");

    let mut term = negotiated(b"\x1b[>1u");
    term.send_key(KeyEvent::press(Key::Escape), None);
    assert_eq!(term.take_replies(), b"\x1b[27u");
}

#[test]
fn a_release_is_reported_only_where_it_is_wanted() {
    let mut term = negotiated(b"");
    term.send_key(KeyEvent::release(Key::Escape), None);
    assert_eq!(term.take_replies(), b"");

    let mut term = negotiated(b"\x1b[>3u");
    term.send_key(KeyEvent::press(Key::Escape), None);
    term.send_key(KeyEvent::release(Key::Escape), None);
    assert_eq!(term.take_replies(), b"\x1b[27u\x1b[27;1:3u");
}

#[test]
fn a_paste_is_bracketed_when_the_application_asked() {
    let mut term = negotiated(b"");
    term.paste(b"hi");
    assert_eq!(term.take_replies(), b"hi");

    let mut term = negotiated(b"\x1b[?2004h");
    term.paste(b"hi");
    assert_eq!(term.take_replies(), b"\x1b[200~hi\x1b[201~");
}

#[test]
fn sgr_mouse_reports_press_and_release() {
    let mut term = negotiated(b"\x1b[?1000h\x1b[?1006h");
    term.mouse_button(MouseButton::Left, true, 4, 2, Modifiers::NONE, 1.0);
    term.mouse_button(MouseButton::Left, false, 4, 2, Modifiers::NONE, 1.1);
    // The protocol counts from one; the facade takes cells from zero.
    assert_eq!(term.take_replies(), b"\x1b[<0;5;3M\x1b[<0;5;3m");
}

#[test]
fn motion_reports_under_any_event_tracking() {
    let mut term = negotiated(b"\x1b[?1003h\x1b[?1006h");
    term.mouse_motion(4, 2, Modifiers::NONE);
    assert_eq!(term.take_replies(), b"\x1b[<35;5;3M");
}

#[test]
fn the_wheel_goes_to_the_application_that_captured_it() {
    let mut term = negotiated(b"\x1b[?1000h\x1b[?1006h");
    term.mouse_scroll(0.0, 1.0, 4, 2, Modifiers::NONE);
    assert_eq!(term.take_replies(), b"\x1b[<64;5;3M");
}

#[test]
fn the_wheel_moves_the_view_when_nothing_captured_it() {
    let mut term = Terminal::new(80, 24, 100);
    for line in 0..40 {
        term.feed(format!("line{line}\r\n").as_bytes());
    }
    term.take_replies();

    term.mouse_scroll(0.0, 1.0, 4, 2, Modifiers::NONE);
    assert!(term.scroll_offset() > 0);
    assert_eq!(term.take_replies(), b"");
}

#[test]
fn focus_is_reported_only_where_it_is_wanted() {
    let mut term = negotiated(b"");
    term.set_focus(false);
    term.set_focus(true);
    assert_eq!(term.take_replies(), b"");

    let mut term = negotiated(b"\x1b[?1004h");
    term.set_focus(false);
    term.set_focus(true);
    assert_eq!(term.take_replies(), b"\x1b[O\x1b[I");
}

#[test]
fn an_unshifted_drag_selects_and_publishes_the_selection() {
    // With no tracking mode capturing the pointer, a drag is a selection;
    // the finished one arrives the way an OSC 52 write would.
    let mut term = negotiated(b"grab me");
    term.mouse_button(MouseButton::Left, true, 0, 0, Modifiers::NONE, 1.0);
    term.mouse_motion(4, 0, Modifiers::NONE);
    term.mouse_button(MouseButton::Left, false, 4, 0, Modifiers::NONE, 1.2);

    let writes = term.take_clipboard_writes();
    assert_eq!(writes.len(), 1, "expected one selection, got {writes:?}");
    assert_eq!(writes[0].0, 0, "the primary selection");
    assert_eq!(String::from_utf8_lossy(&writes[0].1), "grab");
    assert_eq!(term.take_replies(), b"");
}

#[test]
fn a_character_key_sends_nothing_without_its_text() {
    // Under the legacy encoding the key event carries the chord and the text
    // event carries the character, so a plain letter reaches the child only
    // when both have been reported.
    let mut term = negotiated(b"");
    term.key(KeyEvent::printable('a'));
    term.input_flush();
    assert_eq!(term.take_replies(), b"");

    term.key(KeyEvent::printable('a'));
    term.text('a', Modifiers::NONE);
    term.input_flush();
    assert_eq!(term.take_replies(), b"a");
}

#[test]
fn the_flush_is_what_releases_a_held_key() {
    // Kitty flag 8 asks for every key as an escape code, and flag 16 for the
    // text it stands for - so the key cannot be encoded until the batch says
    // whether text follows it.
    let mut term = negotiated(b"\x1b[>9u");
    term.key(KeyEvent::printable('a'));
    assert_eq!(term.take_replies(), b"", "held until the batch ends");
    term.input_flush();
    assert_eq!(term.take_replies(), b"\x1b[97u");

    let mut term = negotiated(b"\x1b[>31u");
    term.send_key(KeyEvent::printable('a'), Some('a'));
    assert_eq!(term.take_replies(), b"\x1b[97;;97u");
}

#[test]
fn a_key_with_nothing_to_say_says_nothing() {
    let mut term = negotiated(b"");
    for event in [
        KeyEvent::press(Key::Unknown),
        KeyEvent::press(Key::LeftShift),
        // A release, which only the kitty protocol reports.
        KeyEvent::release(Key::Up),
    ] {
        term.send_key(event, None);
        assert_eq!(term.take_replies(), b"", "{event:?} should send nothing");
    }

    term.send_key(KeyEvent::press(Key::F5), None);
    assert_eq!(term.take_replies(), b"\x1b[15~");
}

#[test]
fn key_codes_match_the_facade() {
    // The C side static_asserts its codes against the input layer; this is
    // the same check one link further out, so a header that renumbers cannot
    // silently turn one key into another. Every code is here rather than a
    // sample of them, which is also what makes `Key::from_code` sound: 115
    // distinct constants below 115 leave no code in range without a variant.
    assert_eq!(Key::COUNT, shitty_vt_sys::SHITTY_VT_KEY_COUNT);
    for code in 0..Key::COUNT {
        let key = Key::from_code(code).expect("every code below COUNT names a key");
        assert_eq!(key.code(), code);
    }
    assert_eq!(Key::from_code(Key::COUNT), None);

    assert_eq!(Key::Unknown.code(), shitty_vt_sys::SHITTY_VT_KEY_UNKNOWN);
    assert_eq!(
        Key::Printable.code(),
        shitty_vt_sys::SHITTY_VT_KEY_PRINTABLE
    );
    assert_eq!(Key::Space.code(), shitty_vt_sys::SHITTY_VT_KEY_SPACE);
    assert_eq!(Key::Escape.code(), shitty_vt_sys::SHITTY_VT_KEY_ESCAPE);
    assert_eq!(Key::Enter.code(), shitty_vt_sys::SHITTY_VT_KEY_ENTER);
    assert_eq!(
        Key::Backspace.code(),
        shitty_vt_sys::SHITTY_VT_KEY_BACKSPACE
    );
    assert_eq!(Key::Tab.code(), shitty_vt_sys::SHITTY_VT_KEY_TAB);
    assert_eq!(Key::Insert.code(), shitty_vt_sys::SHITTY_VT_KEY_INSERT);
    assert_eq!(Key::Delete.code(), shitty_vt_sys::SHITTY_VT_KEY_DELETE);
    assert_eq!(Key::Home.code(), shitty_vt_sys::SHITTY_VT_KEY_HOME);
    assert_eq!(Key::End.code(), shitty_vt_sys::SHITTY_VT_KEY_END);
    assert_eq!(Key::Up.code(), shitty_vt_sys::SHITTY_VT_KEY_UP);
    assert_eq!(Key::Down.code(), shitty_vt_sys::SHITTY_VT_KEY_DOWN);
    assert_eq!(Key::Left.code(), shitty_vt_sys::SHITTY_VT_KEY_LEFT);
    assert_eq!(Key::Right.code(), shitty_vt_sys::SHITTY_VT_KEY_RIGHT);
    assert_eq!(Key::PageUp.code(), shitty_vt_sys::SHITTY_VT_KEY_PAGE_UP);
    assert_eq!(Key::PageDown.code(), shitty_vt_sys::SHITTY_VT_KEY_PAGE_DOWN);
    assert_eq!(Key::Clear.code(), shitty_vt_sys::SHITTY_VT_KEY_CLEAR);
    assert_eq!(Key::F1.code(), shitty_vt_sys::SHITTY_VT_KEY_F1);
    assert_eq!(Key::F2.code(), shitty_vt_sys::SHITTY_VT_KEY_F2);
    assert_eq!(Key::F3.code(), shitty_vt_sys::SHITTY_VT_KEY_F3);
    assert_eq!(Key::F4.code(), shitty_vt_sys::SHITTY_VT_KEY_F4);
    assert_eq!(Key::F5.code(), shitty_vt_sys::SHITTY_VT_KEY_F5);
    assert_eq!(Key::F6.code(), shitty_vt_sys::SHITTY_VT_KEY_F6);
    assert_eq!(Key::F7.code(), shitty_vt_sys::SHITTY_VT_KEY_F7);
    assert_eq!(Key::F8.code(), shitty_vt_sys::SHITTY_VT_KEY_F8);
    assert_eq!(Key::F9.code(), shitty_vt_sys::SHITTY_VT_KEY_F9);
    assert_eq!(Key::F10.code(), shitty_vt_sys::SHITTY_VT_KEY_F10);
    assert_eq!(Key::F11.code(), shitty_vt_sys::SHITTY_VT_KEY_F11);
    assert_eq!(Key::F12.code(), shitty_vt_sys::SHITTY_VT_KEY_F12);
    assert_eq!(Key::F13.code(), shitty_vt_sys::SHITTY_VT_KEY_F13);
    assert_eq!(Key::F14.code(), shitty_vt_sys::SHITTY_VT_KEY_F14);
    assert_eq!(Key::F15.code(), shitty_vt_sys::SHITTY_VT_KEY_F15);
    assert_eq!(Key::F16.code(), shitty_vt_sys::SHITTY_VT_KEY_F16);
    assert_eq!(Key::F17.code(), shitty_vt_sys::SHITTY_VT_KEY_F17);
    assert_eq!(Key::F18.code(), shitty_vt_sys::SHITTY_VT_KEY_F18);
    assert_eq!(Key::F19.code(), shitty_vt_sys::SHITTY_VT_KEY_F19);
    assert_eq!(Key::F20.code(), shitty_vt_sys::SHITTY_VT_KEY_F20);
    assert_eq!(Key::F21.code(), shitty_vt_sys::SHITTY_VT_KEY_F21);
    assert_eq!(Key::F22.code(), shitty_vt_sys::SHITTY_VT_KEY_F22);
    assert_eq!(Key::F23.code(), shitty_vt_sys::SHITTY_VT_KEY_F23);
    assert_eq!(Key::F24.code(), shitty_vt_sys::SHITTY_VT_KEY_F24);
    assert_eq!(Key::F25.code(), shitty_vt_sys::SHITTY_VT_KEY_F25);
    assert_eq!(Key::F26.code(), shitty_vt_sys::SHITTY_VT_KEY_F26);
    assert_eq!(Key::F27.code(), shitty_vt_sys::SHITTY_VT_KEY_F27);
    assert_eq!(Key::F28.code(), shitty_vt_sys::SHITTY_VT_KEY_F28);
    assert_eq!(Key::F29.code(), shitty_vt_sys::SHITTY_VT_KEY_F29);
    assert_eq!(Key::F30.code(), shitty_vt_sys::SHITTY_VT_KEY_F30);
    assert_eq!(Key::F31.code(), shitty_vt_sys::SHITTY_VT_KEY_F31);
    assert_eq!(Key::F32.code(), shitty_vt_sys::SHITTY_VT_KEY_F32);
    assert_eq!(Key::F33.code(), shitty_vt_sys::SHITTY_VT_KEY_F33);
    assert_eq!(Key::F34.code(), shitty_vt_sys::SHITTY_VT_KEY_F34);
    assert_eq!(Key::F35.code(), shitty_vt_sys::SHITTY_VT_KEY_F35);
    assert_eq!(Key::Keypad0.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_0);
    assert_eq!(Key::Keypad1.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_1);
    assert_eq!(Key::Keypad2.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_2);
    assert_eq!(Key::Keypad3.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_3);
    assert_eq!(Key::Keypad4.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_4);
    assert_eq!(Key::Keypad5.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_5);
    assert_eq!(Key::Keypad6.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_6);
    assert_eq!(Key::Keypad7.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_7);
    assert_eq!(Key::Keypad8.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_8);
    assert_eq!(Key::Keypad9.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_9);
    assert_eq!(
        Key::KeypadDecimal.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_DECIMAL
    );
    assert_eq!(
        Key::KeypadDivide.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_DIVIDE
    );
    assert_eq!(
        Key::KeypadMultiply.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_MULTIPLY
    );
    assert_eq!(
        Key::KeypadSubtract.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_SUBTRACT
    );
    assert_eq!(
        Key::KeypadAdd.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_ADD
    );
    assert_eq!(
        Key::KeypadEnter.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_ENTER
    );
    assert_eq!(
        Key::KeypadEqual.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_EQUAL
    );
    assert_eq!(
        Key::KeypadSeparator.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_SEPARATOR
    );
    assert_eq!(Key::KeypadF1.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_F1);
    assert_eq!(Key::KeypadF2.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_F2);
    assert_eq!(Key::KeypadF3.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_F3);
    assert_eq!(Key::KeypadF4.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_F4);
    assert_eq!(
        Key::KeypadInsert.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_INSERT
    );
    assert_eq!(
        Key::KeypadDelete.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_DELETE
    );
    assert_eq!(Key::KeypadUp.code(), shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_UP);
    assert_eq!(
        Key::KeypadDown.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_DOWN
    );
    assert_eq!(
        Key::KeypadLeft.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_LEFT
    );
    assert_eq!(
        Key::KeypadRight.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_RIGHT
    );
    assert_eq!(
        Key::KeypadHome.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_HOME
    );
    assert_eq!(
        Key::KeypadEnd.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_END
    );
    assert_eq!(
        Key::KeypadPageUp.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_PAGE_UP
    );
    assert_eq!(
        Key::KeypadPageDown.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_PAGE_DOWN
    );
    assert_eq!(
        Key::KeypadBegin.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_BEGIN
    );
    assert_eq!(
        Key::KeypadSpace.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_SPACE
    );
    assert_eq!(
        Key::KeypadTab.code(),
        shitty_vt_sys::SHITTY_VT_KEY_KEYPAD_TAB
    );
    assert_eq!(Key::CapsLock.code(), shitty_vt_sys::SHITTY_VT_KEY_CAPS_LOCK);
    assert_eq!(
        Key::ScrollLock.code(),
        shitty_vt_sys::SHITTY_VT_KEY_SCROLL_LOCK
    );
    assert_eq!(Key::NumLock.code(), shitty_vt_sys::SHITTY_VT_KEY_NUM_LOCK);
    assert_eq!(
        Key::PrintScreen.code(),
        shitty_vt_sys::SHITTY_VT_KEY_PRINT_SCREEN
    );
    assert_eq!(Key::Pause.code(), shitty_vt_sys::SHITTY_VT_KEY_PAUSE);
    assert_eq!(Key::Menu.code(), shitty_vt_sys::SHITTY_VT_KEY_MENU);
    assert_eq!(
        Key::LeftShift.code(),
        shitty_vt_sys::SHITTY_VT_KEY_LEFT_SHIFT
    );
    assert_eq!(
        Key::LeftControl.code(),
        shitty_vt_sys::SHITTY_VT_KEY_LEFT_CONTROL
    );
    assert_eq!(Key::LeftAlt.code(), shitty_vt_sys::SHITTY_VT_KEY_LEFT_ALT);
    assert_eq!(
        Key::LeftSuper.code(),
        shitty_vt_sys::SHITTY_VT_KEY_LEFT_SUPER
    );
    assert_eq!(
        Key::RightShift.code(),
        shitty_vt_sys::SHITTY_VT_KEY_RIGHT_SHIFT
    );
    assert_eq!(
        Key::RightControl.code(),
        shitty_vt_sys::SHITTY_VT_KEY_RIGHT_CONTROL
    );
    assert_eq!(Key::RightAlt.code(), shitty_vt_sys::SHITTY_VT_KEY_RIGHT_ALT);
    assert_eq!(
        Key::RightSuper.code(),
        shitty_vt_sys::SHITTY_VT_KEY_RIGHT_SUPER
    );
    assert_eq!(
        Key::MediaPlay.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_PLAY
    );
    assert_eq!(
        Key::MediaPause.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_PAUSE
    );
    assert_eq!(
        Key::MediaPlayPause.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_PLAY_PAUSE
    );
    assert_eq!(
        Key::MediaReverse.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_REVERSE
    );
    assert_eq!(
        Key::MediaStop.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_STOP
    );
    assert_eq!(
        Key::MediaFastForward.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_FAST_FORWARD
    );
    assert_eq!(
        Key::MediaRewind.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_REWIND
    );
    assert_eq!(
        Key::MediaTrackNext.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_TRACK_NEXT
    );
    assert_eq!(
        Key::MediaTrackPrevious.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_TRACK_PREVIOUS
    );
    assert_eq!(
        Key::MediaRecord.code(),
        shitty_vt_sys::SHITTY_VT_KEY_MEDIA_RECORD
    );
    assert_eq!(
        Key::VolumeDown.code(),
        shitty_vt_sys::SHITTY_VT_KEY_VOLUME_DOWN
    );
    assert_eq!(Key::VolumeUp.code(), shitty_vt_sys::SHITTY_VT_KEY_VOLUME_UP);
    assert_eq!(
        Key::VolumeMute.code(),
        shitty_vt_sys::SHITTY_VT_KEY_VOLUME_MUTE
    );

    assert_eq!(KeyAction::Press as u8, shitty_vt_sys::SHITTY_VT_KEY_PRESS);
    assert_eq!(KeyAction::Repeat as u8, shitty_vt_sys::SHITTY_VT_KEY_REPEAT);
    assert_eq!(
        KeyAction::Release as u8,
        shitty_vt_sys::SHITTY_VT_KEY_RELEASE
    );

    assert_eq!(
        MouseButton::Left as i32,
        shitty_vt_sys::SHITTY_VT_MOUSE_LEFT
    );
    assert_eq!(
        MouseButton::Aux5 as i32,
        shitty_vt_sys::SHITTY_VT_MOUSE_AUX5
    );

    assert_eq!(Modifiers::SHIFT.0, shitty_vt_sys::SHITTY_VT_MOD_SHIFT);
    assert_eq!(
        Modifiers::ALT_GRAPH.0,
        shitty_vt_sys::SHITTY_VT_MOD_ALT_GRAPH
    );
}

#[test]
fn modifiers_combine() {
    let mut held = Modifiers::CONTROL | Modifiers::SHIFT;
    assert!(held.contains(Modifiers::CONTROL));
    assert!(held.contains(Modifiers::CONTROL | Modifiers::SHIFT));
    assert!(!held.contains(Modifiers::ALT));
    assert!(!held.is_empty());
    assert!(Modifiers::NONE.is_empty());

    held |= Modifiers::ALT;
    assert!(held.contains(Modifiers::ALT));
}
