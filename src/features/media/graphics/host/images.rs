use super::*;

impl GraphicsHost {
    pub(super) fn image_operation(
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

    pub(super) fn image_pixbuf(&self, handle: i64) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
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
}
