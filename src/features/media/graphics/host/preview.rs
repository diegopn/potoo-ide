use super::*;

impl GraphicsHost {
    pub(crate) fn set_jerimum_preview_area(&mut self, area: gtk::DrawingArea) {
        area.set_focusable(true);
        let state_for_draw = self.state.clone();
        area.set_draw_func(move |_, context, width, height| {
            let state = state_for_draw.borrow();
            draw_graphics_surface(context, state.presented_surface.as_ref(), width, height);
        });
        let parent: gtk::Window = self.parent.clone().upcast();
        install_input_controllers(&area, &parent, self.state.clone(), false);
        if let Some(frame) = preview_aspect_frame(&area) {
            let state = self.state.borrow();
            frame.set_ratio(state.width as f32 / state.height as f32);
        }
        self.preview_area = Some(area);
    }

    pub(crate) fn start_jerimum_preview(&mut self) -> Result<HostValue, String> {
        let area = self
            .preview_area
            .clone()
            .ok_or_else(|| "a prévia do jogo não está disponível".to_owned())?;
        self.ensure_surface()?;
        let mut state = self.state.borrow_mut();
        state.preview_active = true;
        state.area = Some(area.clone());
        area.queue_draw();
        Ok(HostValue::Null)
    }

    pub(crate) fn open_jerimum_preview_window(&mut self, title: &str) -> Result<(), String> {
        if !self.state.borrow().preview_active {
            return Err("execute um jogo antes de abrir outra janela".to_owned());
        }
        if let Some(window) = &self.preview_window {
            window.maximize();
            window.present();
            if let Some(area) = &self.preview_window_area {
                area.grab_focus();
            }
            return Ok(());
        }

        let (width, height) = {
            let state = self.state.borrow();
            (state.width, state.height)
        };
        let area = gtk::DrawingArea::new();
        area.set_content_width(width);
        area.set_content_height(height);
        area.set_halign(gtk::Align::Center);
        area.set_valign(gtk::Align::Center);
        area.set_focusable(true);
        area.add_css_class("potoo-game-preview-window-area");
        let state_for_draw = self.state.clone();
        area.set_draw_func(move |_, context, width, height| {
            let state = state_for_draw.borrow();
            draw_graphics_surface(context, state.presented_surface.as_ref(), width, height);
        });

        let application = self
            .parent
            .application()
            .ok_or_else(|| "o aplicativo Potoo não está disponível".to_owned())?;
        let window: gtk::Window = gtk::ApplicationWindow::builder()
            .application(&application)
            .title(title)
            .default_width(960)
            .default_height(640)
            .maximized(true)
            .modal(false)
            .build()
            .upcast();
        window.add_css_class("potoo-window");
        if self.parent.has_css_class("potoo-dark") {
            window.add_css_class("potoo-dark");
        }

        let titlebar = gtk::HeaderBar::new();
        titlebar.add_css_class("potoo-game-preview-titlebar");
        titlebar.set_show_title_buttons(false);
        let brand = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let app_icon = gtk::Image::from_icon_name("io.github.diegopn.potooide");
        app_icon.set_pixel_size(28);
        brand.append(&app_icon);
        let brand_label = gtk::Label::new(Some("Potoo IDE"));
        brand_label.add_css_class("potoo-brand-name");
        brand.append(&brand_label);
        titlebar.pack_start(&brand);
        let title_label = gtk::Label::new(Some(title));
        title_label.add_css_class("potoo-project-name");
        titlebar.set_title_widget(Some(&title_label));
        let window_controls = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        for (icon_name, tooltip, action) in [
            ("window-minimize", "Minimizar", false),
            ("close", "Fechar", true),
        ] {
            let button = gtk::Button::new();
            button.add_css_class("potoo-window-control");
            button.set_size_request(28, 28);
            button.set_tooltip_text(Some(tooltip));
            let control_icon = crate::ui::helpers::icon(icon_name);
            control_icon.set_pixel_size(18);
            button.set_child(Some(&control_icon));
            let window_for_action = window.clone();
            button.connect_clicked(move |_| {
                if action {
                    window_for_action.close();
                } else {
                    window_for_action.minimize();
                }
            });
            window_controls.append(&button);
        }
        titlebar.pack_end(&window_controls);
        window.set_titlebar(Some(&titlebar));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.add_css_class("potoo-root");
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&area));
        content.append(&scroll);
        window.set_child(Some(&content));
        install_input_controllers(&area, &window, self.state.clone(), true);
        let state_for_close = self.state.clone();
        let on_close = self.on_close.clone();
        window.connect_close_request(move |_| {
            let mut state = state_for_close.borrow_mut();
            let stop_execution = state.preview_active;
            state.pressed_keys.clear();
            state.pressed_buttons.clear();
            for response in state.pending_key_reads.drain(..) {
                let _ = response.send(Ok(HostValue::Integer(-1)));
            }
            for response in state.pending_button_reads.drain(..) {
                let _ = response.send(Ok(HostValue::Integer(-1)));
            }
            drop(state);
            if stop_execution {
                on_close();
            }
            gtk::glib::Propagation::Proceed
        });
        self.preview_window_area = Some(area.clone());
        self.preview_window = Some(window.clone());
        window.present();
        window.maximize();
        area.grab_focus();
        area.queue_draw();
        Ok(())
    }

    pub(crate) fn close_preview_window(&self) {
        if let Some(window) = &self.preview_window {
            window.close();
        }
    }
}
