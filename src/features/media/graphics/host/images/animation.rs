use super::*;

impl GraphicsHost {
    pub(super) fn animation_operation(
        &mut self,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<Option<HostValue>, String> {
        match function {
            "proximo_frame_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let mut state = self.state.borrow_mut();
                let image = get_gif_mut(&mut state, handle)?;
                image.current_frame = (image.current_frame + 1) % image.frames.len();
                Ok(Some(HostValue::Null))
            }
            "obter_intervalo_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let state = self.state.borrow();
                let image = get_gif(&state, handle)?;
                Ok(Some(HostValue::Integer(
                    image.frame_delays[image.current_frame],
                )))
            }
            "obter_numero_quadros_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let state = self.state.borrow();
                let image = get_gif(&state, handle)?;
                Ok(Some(HostValue::Integer(image.frames.len() as i64)))
            }
            "obter_numero_quadro_atual_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let state = self.state.borrow();
                let image = get_gif(&state, handle)?;
                Ok(Some(HostValue::Integer(image.current_frame as i64)))
            }
            "obter_quadro_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let frame = usize::try_from(integer_argument(arguments, 1)?)
                    .map_err(|_| "quadro de animação inválido".to_owned())?;
                let pixbuf = {
                    let state = self.state.borrow();
                    let gif = get_gif(&state, handle)?;
                    validate_animation_frame(gif, frame)?;
                    gif.frames[frame].clone()
                };
                Ok(Some(HostValue::Integer(
                    self.insert_image(ImageAsset::still(pixbuf))?,
                )))
            }
            "definir_quadro_gif" => {
                let handle = integer_argument(arguments, 0)?;
                let frame = usize::try_from(integer_argument(arguments, 1)?)
                    .map_err(|_| "quadro de animação inválido".to_owned())?;
                let mut state = self.state.borrow_mut();
                let gif = get_gif_mut(&mut state, handle)?;
                validate_animation_frame(gif, frame)?;
                gif.current_frame = frame;
                Ok(Some(HostValue::Null))
            }
            _ => Ok(None),
        }
    }
}

fn validate_animation_frame(image: &ImageAsset, frame: usize) -> Result<(), String> {
    if frame >= image.frames.len() {
        return Err("quadro de animação fora do limite".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_frame_validation_keeps_the_existing_bounds_error() {
        let frame =
            gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 1, 1).unwrap();
        let image = ImageAsset {
            frames: vec![frame],
            frame_delays: vec![100],
            current_frame: 0,
            is_gif: true,
        };

        assert_eq!(validate_animation_frame(&image, 0), Ok(()));
        assert_eq!(
            validate_animation_frame(&image, 1),
            Err("quadro de animação fora do limite".to_owned())
        );
    }
}
