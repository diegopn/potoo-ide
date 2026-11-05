mod commands;
mod drawing;
mod images;
mod input;
mod preview;

use super::*;

pub(crate) struct GraphicsHost {
    parent: gtk::ApplicationWindow,
    state: Rc<RefCell<GraphicsState>>,
    preview_area: Option<gtk::DrawingArea>,
    preview_window: Option<gtk::Window>,
    preview_window_area: Option<gtk::DrawingArea>,
    on_close: Rc<dyn Fn()>,
}

impl GraphicsHost {
    pub(crate) fn new(parent: gtk::ApplicationWindow, on_close: impl Fn() + 'static) -> Self {
        Self {
            parent,
            state: Rc::new(RefCell::new(GraphicsState::default())),
            preview_area: None,
            preview_window: None,
            preview_window_area: None,
            on_close: Rc::new(on_close),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        graphics_is_open(&self.state.borrow())
    }

    pub(crate) fn finish(&mut self) {
        let mut state = self.state.borrow_mut();
        for response in state.pending_key_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
        for response in state.pending_button_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
        let window = state.window.take();
        *state = GraphicsState::default();
        let area = self.preview_area.clone();
        let preview_window = self.preview_window.take();
        self.preview_window_area = None;
        drop(state);
        if let Some(area) = area {
            area.queue_draw();
        }
        if let Some(window) = window {
            window.close();
        }
        if let Some(window) = preview_window {
            window.close();
        }
    }

    fn ensure_surface(&self) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        ensure_surface(&mut state)
    }

    fn ensure_window(&self, title: &str) -> Result<(), String> {
        if self.state.borrow().window.is_some() {
            if let Some(window) = &self.state.borrow().window {
                window.present();
            }
            return Ok(());
        }

        self.ensure_surface()?;
        let area = gtk::DrawingArea::new();
        let (width, height) = {
            let state = self.state.borrow();
            (state.width, state.height)
        };
        let state_for_draw = self.state.clone();
        area.set_content_width(width);
        area.set_content_height(height);
        area.set_focusable(true);
        area.set_draw_func(move |_, context, width, height| {
            let state = state_for_draw.borrow();
            draw_graphics_surface(context, state.presented_surface.as_ref(), width, height);
        });
        let state_for_resize = self.state.clone();
        area.connect_resize(move |_, width, height| {
            let current_dimensions = {
                let state = state_for_resize.borrow();
                (state.width, state.height)
            };
            if width > 0
                && height > 0
                && check_pixel_count(width, height).is_ok()
                && current_dimensions != (width, height)
            {
                let _ = resize_graphics_state(&mut state_for_resize.borrow_mut(), width, height);
            }
        });
        let window = gtk::Window::builder()
            .title(title)
            .default_width(width)
            .default_height(height)
            .transient_for(&self.parent)
            .destroy_with_parent(true)
            .child(&area)
            .build();

        install_input_controllers(&area, &window, self.state.clone(), true);
        let state_for_close = self.state.clone();
        let on_close = self.on_close.clone();
        window.connect_close_request(move |window| {
            let mut state = state_for_close.borrow_mut();
            state.pressed_keys.clear();
            state.pressed_buttons.clear();
            for response in state.pending_key_reads.drain(..) {
                let _ = response.send(Ok(HostValue::Integer(-1)));
            }
            for response in state.pending_button_reads.drain(..) {
                let _ = response.send(Ok(HostValue::Integer(-1)));
            }
            window.set_visible(false);
            on_close();
            gtk::glib::Propagation::Stop
        });

        let mut state = self.state.borrow_mut();
        state.window = Some(window.clone());
        state.area = Some(area.clone());
        if let Some(surface) = &state.presented_surface {
            let _ = surface.flush();
        }
        drop(state);
        window.present();
        area.grab_focus();
        if let Some(icon) = self.state.borrow().icon.clone() {
            set_window_icon(&window, &icon)?;
        }
        Ok(())
    }
}
