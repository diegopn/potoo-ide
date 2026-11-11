mod style;
mod surface;
mod window;

use super::*;

impl GraphicsHost {
    pub(crate) fn graphics(
        &mut self,
        root: Option<&Path>,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        ensure_graphics_initialized(function, &self.state.borrow())?;

        if let Some(value) = self.window_command(function, arguments)? {
            return Ok(value);
        }
        if let Some(value) = self.surface_command(function, arguments)? {
            return Ok(value);
        }
        if let Some(value) = self.style_command(root, function, arguments)? {
            return Ok(value);
        }

        self.image_operation(root, function, arguments)
    }
}

fn ensure_graphics_initialized(function: &str, state: &GraphicsState) -> Result<(), String> {
    if requires_initialized_graphics(function) && !graphics_is_open(state) {
        Err("inicie o modo gráfico antes de usar esta função".to_owned())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphics_commands_require_an_open_window_only_for_existing_commands() {
        let mut state = GraphicsState::default();
        assert_eq!(
            ensure_graphics_initialized("desenhar_ponto", &state),
            Err("inicie o modo gráfico antes de usar esta função".to_owned())
        );
        assert_eq!(
            ensure_graphics_initialized("carregar_imagem", &state),
            Ok(())
        );

        state.preview_active = true;
        assert_eq!(
            ensure_graphics_initialized("desenhar_ponto", &state),
            Ok(())
        );
    }
}
