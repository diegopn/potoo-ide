use super::*;

impl GraphicsHost {
    pub(crate) fn handle_input_request(
        &mut self,
        request: HostRequest,
        response: HostReply,
    ) -> Result<Option<HostValue>, String> {
        let mut state = self.state.borrow_mut();
        let is_mouse_or_keyboard = matches!(
            request,
            HostRequest::KeyboardKeyPressed(_)
                | HostRequest::KeyboardAnyKeyPressed
                | HostRequest::KeyboardReadKey
                | HostRequest::MouseButtonPressed(_)
                | HostRequest::MouseAnyButtonPressed
                | HostRequest::MouseReadButton
                | HostRequest::MousePosition
                | HostRequest::MouseSetCursorVisible(_)
        );
        if is_mouse_or_keyboard && !graphics_is_open(&state) {
            return Err("inicie o modo gráfico antes de usar teclado ou mouse".to_owned());
        }
        let value = match request {
            HostRequest::KeyboardKeyPressed(key) => {
                HostValue::Logical(state.pressed_keys.values().any(|pressed| *pressed == key))
            }
            HostRequest::KeyboardAnyKeyPressed => {
                HostValue::Logical(!state.pressed_keys.is_empty())
            }
            HostRequest::KeyboardReadKey => {
                state.pending_key_reads.push(response);
                return Ok(None);
            }
            HostRequest::MouseButtonPressed(button) => {
                HostValue::Logical(state.pressed_buttons.contains(&button))
            }
            HostRequest::MouseAnyButtonPressed => {
                HostValue::Logical(!state.pressed_buttons.is_empty())
            }
            HostRequest::MouseReadButton => {
                state.pending_button_reads.push(response);
                return Ok(None);
            }
            HostRequest::MousePosition => HostValue::Array(vec![
                HostValue::Integer(state.mouse_x),
                HostValue::Integer(state.mouse_y),
            ]),
            HostRequest::MouseSetCursorVisible(visible) => {
                state.cursor_visible = visible;
                if let Some(area) = &state.area {
                    if visible {
                        area.set_cursor(None);
                    } else if let Some(cursor) = gtk::gdk::Cursor::from_name("none", None) {
                        area.set_cursor(Some(&cursor));
                    }
                }
                HostValue::Null
            }
            HostRequest::GraphicsWindowIsOpen
            | HostRequest::JerimumPreviewStart
            | HostRequest::Graphics { .. } => {
                return Err("requisição incompatível com o serviço de entrada".to_owned());
            }
        };
        Ok(Some(value))
    }
}
