use super::*;

impl GraphicsHost {
    pub(super) fn assets_operation(
        &mut self,
        root: Option<&Path>,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<Option<HostValue>, String> {
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
                Ok(Some(HostValue::Integer(handle)))
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
                Ok(Some(HostValue::Integer(self.insert_image(image)?)))
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
                Ok(Some(HostValue::Null))
            }
            "liberar_imagem" => {
                let handle = integer_argument(arguments, 0)?;
                let mut state = self.state.borrow_mut();
                if let Some(image) = state.images.remove(&handle) {
                    state.image_bytes = state
                        .image_bytes
                        .saturating_sub(image_asset_memory_bytes(&image).unwrap_or(0));
                }
                Ok(Some(HostValue::Null))
            }
            _ => Ok(None),
        }
    }
}

impl GraphicsHost {
    pub(in crate::features::media::graphics::host) fn image_pixbuf(
        &self,
        handle: i64,
    ) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
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

    pub(super) fn insert_image(&self, image: ImageAsset) -> Result<i64, String> {
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
}
