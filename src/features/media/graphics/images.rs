use super::*;

pub(super) fn image_asset_memory_bytes(image: &ImageAsset) -> Result<usize, String> {
    image.frames.iter().try_fold(0_usize, |total, frame| {
        let rowstride = usize::try_from(frame.rowstride())
            .map_err(|_| "dimensão de imagem inválida".to_owned())?;
        let height = usize::try_from(frame.height())
            .map_err(|_| "dimensão de imagem inválida".to_owned())?;
        total
            .checked_add(
                rowstride
                    .checked_mul(height)
                    .ok_or_else(|| "uso de memória das imagens excede o limite".to_owned())?,
            )
            .ok_or_else(|| "uso de memória das imagens excede o limite".to_owned())
    })
}

pub(super) fn checked_image_memory(current: usize, additional: usize) -> Result<usize, String> {
    let total = current
        .checked_add(additional)
        .filter(|total| *total <= MAX_IMAGE_MEMORY_BYTES)
        .ok_or_else(|| "o limite agregado de 128 MiB para imagens foi atingido".to_owned())?;
    Ok(total)
}

pub(super) fn set_window_icon(
    window: &gtk::Window,
    pixbuf: &gtk::gdk_pixbuf::Pixbuf,
) -> Result<(), String> {
    let surface = window
        .surface()
        .ok_or_else(|| "a janela gráfica ainda não tem uma superfície".to_owned())?;
    let toplevel = surface
        .downcast::<gtk::gdk::Toplevel>()
        .map_err(|_| "a plataforma não permite definir o ícone desta janela".to_owned())?;
    let mut icons = Vec::new();
    for size in [16, 32, 48, 64, 128, 256] {
        let scale = (f64::from(size) / f64::from(pixbuf.width()))
            .min(f64::from(size) / f64::from(pixbuf.height()));
        let width = (f64::from(pixbuf.width()) * scale).round().max(1.0) as i32;
        let height = (f64::from(pixbuf.height()) * scale).round().max(1.0) as i32;
        let scaled = if (width, height) == (pixbuf.width(), pixbuf.height()) {
            pixbuf.clone()
        } else {
            pixbuf
                .scale_simple(width, height, gtk::gdk_pixbuf::InterpType::Bilinear)
                .ok_or_else(|| "não foi possível redimensionar o ícone da janela".to_owned())?
        };
        let rgba = apply_chroma_key(&scaled, 0)?;
        let bytes = rgba.read_pixel_bytes();
        icons.push(
            gtk::gdk::MemoryTexture::new(
                rgba.width(),
                rgba.height(),
                gtk::gdk::MemoryFormat::R8g8b8a8,
                &bytes,
                rgba.rowstride() as usize,
            )
            .upcast::<gtk::gdk::Texture>(),
        );
    }
    toplevel.set_icon_list(&icons);
    Ok(())
}

pub(super) fn resize_image_asset(
    source: &ImageAsset,
    width: i32,
    height: i32,
    interpolation: gtk::gdk_pixbuf::InterpType,
) -> Result<ImageAsset, String> {
    check_pixel_count(width, height)?;
    if i64::from(width) * i64::from(height) * source.frames.len() as i64 > 32 * 1024 * 1024 {
        return Err("a animação GIF excede o limite total de pixels".to_owned());
    }
    let frames = source
        .frames
        .iter()
        .map(|frame| {
            frame
                .scale_simple(width, height, interpolation)
                .ok_or_else(|| "não foi possível redimensionar a imagem".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ImageAsset {
        frames,
        frame_delays: source.frame_delays.clone(),
        current_frame: source.current_frame,
        is_gif: source.is_gif,
    })
}

pub(super) fn load_image_asset(path: &Path) -> Result<ImageAsset, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("não foi possível ler a imagem: {error}"))?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("a imagem excede o limite de 16 MiB".to_owned());
    }
    let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");
    if is_gif {
        let delays = gif_frame_delays(&bytes)?;
        if delays.is_empty() || delays.len() > 500 {
            return Err("a animação GIF não contém quadros válidos".to_owned());
        }
        let header_width = i32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
        let header_height = i32::from(u16::from_le_bytes([bytes[8], bytes[9]]));
        if !(1..=8192).contains(&header_width)
            || !(1..=8192).contains(&header_height)
            || i64::from(header_width) * i64::from(header_height) * delays.len() as i64
                > 32 * 1024 * 1024
        {
            return Err("a animação GIF excede os limites de dimensão".to_owned());
        }
        let animation = gtk::gdk_pixbuf::PixbufAnimation::from_file(path)
            .map_err(|error| format!("não foi possível decodificar a animação GIF: {error}"))?;
        let width = animation.width();
        let height = animation.height();
        if !(1..=8192).contains(&width) || !(1..=8192).contains(&height) {
            return Err("as dimensões da animação excedem o limite de 8192 pixels".to_owned());
        }
        let total_pixels = i64::from(width) * i64::from(height) * delays.len() as i64;
        if total_pixels > 32 * 1024 * 1024 {
            return Err("a animação GIF excede o limite total de pixels".to_owned());
        }
        let start = SystemTime::now();
        let iterator = animation.iter(Some(start));
        let mut frames = Vec::with_capacity(delays.len());
        let mut elapsed = Duration::ZERO;
        for (index, delay) in delays.iter().enumerate() {
            let frame = iterator.pixbuf();
            frames.push(frame);
            if index + 1 < delays.len() {
                elapsed += Duration::from_millis((*delay).max(10) as u64);
                iterator.advance(start + elapsed);
            }
        }
        return Ok(ImageAsset {
            frames,
            frame_delays: delays,
            current_frame: 0,
            is_gif: true,
        });
    }

    let (_, width, height) = gtk::gdk_pixbuf::Pixbuf::file_info(path)
        .ok_or_else(|| "formato de imagem não suportado".to_owned())?;
    if !(1..=8192).contains(&width)
        || !(1..=8192).contains(&height)
        || i64::from(width) * i64::from(height) > 32 * 1024 * 1024
    {
        return Err("as dimensões da imagem excedem os limites suportados".to_owned());
    }
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_file(path)
        .map_err(|error| format!("não foi possível decodificar a imagem: {error}"))?;
    if !(1..=8192).contains(&pixbuf.width()) || !(1..=8192).contains(&pixbuf.height()) {
        return Err("as dimensões da imagem excedem o limite de 8192 pixels".to_owned());
    }
    Ok(ImageAsset::still(pixbuf))
}

