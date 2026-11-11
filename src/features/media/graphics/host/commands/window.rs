use super::*;

impl GraphicsHost {
    pub(super) fn window_command(
        &self,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<Option<HostValue>, String> {
        match function {
            "iniciar_modo_grafico" => {
                let _keep_visible = logical_argument(arguments, 0)?;
                self.ensure_window("Portugol")?;
                Ok(Some(HostValue::Null))
            }
            "fechar_janela" => {
                let window = self.state.borrow().window.clone();
                if let Some(window) = window {
                    window.close();
                }
                Ok(Some(HostValue::Null))
            }
            "minimizar_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.minimize();
                }
                Ok(Some(HostValue::Null))
            }
            "restaurar_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.unminimize();
                    window.present();
                }
                Ok(Some(HostValue::Null))
            }
            "ocultar_borda_janela" | "exibir_borda_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.set_decorated(function == "exibir_borda_janela");
                }
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
            }
            "definir_dimensoes_janela" => {
                let width = checked_dimension(integer_argument(arguments, 0)?)?;
                let height = checked_dimension(integer_argument(arguments, 1)?)?;
                self.resize(width, height)?;
                Ok(Some(HostValue::Null))
            }
            "definir_titulo_janela" => {
                let title = string_argument(arguments, 0)?;
                if let Some(window) = &self.state.borrow().window {
                    window.set_title(Some(&title));
                }
                Ok(Some(HostValue::Null))
            }
            "largura_janela" => Ok(Some(HostValue::Integer(i64::from(
                self.state.borrow().width,
            )))),
            "altura_janela" => Ok(Some(HostValue::Integer(i64::from(
                self.state.borrow().height,
            )))),
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
                Ok(Some(HostValue::Integer(i64::from(dimension))))
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
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
            }
            "definir_icone_janela" => {
                let handle = integer_argument(arguments, 0)?;
                let pixbuf = self.image_pixbuf(handle)?;
                let window = self.state.borrow().window.clone();
                self.state.borrow_mut().icon = Some(pixbuf.clone());
                if let Some(window) = window {
                    set_window_icon(&window, &pixbuf)?;
                }
                Ok(Some(HostValue::Null))
            }
            _ => Ok(None),
        }
    }
}

impl GraphicsHost {
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
