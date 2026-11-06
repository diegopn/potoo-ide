use super::*;

impl GraphicsHost {
    pub(super) fn drawing_operation(
        &mut self,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<Option<HostValue>, String> {
        match function {
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
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
            }
            _ => Ok(None),
        }
    }
}

impl GraphicsHost {
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
}
