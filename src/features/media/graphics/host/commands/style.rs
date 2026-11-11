use super::*;

impl GraphicsHost {
    pub(super) fn style_command(
        &self,
        root: Option<&Path>,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<Option<HostValue>, String> {
        match function {
            "definir_cor" => {
                let mut state = self.state.borrow_mut();
                state.color = integer_argument(arguments, 0)?;
                state.gradient = None;
                Ok(Some(HostValue::Null))
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
                Ok(Some(HostValue::Null))
            }
            "definir_fonte_texto" => {
                self.state.borrow_mut().font = string_argument(arguments, 0)?;
                Ok(Some(HostValue::Null))
            }
            "definir_tamanho_texto" => {
                let size = real_argument(arguments, 0)?;
                if !size.is_finite() || size <= 0.0 || size > 512.0 {
                    return Err("o tamanho do texto deve estar entre 0 e 512".to_owned());
                }
                self.state.borrow_mut().text_size = size;
                Ok(Some(HostValue::Null))
            }
            "definir_estilo_texto" => {
                let mut state = self.state.borrow_mut();
                state.italic = logical_argument(arguments, 0)?;
                state.bold = logical_argument(arguments, 1)?;
                state.underline = logical_argument(arguments, 2)?;
                Ok(Some(HostValue::Null))
            }
            "definir_opacidade" => {
                let opacity = checked_opacity(integer_argument(arguments, 0)?)?;
                self.state.borrow_mut().opacity = opacity;
                Ok(Some(HostValue::Null))
            }
            "definir_rotacao" => {
                self.state.borrow_mut().rotation = integer_argument(arguments, 0)?;
                Ok(Some(HostValue::Null))
            }
            "criar_cor" => {
                let red = color_channel(integer_argument(arguments, 0)?)?;
                let green = color_channel(integer_argument(arguments, 1)?)?;
                let blue = color_channel(integer_argument(arguments, 2)?)?;
                Ok(Some(HostValue::Integer(i64::from(
                    u32::from_be_bytes([255, red, green, blue]) as i32,
                ))))
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
                Ok(Some(HostValue::Integer(i64::from(value))))
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
                Ok(Some(HostValue::Null))
            }
            _ => Ok(None),
        }
    }
}

fn checked_opacity(value: i64) -> Result<i64, String> {
    if !(0..=255).contains(&value) {
        return Err("o valor da opacidade deve estar entre 0 e 255".to_owned());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_keeps_the_supported_range() {
        assert_eq!(checked_opacity(0), Ok(0));
        assert_eq!(checked_opacity(255), Ok(255));
        assert_eq!(
            checked_opacity(-1),
            Err("o valor da opacidade deve estar entre 0 e 255".to_owned())
        );
        assert_eq!(
            checked_opacity(256),
            Err("o valor da opacidade deve estar entre 0 e 255".to_owned())
        );
    }
}
