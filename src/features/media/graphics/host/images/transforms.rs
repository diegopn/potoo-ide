use super::*;

impl GraphicsHost {
    pub(super) fn transforms_operation(
        &mut self,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<Option<HostValue>, String> {
        match function {
            "transformar_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let horizontal = logical_argument(arguments, 1)?;
                let vertical = logical_argument(arguments, 2)?;
                let rotation = integer_argument(arguments, 3)?;
                let ignored_color = integer_argument(arguments, 4)?;
                let source = self.image_pixbuf(handle)?;
                let pixbuf =
                    transform_pixbuf(&source, horizontal, vertical, rotation, ignored_color)?;
                Ok(Some(HostValue::Integer(
                    self.insert_image(ImageAsset::still(pixbuf))?,
                )))
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
                Ok(Some(HostValue::Integer(self.insert_image(image)?)))
            }
            "obter_cor_pixel" => {
                let handle = integer_argument(arguments, 0)?;
                let x = i32::try_from(integer_argument(arguments, 1)?)
                    .map_err(|_| "coordenada X inválida".to_owned())?;
                let y = i32::try_from(integer_argument(arguments, 2)?)
                    .map_err(|_| "coordenada Y inválida".to_owned())?;
                let pixbuf = self.image_pixbuf(handle)?;
                Ok(Some(HostValue::Integer(pixbuf_pixel_color(&pixbuf, x, y)?)))
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
                Ok(Some(HostValue::Integer(
                    self.insert_image(ImageAsset::still(transformed))?,
                )))
            }
            _ => Ok(None),
        }
    }
}
