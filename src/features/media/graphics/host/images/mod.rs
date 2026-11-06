mod animation;
mod assets;
mod drawing;
mod transforms;

use super::*;

impl GraphicsHost {
    pub(super) fn image_operation(
        &mut self,
        root: Option<&Path>,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        if let Some(value) = self.assets_operation(root, function, arguments)? {
            return Ok(value);
        }
        if let Some(value) = self.transforms_operation(function, arguments)? {
            return Ok(value);
        }
        if let Some(value) = self.drawing_operation(function, arguments)? {
            return Ok(value);
        }
        if let Some(value) = self.animation_operation(function, arguments)? {
            return Ok(value);
        }

        Err(unknown_graphics_function(function))
    }
}

fn unknown_graphics_function(function: &str) -> String {
    format!("função gráfica desconhecida: {function}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_graphics_commands_keep_the_interpreter_error() {
        assert_eq!(
            unknown_graphics_function("comando_inexistente"),
            "função gráfica desconhecida: comando_inexistente"
        );
    }
}
