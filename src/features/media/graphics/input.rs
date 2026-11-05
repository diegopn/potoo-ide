use super::*;

pub(super) fn install_input_controllers(
    area: &gtk::DrawingArea,
    window: &gtk::Window,
    state: Rc<RefCell<GraphicsState>>,
    focus_on_window_active: bool,
) {
    let keys = gtk::EventControllerKey::new();
    let key_state = state.clone();
    keys.connect_key_pressed(move |_, key, hardware_keycode, _| {
        if let Some(code) = java_key_code(key) {
            key_state
                .borrow_mut()
                .pressed_keys
                .insert(hardware_keycode, code);
        }
        gtk::glib::Propagation::Proceed
    });
    let key_state = state.clone();
    keys.connect_key_released(move |_, key, hardware_keycode, _| {
        if let Some(code) = java_key_code(key) {
            let mut state = key_state.borrow_mut();
            let code = state.pressed_keys.remove(&hardware_keycode).unwrap_or(code);
            let reads = std::mem::take(&mut state.pending_key_reads);
            drop(state);
            for response in reads {
                let _ = response.send(Ok(HostValue::Integer(code)));
            }
        }
    });
    area.add_controller(keys);

    let pointer = gtk::EventControllerMotion::new();
    let motion_state = state.clone();
    let area_for_motion = area.clone();
    pointer.connect_motion(move |_, x, y| {
        let (x, y) = graphics_pointer_position(&motion_state.borrow(), &area_for_motion, x, y);
        let mut state = motion_state.borrow_mut();
        state.mouse_x = x;
        state.mouse_y = y;
    });
    area.add_controller(pointer);

    let click = gtk::GestureClick::new();
    click.set_button(0);
    let press_state = state.clone();
    let area_for_press = area.clone();
    click.connect_pressed(move |gesture, _, x, y| {
        area_for_press.grab_focus();
        if let Some(button) = portugol_mouse_button(gesture.current_button()) {
            let (x, y) = graphics_pointer_position(&press_state.borrow(), &area_for_press, x, y);
            let mut state = press_state.borrow_mut();
            state.pressed_buttons.insert(button);
            state.mouse_x = x;
            state.mouse_y = y;
        }
    });
    let release_state = state.clone();
    let area_for_release = area.clone();
    click.connect_released(move |gesture, _, x, y| {
        if let Some(button) = portugol_mouse_button(gesture.current_button()) {
            let (x, y) =
                graphics_pointer_position(&release_state.borrow(), &area_for_release, x, y);
            let mut state = release_state.borrow_mut();
            state.pressed_buttons.remove(&button);
            state.mouse_x = x;
            state.mouse_y = y;
            let reads = std::mem::take(&mut state.pending_button_reads);
            drop(state);
            for response in reads {
                let _ = response.send(Ok(HostValue::Integer(button)));
            }
        }
    });
    area.add_controller(click);

    let focus = gtk::EventControllerFocus::new();
    let focus_state = state;
    focus.connect_leave(move |_| {
        let mut state = focus_state.borrow_mut();
        state.pressed_keys.clear();
        state.pressed_buttons.clear();
        for response in state.pending_key_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
        for response in state.pending_button_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
    });
    area.add_controller(focus);

    if focus_on_window_active {
        let area_for_window = area.clone();
        window.connect_is_active_notify(move |window| {
            if window.is_active() {
                area_for_window.grab_focus();
            }
        });
    }
}

pub(super) fn graphics_pointer_position(
    state: &GraphicsState,
    area: &gtk::DrawingArea,
    x: f64,
    y: f64,
) -> (i64, i64) {
    if state.preview_active {
        let width = f64::from(area.width().max(1));
        let height = f64::from(area.height().max(1));
        let scale = (width / f64::from(state.width)).min(height / f64::from(state.height));
        let left = (width - f64::from(state.width) * scale) / 2.0;
        let top = (height - f64::from(state.height) * scale) / 2.0;
        (
            ((x - left) / scale).round() as i64,
            ((y - top) / scale).round() as i64,
        )
    } else {
        (x.round() as i64, y.round() as i64)
    }
}

