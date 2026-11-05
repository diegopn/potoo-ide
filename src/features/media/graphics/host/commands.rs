use super::*;

impl GraphicsHost {
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
}
