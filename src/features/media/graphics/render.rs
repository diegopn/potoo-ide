use super::*;

pub(super) fn ensure_surface(state: &mut GraphicsState) -> Result<(), String> {
    if state.surface.is_some() {
        return Ok(());
    }
    check_pixel_count(state.width, state.height)?;
    state.surface = Some(transparent_surface(state.width, state.height)?);
    Ok(())
}

pub(super) fn transparent_surface(
    width: i32,
    height: i32,
) -> Result<gtk::cairo::ImageSurface, String> {
    check_pixel_count(width, height)?;
    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
        .map_err(|error| format!("não foi possível criar a tela gráfica: {error}"))?;
    let context = gtk::cairo::Context::new(&surface)
        .map_err(|error| format!("não foi possível preparar a tela gráfica: {error}"))?;
    context.set_operator(gtk::cairo::Operator::Source);
    context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    context.paint().map_err(|error| error.to_string())?;
    Ok(surface)
}

pub(super) fn resize_graphics_state(
    state: &mut GraphicsState,
    width: i32,
    height: i32,
) -> Result<(), String> {
    check_pixel_count(width, height)?;
    let resize = |source: &Option<gtk::cairo::ImageSurface>| {
        let target = transparent_surface(width, height)?;
        if let Some(source) = source {
            let context = gtk::cairo::Context::new(&target).map_err(|error| error.to_string())?;
            context
                .set_source_surface(source, 0.0, 0.0)
                .map_err(|error| error.to_string())?;
            context.paint().map_err(|error| error.to_string())?;
        }
        Ok::<_, String>(target)
    };
    let surface = resize(&state.surface)?;
    let presented_surface = resize(&state.presented_surface)?;
    state.width = width;
    state.height = height;
    state.surface = Some(surface);
    state.presented_surface = Some(presented_surface);
    Ok(())
}

pub(super) fn present_frame(state: &mut GraphicsState) -> Result<(), String> {
    ensure_surface(state)?;
    state.presented_surface = state.surface.clone();
    state.surface = Some(transparent_surface(state.width, state.height)?);
    Ok(())
}

pub(super) fn capture_pending_pixbuf(
    pending: &gtk::cairo::ImageSurface,
    width: i32,
    height: i32,
) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
    surface_region_to_pixbuf(pending, width, height)
}

pub(super) fn draw_graphics_surface(
    context: &gtk::cairo::Context,
    surface: Option<&gtk::cairo::ImageSurface>,
    width: i32,
    height: i32,
) {
    context.set_source_rgb(0.0, 0.0, 0.0);
    let _ = context.paint();
    let Some(surface) = surface else {
        return;
    };
    if width <= 0 || height <= 0 {
        return;
    }
    let image_width = f64::from(surface.width());
    let image_height = f64::from(surface.height());
    let scale = (f64::from(width) / image_width).min(f64::from(height) / image_height);
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let left = (f64::from(width) - image_width * scale) / 2.0;
    let top = (f64::from(height) - image_height * scale) / 2.0;
    let _ = context.save();
    context.translate(left, top);
    context.scale(scale, scale);
    let _ = context.set_source_surface(surface, 0.0, 0.0);
    let _ = context.paint();
    let _ = context.restore();
}

pub(super) fn preview_aspect_frame(area: &gtk::DrawingArea) -> Option<gtk::AspectFrame> {
    area.parent()?.downcast().ok()
}

pub(super) fn graphics_is_open(state: &GraphicsState) -> bool {
    state.preview_active
        || state
            .window
            .as_ref()
            .is_some_and(gtk::prelude::WidgetExt::is_visible)
}

pub(super) fn requires_initialized_graphics(function: &str) -> bool {
    matches!(
        function,
        "fechar_janela"
            | "minimizar_janela"
            | "restaurar_janela"
            | "ocultar_borda_janela"
            | "exibir_borda_janela"
            | "encerrar_modo_grafico"
            | "definir_dimensoes_janela"
            | "definir_titulo_janela"
            | "limpar"
            | "renderizar"
            | "definir_fonte_texto"
            | "definir_tamanho_texto"
            | "definir_estilo_texto"
            | "largura_janela"
            | "altura_janela"
            | "entrar_modo_tela_cheia"
            | "sair_modo_tela_cheia"
            | "desenhar_ponto"
            | "desenhar_linha"
            | "desenhar_retangulo"
            | "desenhar_elipse"
            | "desenhar_poligono"
            | "desenhar_texto"
            | "jerimum_desenhar_texto"
            | "largura_texto"
            | "altura_texto"
            | "carregar_fonte"
            | "definir_icone_janela"
            | "desenhar_imagem"
            | "jerimum_desenhar_imagem"
            | "desenhar_quadro_atual_gif"
            | "desenhar_porcao_imagem"
    )
}