pub(super) fn portugol_mouse_button(button: u32) -> Option<i64> {
    match button {
        1 => Some(0),
        3 => Some(1),
        2 => Some(2),
        _ => None,
    }
}

pub(super) fn java_key_code(key: gtk::gdk::Key) -> Option<i64> {
    let name = key.name()?.to_string();
    let simple = match name.as_str() {
        "Return" => 10,
        "KP_Enter" => 10,
        "BackSpace" => 8,
        "Tab" | "ISO_Left_Tab" => 9,
        "Cancel" => 3,
        "Clear" => 12,
        "Shift_L" | "Shift_R" => 16,
        "Control_L" | "Control_R" => 17,
        "Alt_L" | "Alt_R" => 18,
        "Pause" => 19,
        "Caps_Lock" => 20,
        "Escape" => 27,
        "space" => 32,
        "Page_Up" => 33,
        "Page_Down" => 34,
        "End" => 35,
        "Home" => 36,
        "Left" => 37,
        "Up" => 38,
        "Right" => 39,
        "Down" => 40,
        "comma" => 44,
        "minus" => 45,
        "period" => 46,
        "slash" => 47,
        "semicolon" => 59,
        "equal" => 61,
        "KP_Equal" => 61,
        "bracketleft" => 91,
        "backslash" => 92,
        "bracketright" => 93,
        "KP_0" => 96,
        "KP_1" => 97,
        "KP_2" => 98,
        "KP_3" => 99,
        "KP_4" => 100,
        "KP_5" => 101,
        "KP_6" => 102,
        "KP_7" => 103,
        "KP_8" => 104,
        "KP_9" => 105,
        "KP_Multiply" => 106,
        "KP_Add" => 107,
        "KP_Separator" => 108,
        "KP_Subtract" => 109,
        "KP_Decimal" => 110,
        "KP_Divide" => 111,
        "Delete" => 127,
        "Num_Lock" => 144,
        "Scroll_Lock" => 145,
        "Print" => 154,
        "Insert" => 155,
        "Help" => 156,
        "Meta_L" | "Meta_R" => 157,
        "grave" => 192,
        "apostrophe" => 222,
        "ampersand" => 150,
        "asterisk" => 151,
        "quotedbl" => 152,
        "less" => 153,
        "greater" => 160,
        "braceleft" => 161,
        "braceright" => 162,
        "at" => 512,
        "colon" => 513,
        "asciicircum" => 514,
        "dollar" => 515,
        "EuroSign" => 516,
        "exclam" => 517,
        "exclamdown" => 518,
        "parenleft" => 519,
        "numbersign" => 520,
        "plus" => 521,
        "parenright" => 522,
        "underscore" => 523,
        "Super_L" | "Super_R" => 524,
        "Menu" => 525,
        "F1" => 112,
        "F2" => 113,
        "F3" => 114,
        "F4" => 115,
        "F5" => 116,
        "F6" => 117,
        "F7" => 118,
        "F8" => 119,
        "F9" => 120,
        "F10" => 121,
        "F11" => 122,
        "F12" => 123,
        "F13" => 61_440,
        "F14" => 61_441,
        "F15" => 61_442,
        "F16" => 61_443,
        "F17" => 61_444,
        "F18" => 61_445,
        "F19" => 61_446,
        "F20" => 61_447,
        "F21" => 61_448,
        "F22" => 61_449,
        "F23" => 61_450,
        "F24" => 61_451,
        _ => {
            if name.len() == 1 {
                let character = name.chars().next()?;
                if character.is_ascii_alphabetic() {
                    return Some(i64::from(character.to_ascii_uppercase() as u8));
                }
                if character.is_ascii_digit() {
                    return Some(i64::from(character as u8));
                }
            }
            return Some(0);
        }
    };
    Some(simple)
}