pub(super) fn get_gif(state: &GraphicsState, handle: i64) -> Result<&ImageAsset, String> {
    let image = state
        .images
        .get(&handle)
        .ok_or_else(|| format!("endereço de imagem inválido ({handle})"))?;
    if !image.is_gif {
        return Err(format!("a imagem ({handle}) não é uma animação GIF"));
    }
    Ok(image)
}

pub(super) fn get_gif_mut(
    state: &mut GraphicsState,
    handle: i64,
) -> Result<&mut ImageAsset, String> {
    let image = state
        .images
        .get_mut(&handle)
        .ok_or_else(|| format!("endereço de imagem inválido ({handle})"))?;
    if !image.is_gif {
        return Err(format!("a imagem ({handle}) não é uma animação GIF"));
    }
    Ok(image)
}

pub(super) fn validate_crop(
    image: &gtk::gdk_pixbuf::Pixbuf,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    if x < 0
        || y < 0
        || width <= 0
        || height <= 0
        || x.checked_add(width).is_none_or(|end| end > image.width())
        || y.checked_add(height).is_none_or(|end| end > image.height())
    {
        return Err("o recorte está fora dos limites da imagem".to_owned());
    }
    Ok(())
}

pub(super) fn pixbuf_pixel_color(
    pixbuf: &gtk::gdk_pixbuf::Pixbuf,
    x: i32,
    y: i32,
) -> Result<i64, String> {
    if x < 0 || y < 0 || x >= pixbuf.width() || y >= pixbuf.height() {
        return Err("a coordenada do pixel está fora da imagem".to_owned());
    }
    let bytes = pixbuf.read_pixel_bytes();
    let data = bytes.as_ref();
    let channels = usize::try_from(pixbuf.n_channels())
        .map_err(|_| "formato de imagem inválido".to_owned())?;
    if channels < 3 {
        return Err("formato de imagem inválido".to_owned());
    }
    let index = usize::try_from(y).unwrap() * pixbuf.rowstride() as usize
        + usize::try_from(x).unwrap() * channels;
    let pixel = data
        .get(index..index + channels)
        .ok_or_else(|| "dados de pixel incompletos".to_owned())?;
    let alpha = if pixbuf.has_alpha() { pixel[3] } else { 255 };
    Ok(i64::from(
        u32::from_be_bytes([alpha, pixel[0], pixel[1], pixel[2]]) as i32,
    ))
}

pub(super) fn surface_to_pixbuf(
    surface: &gtk::cairo::ImageSurface,
) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
    surface_region_to_pixbuf(surface, surface.width(), surface.height())
}

