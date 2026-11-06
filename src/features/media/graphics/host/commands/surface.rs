use super::*;

impl GraphicsHost {
    pub(super) fn surface_command(
        &self,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<Option<HostValue>, String> {
        match function {
            "limpar" => {
                self.clear_surface()?;
                Ok(Some(HostValue::Null))
            }
            "renderizar" => {
                present_frame(&mut self.state.borrow_mut())?;
                self.queue_draw();
                Ok(Some(HostValue::Null))
            }
            "desenhar_ponto" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state, (x, y, 1.0, 1.0))?;
                    context.rectangle(x, y, 1.0, 1.0);
                    context.fill().map_err(|error| error.to_string())
                })?;
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
            }
            "desenhar_texto" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let text = string_argument(arguments, 2)?;
                self.draw_text(x, y, &text)?;
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
            }
            "largura_texto" | "altura_texto" => {
                let text = string_argument(arguments, 0)?;
                let context = self.measure_context()?;
                let value = if function == "largura_texto" {
                    text_advance(&context, &text)?
                } else {
                    text_height(&context)?
                };
                Ok(Some(HostValue::Integer(value.round() as i64)))
            }
            _ => Ok(None),
        }
    }
}
