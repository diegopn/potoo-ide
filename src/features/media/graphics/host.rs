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

    pub(crate) fn close_preview_window(&self) {
        if let Some(window) = &self.preview_window {
            window.close();
        }
    }

    pub(crate) fn graphics(
        &mut self,
        root: Option<&Path>,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        if requires_initialized_graphics(function) && !graphics_is_open(&self.state.borrow()) {
            return Err("inicie o modo gráfico antes de usar esta função".to_owned());
        }
        match function {
            "iniciar_modo_grafico" => {
                let _keep_visible = logical_argument(arguments, 0)?;
                self.ensure_window("Portugol")?;
                return Ok(HostValue::Null);
            }
            "fechar_janela" => {
                let window = self.state.borrow().window.clone();
                if let Some(window) = window {
                    window.close();
                }
                return Ok(HostValue::Null);
            }
            "minimizar_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.minimize();
                }
                return Ok(HostValue::Null);
            }
            "restaurar_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.unminimize();
                    window.present();
                }
                return Ok(HostValue::Null);
            }
            "ocultar_borda_janela" | "exibir_borda_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.set_decorated(function == "exibir_borda_janela");
                }
                return Ok(HostValue::Null);
            }
            "encerrar_modo_grafico" => {
                let mut state = self.state.borrow_mut();
                let window = state.window.clone();
                state.preview_active = false;
                drop(state);
                if let Some(window) = window {
                    window.set_visible(false);
                }
                self.queue_draw();
                return Ok(HostValue::Null);
            }
            "definir_dimensoes_janela" => {
                let width = checked_dimension(integer_argument(arguments, 0)?)?;
                let height = checked_dimension(integer_argument(arguments, 1)?)?;
                self.resize(width, height)?;
                return Ok(HostValue::Null);
            }
            "definir_titulo_janela" => {
                let title = string_argument(arguments, 0)?;
                if let Some(window) = &self.state.borrow().window {
                    window.set_title(Some(&title));
                }
                return Ok(HostValue::Null);
            }
            "limpar" => {
                self.clear_surface()?;
                return Ok(HostValue::Null);
            }
            "renderizar" => {
                present_frame(&mut self.state.borrow_mut())?;
                self.queue_draw();
                return Ok(HostValue::Null);
            }
            "definir_cor" => {
                let mut state = self.state.borrow_mut();
                state.color = integer_argument(arguments, 0)?;
                state.gradient = None;
                return Ok(HostValue::Null);
            }
            "definir_gradiente" => {
                let kind = integer_argument(arguments, 0)?;
                if !(0..=7).contains(&kind) {
                    return Err("tipo de gradiente inválido".to_owned());
                }
                let mut state = self.state.borrow_mut();
                state.gradient = Some((
                    kind,
                    integer_argument(arguments, 1)?,
                    integer_argument(arguments, 2)?,
                ));
                return Ok(HostValue::Null);
            }
            "definir_fonte_texto" => {
                self.state.borrow_mut().font = string_argument(arguments, 0)?;
                return Ok(HostValue::Null);
            }
            "definir_tamanho_texto" => {
                let size = real_argument(arguments, 0)?;
                if !size.is_finite() || size <= 0.0 || size > 512.0 {
                    return Err("o tamanho do texto deve estar entre 0 e 512".to_owned());
                }
                self.state.borrow_mut().text_size = size;
                return Ok(HostValue::Null);
            }
            "definir_estilo_texto" => {
                let mut state = self.state.borrow_mut();
                state.italic = logical_argument(arguments, 0)?;
                state.bold = logical_argument(arguments, 1)?;
                state.underline = logical_argument(arguments, 2)?;
                return Ok(HostValue::Null);
            }
            "definir_opacidade" => {
                let opacity = integer_argument(arguments, 0)?;
                if !(0..=255).contains(&opacity) {
                    return Err("o valor da opacidade deve estar entre 0 e 255".to_owned());
                }
                self.state.borrow_mut().opacity = opacity;
                return Ok(HostValue::Null);
            }
            "definir_rotacao" => {
                self.state.borrow_mut().rotation = integer_argument(arguments, 0)?;
                return Ok(HostValue::Null);
            }
            "largura_janela" => {
                return Ok(HostValue::Integer(i64::from(self.state.borrow().width)));
            }
            "altura_janela" => {
                return Ok(HostValue::Integer(i64::from(self.state.borrow().height)));
            }
            "largura_tela" | "altura_tela" => {
                let display = gtk::gdk::Display::default()
                    .ok_or_else(|| "não há uma tela gráfica disponível".to_owned())?;
                let monitor = display
                    .monitors()
                    .item(0)
                    .and_downcast::<gtk::gdk::Monitor>()
                    .ok_or_else(|| "não há um monitor disponível".to_owned())?;
                let geometry = monitor.geometry();
                let dimension = if function == "largura_tela" {
                    geometry.width()
                } else {
                    geometry.height()
                };
                return Ok(HostValue::Integer(i64::from(dimension)));
            }
            "entrar_modo_tela_cheia" => {
                let window = self.state.borrow().window.clone();
                if let Some(window) = window {
                    if self.state.borrow().fullscreen_restore.is_none() {
                        let dimensions = gtk::prelude::WidgetExt::display(&window)
                            .monitors()
                            .item(0)
                            .and_downcast::<gtk::gdk::Monitor>()
                            .map(|monitor| {
                                let geometry = monitor.geometry();
                                (geometry.width(), geometry.height())
                            });
                        {
                            let mut state = self.state.borrow_mut();
                            state.fullscreen_restore = Some((state.width, state.height));
                        }
                        if let Some((width, height)) = dimensions {
                            self.resize(width, height)?;
                        }
                    }
                    window.fullscreen();
                    self.queue_draw();
                }
                return Ok(HostValue::Null);
            }
            "sair_modo_tela_cheia" => {
                let window = self.state.borrow().window.clone();
                if let Some(window) = window {
                    window.unfullscreen();
                    let restore = self.state.borrow_mut().fullscreen_restore.take();
                    if let Some((width, height)) = restore {
                        self.resize(width, height)?;
                    }
                }
                return Ok(HostValue::Null);
            }
            "desenhar_ponto" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state, (x, y, 1.0, 1.0))?;
                    context.rectangle(x, y, 1.0, 1.0);
                    context.fill().map_err(|error| error.to_string())
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_linha" => {
                let x1 = real_argument(arguments, 0)?;
                let y1 = real_argument(arguments, 1)?;
                let x2 = real_argument(arguments, 2)?;
                let y2 = real_argument(arguments, 3)?;
                self.with_draw_context(|context, state| {
                    let bounds = bounds_from_points(&[(x1, y1), (x2, y2)]);
                    set_graphics_source(context, state, bounds)?;
                    rotate_about(context, state.rotation, (x1 + x2) / 2.0, (y1 + y2) / 2.0)?;
                    context.move_to(x1, y1);
                    context.line_to(x2, y2);
                    let result = context.stroke().map_err(|error| error.to_string());
                    context.restore().map_err(|error| error.to_string())?;
                    result
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_retangulo" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let width = real_argument(arguments, 2)?;
                let height = real_argument(arguments, 3)?;
                let rounded = logical_argument(arguments, 4)?;
                let filled = logical_argument(arguments, 5)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state, (x, y, width, height))?;
                    rotate_about(context, state.rotation, x + width / 2.0, y + height / 2.0)?;
                    if rounded {
                        rounded_rectangle(context, x, y, width, height, 8.0);
                    } else {
                        context.rectangle(x, y, width, height);
                    }
                    let result = if filled {
                        context.fill().map_err(|error| error.to_string())
                    } else {
                        context.stroke().map_err(|error| error.to_string())
                    };
                    context.restore().map_err(|error| error.to_string())?;
                    result
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_elipse" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let width = real_argument(arguments, 2)?;
                let height = real_argument(arguments, 3)?;
                let filled = logical_argument(arguments, 4)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state, (x, y, width, height))?;
                    rotate_about(context, state.rotation, x + width / 2.0, y + height / 2.0)?;
                    let result = draw_ellipse(context, x, y, width, height, filled);
                    context.restore().map_err(|error| error.to_string())?;
                    result
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_poligono" => {
                let points = point_array_argument(arguments, 0)?;
                let filled = logical_argument(arguments, 1)?;
                if points.len() < 2 {
                    return Err("um polígono precisa de pelo menos dois pontos".to_owned());
                }
                self.with_draw_context(|context, state| {
                    let bounds = bounds_from_points(&points);
                    set_graphics_source(context, state, bounds)?;
                    rotate_about(
                        context,
                        state.rotation,
                        bounds.0 + bounds.2 / 2.0,
                        bounds.1 + bounds.3 / 2.0,
                    )?;
                    let result = draw_polygon(context, &points, filled);
                    context.restore().map_err(|error| error.to_string())?;
                    result
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_texto" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let text = string_argument(arguments, 2)?;
                self.draw_text(x, y, &text)?;
                return Ok(HostValue::Null);
            }
            "jerimum_desenhar_texto" => {
                let text = string_argument(arguments, 0)?;
                let mut x = real_argument(arguments, 1)?;
                let mut y = real_argument(arguments, 2)?;
                let size = real_argument(arguments, 3)?;
                let centered = logical_argument(arguments, 4)?;
                if !size.is_finite() || !(1.0..=512.0).contains(&size) {
                    return Err("o tamanho da fonte deve estar entre 1 e 512".to_owned());
                }
                {
                    let mut state = self.state.borrow_mut();
                    state.text_size = size;
                    state.bold = true;
                }
                if centered {
                    let context = self.measure_context()?;
                    x -= text_advance(&context, &text)? / 2.0;
                    y -= text_height(&context)? / 2.0;
                }
                self.draw_text(x, y, &text)?;
                return Ok(HostValue::Null);
            }
            "largura_texto" | "altura_texto" => {
                let text = string_argument(arguments, 0)?;
                let context = self.measure_context()?;
                let value = if function == "largura_texto" {
                    text_advance(&context, &text)?
                } else {
                    text_height(&context)?
                };
                return Ok(HostValue::Integer(value.round() as i64));
            }
            "criar_cor" => {
                let red = color_channel(integer_argument(arguments, 0)?)?;
                let green = color_channel(integer_argument(arguments, 1)?)?;
                let blue = color_channel(integer_argument(arguments, 2)?)?;
                return Ok(HostValue::Integer(i64::from(
                    u32::from_be_bytes([255, red, green, blue]) as i32,
                )));
            }
            "obter_rgb" => {
                let color = integer_argument(arguments, 0)? as u32;
                let channel = integer_argument(arguments, 1)?;
                let value = match channel {
                    0 => (color >> 16) & 0xff,
                    1 => (color >> 8) & 0xff,
                    2 => color & 0xff,
                    _ => 0,
                };
                return Ok(HostValue::Integer(i64::from(value)));
            }
            "carregar_fonte" => {
                let path = safe_project_file(root, &string_argument(arguments, 0)?)?;
                let area =
                    self.state.borrow().area.clone().ok_or_else(|| {
                        "inicie o modo gráfico antes de carregar fontes".to_owned()
                    })?;
                let font_map = area
                    .pango_context()
                    .font_map()
                    .ok_or_else(|| "não foi possível acessar o catálogo de fontes".to_owned())?;
                crate::register_program_font(&path)?;
                font_map.changed();
                return Ok(HostValue::Null);
            }
            "definir_icone_janela" => {
                let handle = integer_argument(arguments, 0)?;
                let pixbuf = self.image_pixbuf(handle)?;
                let window = self.state.borrow().window.clone();
                self.state.borrow_mut().icon = Some(pixbuf.clone());
                if let Some(window) = window {
                    set_window_icon(&window, &pixbuf)?;
                }
                return Ok(HostValue::Null);
            }
            _ => {}
        }

        self.image_operation(root, function, arguments)
    }

    fn resize(&self, width: i32, height: i32) -> Result<(), String> {
        resize_graphics_state(&mut self.state.borrow_mut(), width, height)?;
        let state = self.state.borrow();
        if !state.preview_active
            && let Some(area) = &state.area
        {
            area.set_content_width(width);
            area.set_content_height(height);
        }
        if let Some(window) = &state.window {
            window.set_default_size(width, height);
        }
        if let Some(area) = &self.preview_window_area {
            area.set_content_width(width);
            area.set_content_height(height);
        }
        if let Some(area) = &self.preview_area
            && let Some(frame) = preview_aspect_frame(area)
        {
            frame.set_ratio(width as f32 / height as f32);
        }
        drop(state);
        self.queue_draw();
        Ok(())
    }

    fn clear_surface(&self) -> Result<(), String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let surface = state.surface.as_ref().unwrap();
        let context = gtk::cairo::Context::new(surface).map_err(|error| error.to_string())?;
        context.set_operator(gtk::cairo::Operator::Source);
        let (red, green, blue, alpha) = rgba(state.color, state.opacity as f64 / 255.0);
        context.set_source_rgba(red, green, blue, alpha);
        context.paint().map_err(|error| error.to_string())?;
        surface.flush();
        Ok(())
    }

    fn queue_draw(&self) {
        if let Some(area) = &self.state.borrow().area {
            area.queue_draw();
        }
        if let Some(area) = &self.preview_window_area {
            area.queue_draw();
        }
    }

    fn with_draw_context(
        &self,
        draw: impl FnOnce(&gtk::cairo::Context, &GraphicsState) -> Result<(), String>,
    ) -> Result<(), String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let surface = state.surface.as_ref().unwrap();
        let context = gtk::cairo::Context::new(surface).map_err(|error| error.to_string())?;
        context.set_line_width(1.0);
        draw(&context, &state)?;
        surface.flush();
        Ok(())
    }

    fn draw_text(&self, x: f64, y: f64, text: &str) -> Result<(), String> {
        self.with_draw_context(|context, state| {
            select_font(context, state);
            let extents = context
                .text_extents(text)
                .map_err(|error| error.to_string())?;
            let font_extents = context.font_extents().map_err(|error| error.to_string())?;
            let width = extents.x_advance();
            let height = font_extents.height();
            set_graphics_source(context, state, (x, y, width, height))?;
            rotate_about(context, state.rotation, x + width / 2.0, y + height / 2.0)?;
            context.move_to(x, y + font_extents.ascent());
            let result = context.show_text(text).map_err(|error| error.to_string());
            result?;
            if state.underline {
                let underline_y = y + font_extents.ascent() + 2.0;
                context.move_to(x, underline_y);
                context.line_to(x + width, underline_y);
                context.stroke().map_err(|error| error.to_string())?;
            }
            context.restore().map_err(|error| error.to_string())
        })
    }

    fn measure_context(&self) -> Result<gtk::cairo::Context, String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let context = gtk::cairo::Context::new(state.surface.as_ref().unwrap())
            .map_err(|error| error.to_string())?;
        select_font(&context, &state);
        Ok(context)
    }

    fn image_operation(
        &mut self,
        root: Option<&Path>,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        match function {
            "renderizar_imagem" => {
                self.ensure_surface()?;
                let width = checked_dimension(integer_argument(arguments, 0)?)?;
                let height = checked_dimension(integer_argument(arguments, 1)?)?;
                let state = self.state.borrow();
                checked_image_memory(state.image_bytes, width as usize * height as usize * 4)?;
                let pending = state.surface.as_ref().unwrap().clone();
                let pixbuf = capture_pending_pixbuf(&pending, width, height)?;
                let empty_surface = transparent_surface(state.width, state.height)?;
                drop(state);
                let handle = self.insert_image(ImageAsset::still(pixbuf))?;
                self.state.borrow_mut().surface = Some(empty_surface);
                Ok(HostValue::Integer(handle))
            }
            "carregar_imagem" => {
                let path = safe_project_image_file(root, &string_argument(arguments, 0)?)?;
                let image = match path {
                    Some(path) => match load_image_asset(&path) {
                        Ok(image) => image,
                        Err(_) => empty_image_asset()?,
                    },
                    None => empty_image_asset()?,
                };
                Ok(HostValue::Integer(self.insert_image(image)?))
            }
            "proximo_frame_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let mut state = self.state.borrow_mut();
                let image = get_gif_mut(&mut state, handle)?;
                image.current_frame = (image.current_frame + 1) % image.frames.len();
                Ok(HostValue::Null)
            }
            "transformar_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let horizontal = logical_argument(arguments, 1)?;
                let vertical = logical_argument(arguments, 2)?;
                let rotation = integer_argument(arguments, 3)?;
                let ignored_color = integer_argument(arguments, 4)?;
                let source = self.image_pixbuf(handle)?;
                let pixbuf =
                    transform_pixbuf(&source, horizontal, vertical, rotation, ignored_color)?;
                Ok(HostValue::Integer(
                    self.insert_image(ImageAsset::still(pixbuf))?,
                ))
            }
            "redimensionar_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let mut width = integer_argument(arguments, 1)?;
                let mut height = integer_argument(arguments, 2)?;
                let quality = logical_argument(arguments, 3)?;
                let source = self.image_pixbuf(handle)?;
                if width <= 0 && height <= 0 {
                    return Err(
                        "a largura e a altura da imagem devem ser maiores que zero".to_owned()
                    );
                }
                if width == 0 {
                    width =
                        (i64::from(source.width()) * height / i64::from(source.height())).max(1);
                } else if height == 0 {
                    height =
                        (i64::from(source.height()) * width / i64::from(source.width())).max(1);
                }
                let width = checked_dimension(width)?;
                let height = checked_dimension(height)?;
                check_pixel_count(width, height)?;
                let interpolation = if quality {
                    gtk::gdk_pixbuf::InterpType::Bilinear
                } else {
                    gtk::gdk_pixbuf::InterpType::Nearest
                };
                let image = {
                    let state = self.state.borrow();
                    let source = state
                        .images
                        .get(&handle)
                        .ok_or_else(|| format!("endereço de imagem inválido ({handle})"))?;
                    resize_image_asset(source, width, height, interpolation)?
                };
                Ok(HostValue::Integer(self.insert_image(image)?))
            }
            "obter_cor_pixel" => {
                let handle = integer_argument(arguments, 0)?;
                let x = i32::try_from(integer_argument(arguments, 1)?)
                    .map_err(|_| "coordenada X inválida".to_owned())?;
                let y = i32::try_from(integer_argument(arguments, 2)?)
                    .map_err(|_| "coordenada Y inválida".to_owned())?;
                let pixbuf = self.image_pixbuf(handle)?;
                Ok(HostValue::Integer(pixbuf_pixel_color(&pixbuf, x, y)?))
            }
            "transformar_porcao_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let x = i32_argument(arguments, 1, "coordenada X inválida")?;
                let y = i32_argument(arguments, 2, "coordenada Y inválida")?;
                let width = i32_argument(arguments, 3, "largura inválida")?;
                let height = i32_argument(arguments, 4, "altura inválida")?;
                let horizontal = logical_argument(arguments, 5)?;
                let vertical = logical_argument(arguments, 6)?;
                let rotation = integer_argument(arguments, 7)?;
                let ignored_color = integer_argument(arguments, 8)?;
                let source = self.image_pixbuf(handle)?;
                validate_crop(&source, x, y, width, height)?;
                let region = source.new_subpixbuf(x, y, width, height);
                let transformed =
                    transform_pixbuf(&region, horizontal, vertical, rotation, ignored_color)?;
                Ok(HostValue::Integer(
                    self.insert_image(ImageAsset::still(transformed))?,
                ))
            }
            "desenhar_imagem" | "desenhar_quadro_atual_gif" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let handle = integer_argument(arguments, 2)?;
                if function == "desenhar_quadro_atual_gif" {
                    let state = self.state.borrow();
                    get_gif(&state, handle)?;
                }
                let pixbuf = self.image_pixbuf(handle)?;
                self.draw_pixbuf(x, y, &pixbuf)?;
                Ok(HostValue::Null)
            }
            "jerimum_desenhar_imagem" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let handle = integer_argument(arguments, 2)?;
                let angle = real_argument(arguments, 3)?;
                let scale_x = real_argument(arguments, 4)?;
                let scale_y = real_argument(arguments, 5)?;
                if !angle.is_finite() || !scale_x.is_finite() || !scale_y.is_finite() {
                    return Err("transformação de imagem inválida".to_owned());
                }
                let pixbuf = self.image_pixbuf(handle)?;
                self.with_draw_context(|context, state| {
                    context.save().map_err(|error| error.to_string())?;
                    context.translate(
                        x + f64::from(pixbuf.width()) / 2.0,
                        y + f64::from(pixbuf.height()) / 2.0,
                    );
                    context.rotate(angle.to_radians());
                    context.scale(scale_x, scale_y);
                    context.set_source_pixbuf(
                        &pixbuf,
                        -f64::from(pixbuf.width()) / 2.0,
                        -f64::from(pixbuf.height()) / 2.0,
                    );
                    let result = context
                        .paint_with_alpha(state.opacity as f64 / 255.0)
                        .map_err(|error| error.to_string());
                    context.restore().map_err(|error| error.to_string())?;
                    result
                })?;
                Ok(HostValue::Null)
            }
            "desenhar_porcao_imagem" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let source_x = i32_argument(arguments, 2, "coordenada X inválida")?;
                let source_y = i32_argument(arguments, 3, "coordenada Y inválida")?;
                let width = i32_argument(arguments, 4, "largura inválida")?;
                let height = i32_argument(arguments, 5, "altura inválida")?;
                let handle = integer_argument(arguments, 6)?;
                let pixbuf = self.image_pixbuf(handle)?;
                validate_crop(&pixbuf, source_x, source_y, width, height)?;
                let region = pixbuf.new_subpixbuf(source_x, source_y, width, height);
                self.draw_pixbuf(x, y, &region)?;
                Ok(HostValue::Null)
            }
            "salvar_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let target = safe_project_output(root, &string_argument(arguments, 1)?)?;
                let pixbuf = self.image_pixbuf(handle)?;
                let extension = target
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("png")
                    .to_ascii_lowercase();
                let format = match extension.as_str() {
                    "jpg" | "jpeg" => "jpeg",
                    "png" | "bmp" | "gif" | "tif" | "tiff" | "webp" => extension.as_str(),
                    _ => return Err("formato de imagem não suportado".to_owned()),
                };
                pixbuf
                    .savev(&target, format, &[])
                    .map_err(|error| format!("não foi possível salvar a imagem: {error}"))?;
                Ok(HostValue::Null)
            }
            "obter_intervalo_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let state = self.state.borrow();
                let image = get_gif(&state, handle)?;
                Ok(HostValue::Integer(image.frame_delays[image.current_frame]))
            }
            "obter_numero_quadros_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let state = self.state.borrow();
                let image = get_gif(&state, handle)?;
                Ok(HostValue::Integer(image.frames.len() as i64))
            }
            "obter_numero_quadro_atual_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let state = self.state.borrow();
                let image = get_gif(&state, handle)?;
                Ok(HostValue::Integer(image.current_frame as i64))
            }
            "obter_quadro_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let frame = usize::try_from(integer_argument(arguments, 1)?)
                    .map_err(|_| "quadro de animação inválido".to_owned())?;
                let pixbuf = {
                    let state = self.state.borrow();
                    let gif = get_gif(&state, handle)?;
                    gif.frames
                        .get(frame)
                        .cloned()
                        .ok_or_else(|| "quadro de animação fora do limite".to_owned())?
                };
                Ok(HostValue::Integer(
                    self.insert_image(ImageAsset::still(pixbuf))?,
                ))
            }
            "definir_quadro_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let frame = usize::try_from(integer_argument(arguments, 1)?)
                    .map_err(|_| "quadro de animação inválido".to_owned())?;
                let mut state = self.state.borrow_mut();
                let gif = get_gif_mut(&mut state, handle)?;
                if frame >= gif.frames.len() {
                    return Err("quadro de animação fora do limite".to_owned());
                }
                gif.current_frame = frame;
                Ok(HostValue::Null)
            }
            "liberar_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let mut state = self.state.borrow_mut();
                if let Some(image) = state.images.remove(&handle) {
                    state.image_bytes = state
                        .image_bytes
                        .saturating_sub(image_asset_memory_bytes(&image).unwrap_or(0));
                }
                Ok(HostValue::Null)
            }
            "largura_imagem" | "altura_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let pixbuf = self.image_pixbuf(handle)?;
                let dimension = if function == "largura_imagem" {
                    pixbuf.width()
                } else {
                    pixbuf.height()
                };
                Ok(HostValue::Integer(i64::from(dimension)))
            }
            _ => Err(format!("função gráfica desconhecida: {function}")),
        }
    }

    fn insert_image(&self, image: ImageAsset) -> Result<i64, String> {
        let image_bytes = image_asset_memory_bytes(&image)?;
        let mut state = self.state.borrow_mut();
        let total_image_bytes = checked_image_memory(state.image_bytes, image_bytes)?;
        let handle = state.next_image_handle;
        state.next_image_handle = state
            .next_image_handle
            .checked_add(1)
            .ok_or_else(|| "não há mais identificadores de imagem disponíveis".to_owned())?;
        state.images.insert(handle, image);
        state.image_bytes = total_image_bytes;
        Ok(handle)
    }

    fn image_pixbuf(&self, handle: i64) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
        let state = self.state.borrow();
        let image = state
            .images
            .get(&handle)
            .ok_or_else(|| format!("endereço de imagem inválido ({handle})"))?;
        image
            .frames
            .get(image.current_frame)
            .cloned()
            .ok_or_else(|| format!("endereço de imagem inválido ({handle})"))
    }

    fn draw_pixbuf(&self, x: f64, y: f64, pixbuf: &gtk::gdk_pixbuf::Pixbuf) -> Result<(), String> {
        self.with_draw_context(|context, state| {
            rotate_about(
                context,
                state.rotation,
                x + f64::from(pixbuf.width()) / 2.0,
                y + f64::from(pixbuf.height()) / 2.0,
            )?;
            context.set_source_pixbuf(pixbuf, x, y);
            let result = context
                .paint_with_alpha(state.opacity as f64 / 255.0)
                .map_err(|error| error.to_string());
            context.restore().map_err(|error| error.to_string())?;
            result
        })
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