pub(super) fn surface_region_to_pixbuf(
    surface: &gtk::cairo::ImageSurface,
    width: i32,
    height: i32,
) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
    if surface.format() != gtk::cairo::Format::ARgb32 {
        return Err("formato de captura gráfica não suportado".to_owned());
    }
    check_pixel_count(width, height)?;
    let source_width = surface.width().min(width);
    let source_height = surface.height().min(height);
    let stride = usize::try_from(surface.stride()).map_err(|_| "superfície inválida".to_owned())?;
    let mut rgba = vec![0; width as usize * height as usize * 4];
    surface
        .with_data(|source| {
            for y in 0..source_height as usize {
                for x in 0..source_width as usize {
                    let start = y * stride + x * 4;
                    let pixel = u32::from_ne_bytes(source[start..start + 4].try_into().unwrap());
                    let alpha = (pixel >> 24) as u8;
                    let unpremultiply = |channel: u32| -> u8 {
                        if alpha == 0 {
                            0
                        } else {
                            (((channel * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255))
                                as u8
                        }
                    };
                    let target = (y * width as usize + x) * 4;
                    rgba[target] = unpremultiply((pixel >> 16) & 0xff);
                    rgba[target + 1] = unpremultiply((pixel >> 8) & 0xff);
                    rgba[target + 2] = unpremultiply(pixel & 0xff);
                    rgba[target + 3] = alpha;
                }
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(gtk::gdk_pixbuf::Pixbuf::from_mut_slice(
        rgba,
        gtk::gdk_pixbuf::Colorspace::Rgb,
        true,
        8,
        width,
        height,
        width * 4,
    ))
}

pub(super) fn apply_chroma_key(
    source: &gtk::gdk_pixbuf::Pixbuf,
    ignored_color: i64,
) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
    let width = source.width();
    let height = source.height();
    check_pixel_count(width, height)?;
    let channels = usize::try_from(source.n_channels())
        .map_err(|_| "formato de imagem inválido".to_owned())?;
    if channels < 3 {
        return Err("formato de imagem inválido".to_owned());
    }
    let rowstride = source.rowstride() as usize;
    let input = source.read_pixel_bytes();
    let input = input.as_ref();
    let ignored = ignored_color as u32;
    let ignored_rgb = [(ignored >> 16) as u8, (ignored >> 8) as u8, ignored as u8];
    let mut output = vec![0; width as usize * height as usize * 4];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let from = y * rowstride + x * channels;
            let to = (y * width as usize + x) * 4;
            let rgb = [input[from], input[from + 1], input[from + 2]];
            let ignored = ignored_color != 0 && rgb == ignored_rgb;
            output[to..to + 3].copy_from_slice(&rgb);
            output[to + 3] = if ignored {
                0
            } else if source.has_alpha() {
                input[from + 3]
            } else {
                255
            };
            if ignored {
                output[to..to + 3].fill(0);
            }
        }
    }
    Ok(gtk::gdk_pixbuf::Pixbuf::from_mut_slice(
        output,
        gtk::gdk_pixbuf::Colorspace::Rgb,
        true,
        8,
        width,
        height,
        width * 4,
    ))
}

pub(super) fn transform_pixbuf(
    source: &gtk::gdk_pixbuf::Pixbuf,
    horizontal: bool,
    vertical: bool,
    rotation: i64,
    ignored_color: i64,
) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
    let mut image = if ignored_color == 0 {
        source.clone()
    } else {
        apply_chroma_key(source, ignored_color)?
    };
    if horizontal {
        image = image
            .flip(true)
            .ok_or_else(|| "não foi possível espelhar a imagem".to_owned())?;
    }
    if vertical {
        image = image
            .flip(false)
            .ok_or_else(|| "não foi possível espelhar a imagem".to_owned())?;
    }
    let degrees = rotation.rem_euclid(360);
    if degrees == 0 {
        return Ok(image);
    }

    let angle = (degrees as f64).to_radians();
    let (sin, cos) = angle.sin_cos();
    let extent = |value: f64| {
        let nearest = value.round();
        if (value - nearest).abs() < 1e-10 {
            nearest as i64
        } else {
            value.ceil() as i64
        }
    };
    let width =
        extent(f64::from(image.width()) * cos.abs() + f64::from(image.height()) * sin.abs());
    let height =
        extent(f64::from(image.width()) * sin.abs() + f64::from(image.height()) * cos.abs());
    let width = checked_dimension(width)?;
    let height = checked_dimension(height)?;
    check_pixel_count(width, height)?;
    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
        .map_err(|error| format!("não foi possível rotacionar a imagem: {error}"))?;
    let context = gtk::cairo::Context::new(&surface).map_err(|error| error.to_string())?;
    context.set_operator(gtk::cairo::Operator::Source);
    context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    context.paint().map_err(|error| error.to_string())?;
    context.translate(f64::from(width) / 2.0, f64::from(height) / 2.0);
    context.rotate(angle);
    context.translate(
        -f64::from(image.width()) / 2.0,
        -f64::from(image.height()) / 2.0,
    );
    context.set_source_pixbuf(&image, 0.0, 0.0);
    context.paint().map_err(|error| error.to_string())?;
    surface_to_pixbuf(&surface)
}
