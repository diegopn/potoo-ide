use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime};

use gtk::gdk::prelude::*;
use gtk::prelude::*;
use potoo_portugol_interpreter::HostValue;

use crate::execution::HostRequest;

type HostReply = Sender<Result<HostValue, String>>;

struct ImageAsset {
    frames: Vec<gtk::gdk_pixbuf::Pixbuf>,
    frame_delays: Vec<i64>,
    current_frame: usize,
    is_gif: bool,
}

impl ImageAsset {
    fn still(pixbuf: gtk::gdk_pixbuf::Pixbuf) -> Self {
        Self {
            frames: vec![pixbuf],
            frame_delays: vec![100],
            current_frame: 0,
            is_gif: false,
        }
    }
}

struct GraphicsState {
    window: Option<gtk::Window>,
    area: Option<gtk::DrawingArea>,
    surface: Option<gtk::cairo::ImageSurface>,
    width: i32,
    height: i32,
    color: i64,
    gradient: Option<(i64, i64, i64)>,
    font: String,
    text_size: f64,
    italic: bool,
    bold: bool,
    underline: bool,
    rotation: i64,
    opacity: i64,
    icon: Option<gtk::gdk_pixbuf::Pixbuf>,
    images: HashMap<i64, ImageAsset>,
    next_image_handle: i64,
    pressed_keys: HashMap<u32, i64>,
    pressed_buttons: HashSet<i64>,
    mouse_x: i64,
    mouse_y: i64,
    cursor_visible: bool,
    pending_key_reads: Vec<HostReply>,
    pending_button_reads: Vec<HostReply>,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            window: None,
            area: None,
            surface: None,
            width: 640,
            height: 480,
            color: -1,
            gradient: None,
            font: "Sans".to_owned(),
            text_size: 14.0,
            italic: false,
            bold: false,
            underline: false,
            rotation: 0,
            opacity: 255,
            icon: None,
            images: HashMap::new(),
            next_image_handle: 1,
            pressed_keys: HashMap::new(),
            pressed_buttons: HashSet::new(),
            mouse_x: 0,
            mouse_y: 0,
            cursor_visible: true,
            pending_key_reads: Vec::new(),
            pending_button_reads: Vec::new(),
        }
    }
}

pub(crate) struct GraphicsHost {
    parent: gtk::ApplicationWindow,
    state: Rc<RefCell<GraphicsState>>,
    on_close: Rc<dyn Fn()>,
}

impl GraphicsHost {
    pub(crate) fn new(parent: gtk::ApplicationWindow, on_close: impl Fn() + 'static) -> Self {
        Self {
            parent,
            state: Rc::new(RefCell::new(GraphicsState::default())),
            on_close: Rc::new(on_close),
        }
    }

    pub(crate) fn handle_input_request(
        &mut self,
        request: HostRequest,
        response: HostReply,
    ) -> Result<Option<HostValue>, String> {
        let mut state = self.state.borrow_mut();
        let is_mouse_or_keyboard = matches!(
            request,
            HostRequest::KeyboardKeyPressed(_)
                | HostRequest::KeyboardAnyKeyPressed
                | HostRequest::KeyboardReadKey
                | HostRequest::MouseButtonPressed(_)
                | HostRequest::MouseAnyButtonPressed
                | HostRequest::MouseReadButton
                | HostRequest::MousePosition
                | HostRequest::MouseSetCursorVisible(_)
        );
        if is_mouse_or_keyboard
            && !state
                .window
                .as_ref()
                .is_some_and(gtk::prelude::WidgetExt::is_visible)
        {
            return Err("inicie o modo gráfico antes de usar teclado ou mouse".to_owned());
        }
        let value = match request {
            HostRequest::KeyboardKeyPressed(key) => {
                HostValue::Logical(state.pressed_keys.values().any(|pressed| *pressed == key))
            }
            HostRequest::KeyboardAnyKeyPressed => {
                HostValue::Logical(!state.pressed_keys.is_empty())
            }
            HostRequest::KeyboardReadKey => {
                state.pending_key_reads.push(response);
                return Ok(None);
            }
            HostRequest::MouseButtonPressed(button) => {
                HostValue::Logical(state.pressed_buttons.contains(&button))
            }
            HostRequest::MouseAnyButtonPressed => {
                HostValue::Logical(!state.pressed_buttons.is_empty())
            }
            HostRequest::MouseReadButton => {
                state.pending_button_reads.push(response);
                return Ok(None);
            }
            HostRequest::MousePosition => HostValue::Array(vec![
                HostValue::Integer(state.mouse_x),
                HostValue::Integer(state.mouse_y),
            ]),
            HostRequest::MouseSetCursorVisible(visible) => {
                state.cursor_visible = visible;
                if let Some(area) = &state.area {
                    if visible {
                        area.set_cursor(None);
                    } else if let Some(cursor) = gtk::gdk::Cursor::from_name("none", None) {
                        area.set_cursor(Some(&cursor));
                    }
                }
                HostValue::Null
            }
            HostRequest::Graphics { .. } | HostRequest::Audio { .. } => {
                return Err("requisição incompatível com o serviço de entrada".to_owned());
            }
        };
        Ok(Some(value))
    }

    pub(crate) fn finish(&mut self) {
        let mut state = self.state.borrow_mut();
        for response in state.pending_key_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
        for response in state.pending_button_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
        let window = state.window.take();
        *state = GraphicsState::default();
        drop(state);
        if let Some(window) = window {
            window.close();
        }
    }

    pub(crate) fn graphics(
        &mut self,
        root: Option<&Path>,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        match function {
            "iniciar_modo_grafico" => {
                let _keep_visible = logical_argument(arguments, 0)?;
                self.ensure_window("Portugol")?;
                return Ok(HostValue::Null);
            }
            "fechar_janela" => {
                let window = self.state.borrow().window.clone();
                if let Some(window) = window {
                    window.close();
                }
                return Ok(HostValue::Null);
            }
            "minimizar_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.minimize();
                }
                return Ok(HostValue::Null);
            }
            "restaurar_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.unminimize();
                    window.present();
                }
                return Ok(HostValue::Null);
            }
            "ocultar_borda_janela" | "exibir_borda_janela" => {
                if let Some(window) = &self.state.borrow().window {
                    window.set_decorated(function == "exibir_borda_janela");
                }
                return Ok(HostValue::Null);
            }
            "encerrar_modo_grafico" => {
                let window = self.state.borrow().window.clone();
                if let Some(window) = window {
                    window.set_visible(false);
                }
                return Ok(HostValue::Null);
            }
            "definir_dimensoes_janela" => {
                let width = checked_dimension(integer_argument(arguments, 0)?)?;
                let height = checked_dimension(integer_argument(arguments, 1)?)?;
                self.resize(width, height)?;
                return Ok(HostValue::Null);
            }
            "definir_titulo_janela" => {
                let title = string_argument(arguments, 0)?;
                if let Some(window) = &self.state.borrow().window {
                    window.set_title(Some(&title));
                }
                return Ok(HostValue::Null);
            }
            "limpar" => {
                self.clear_surface()?;
                return Ok(HostValue::Null);
            }
            "renderizar" => {
                self.queue_draw();
                return Ok(HostValue::Null);
            }
            "definir_cor" => {
                let mut state = self.state.borrow_mut();
                state.color = integer_argument(arguments, 0)?;
                state.gradient = None;
                return Ok(HostValue::Null);
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
                return Ok(HostValue::Null);
            }
            "definir_fonte_texto" => {
                self.state.borrow_mut().font = string_argument(arguments, 0)?;
                return Ok(HostValue::Null);
            }
            "definir_tamanho_texto" => {
                let size = real_argument(arguments, 0)?;
                if !size.is_finite() || size <= 0.0 || size > 512.0 {
                    return Err("o tamanho do texto deve estar entre 0 e 512".to_owned());
                }
                self.state.borrow_mut().text_size = size;
                return Ok(HostValue::Null);
            }
            "definir_estilo_texto" => {
                let mut state = self.state.borrow_mut();
                state.italic = logical_argument(arguments, 0)?;
                state.bold = logical_argument(arguments, 1)?;
                state.underline = logical_argument(arguments, 2)?;
                return Ok(HostValue::Null);
            }
            "definir_opacidade" => {
                let opacity = integer_argument(arguments, 0)?;
                if !(0..=255).contains(&opacity) {
                    return Err("o valor da opacidade deve estar entre 0 e 255".to_owned());
                }
                self.state.borrow_mut().opacity = opacity;
                return Ok(HostValue::Null);
            }
            "definir_rotacao" => {
                self.state.borrow_mut().rotation = integer_argument(arguments, 0)?;
                return Ok(HostValue::Null);
            }
            "largura_janela" => {
                return Ok(HostValue::Integer(i64::from(self.state.borrow().width)));
            }
            "altura_janela" => {
                return Ok(HostValue::Integer(i64::from(self.state.borrow().height)));
            }
            "largura_tela" | "altura_tela" => {
                let display = gtk::gdk::Display::default()
                    .ok_or_else(|| "não há uma tela gráfica disponível".to_owned())?;
                let monitor = display
                    .monitors()
                    .item(0)
                    .and_downcast::<gtk::gdk::Monitor>()
                    .ok_or_else(|| "não há um monitor disponível".to_owned())?;
                let geometry = monitor.geometry();
                let dimension = if function == "largura_tela" {
                    geometry.width()
                } else {
                    geometry.height()
                };
                return Ok(HostValue::Integer(i64::from(dimension)));
            }
            "entrar_modo_tela_cheia" => {
                if let Some(window) = &self.state.borrow().window {
                    window.fullscreen();
                }
                return Ok(HostValue::Null);
            }
            "sair_modo_tela_cheia" => {
                if let Some(window) = &self.state.borrow().window {
                    window.unfullscreen();
                }
                return Ok(HostValue::Null);
            }
            "desenhar_ponto" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state)?;
                    context.rectangle(x, y, 1.0, 1.0);
                    context.fill().map_err(|error| error.to_string())
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_linha" => {
                let x1 = real_argument(arguments, 0)?;
                let y1 = real_argument(arguments, 1)?;
                let x2 = real_argument(arguments, 2)?;
                let y2 = real_argument(arguments, 3)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state)?;
                    context.move_to(x1, y1);
                    context.line_to(x2, y2);
                    context.stroke().map_err(|error| error.to_string())
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_retangulo" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let width = real_argument(arguments, 2)?;
                let height = real_argument(arguments, 3)?;
                let rounded = logical_argument(arguments, 4)?;
                let filled = logical_argument(arguments, 5)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state)?;
                    if rounded {
                        rounded_rectangle(context, x, y, width, height, 8.0);
                    } else {
                        context.rectangle(x, y, width, height);
                    }
                    if filled {
                        context.fill().map_err(|error| error.to_string())
                    } else {
                        context.stroke().map_err(|error| error.to_string())
                    }
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_elipse" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let width = real_argument(arguments, 2)?;
                let height = real_argument(arguments, 3)?;
                let filled = logical_argument(arguments, 4)?;
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state)?;
                    context.save().map_err(|error| error.to_string())?;
                    context.translate(x + width / 2.0, y + height / 2.0);
                    context.scale(width / 2.0, height / 2.0);
                    context.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
                    let result = if filled {
                        context.fill().map_err(|error| error.to_string())
                    } else {
                        context.stroke().map_err(|error| error.to_string())
                    };
                    context.restore().map_err(|error| error.to_string())?;
                    result
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_poligono" => {
                let points = point_array_argument(arguments, 0)?;
                let filled = logical_argument(arguments, 1)?;
                if points.len() < 2 {
                    return Err("um polígono precisa de pelo menos dois pontos".to_owned());
                }
                self.with_draw_context(|context, state| {
                    set_graphics_source(context, state)?;
                    context.move_to(points[0].0, points[0].1);
                    for (x, y) in points.iter().skip(1) {
                        context.line_to(*x, *y);
                    }
                    if filled {
                        context.close_path();
                        context.fill().map_err(|error| error.to_string())
                    } else {
                        context.stroke().map_err(|error| error.to_string())
                    }
                })?;
                return Ok(HostValue::Null);
            }
            "desenhar_texto" => {
                let x = real_argument(arguments, 0)?;
                let y = real_argument(arguments, 1)?;
                let text = string_argument(arguments, 2)?;
                self.draw_text(x, y, &text)?;
                return Ok(HostValue::Null);
            }
            "largura_texto" | "altura_texto" => {
                let text = string_argument(arguments, 0)?;
                let context = self.measure_context()?;
                let value = if function == "largura_texto" {
                    context
                        .text_extents(&text)
                        .map_err(|error| error.to_string())?
                        .width()
                } else {
                    context
                        .font_extents()
                        .map_err(|error| error.to_string())?
                        .height()
                };
                return Ok(HostValue::Integer(value.round() as i64));
            }
            "criar_cor" => {
                let red = color_channel(integer_argument(arguments, 0)?)?;
                let green = color_channel(integer_argument(arguments, 1)?)?;
                let blue = color_channel(integer_argument(arguments, 2)?)?;
                return Ok(HostValue::Integer(i64::from(
                    u32::from_be_bytes([255, red, green, blue]) as i32,
                )));
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
                return Ok(HostValue::Integer(i64::from(value)));
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
                return Ok(HostValue::Null);
            }
            "definir_icone_janela" => {
                let handle = integer_argument(arguments, 0)?;
                let pixbuf = self.image_pixbuf(handle)?;
                let window = self.state.borrow().window.clone();
                self.state.borrow_mut().icon = Some(pixbuf.clone());
                if let Some(window) = window {
                    set_window_icon(&window, &pixbuf)?;
                }
                return Ok(HostValue::Null);
            }
            _ => {}
        }

        self.image_operation(root, function, arguments)
    }

    fn resize(&self, width: i32, height: i32) -> Result<(), String> {
        check_pixel_count(width, height)?;
        let mut state = self.state.borrow_mut();
        let previous = state.surface.clone();
        let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
            .map_err(|error| {
            format!("não foi possível redimensionar a tela gráfica: {error}")
        })?;
        let context = gtk::cairo::Context::new(&surface).map_err(|error| error.to_string())?;
        context.set_source_rgb(0.0, 0.0, 0.0);
        context.paint().map_err(|error| error.to_string())?;
        context.set_operator(gtk::cairo::Operator::Source);
        if let Some(previous) = previous {
            context
                .set_source_surface(&previous, 0.0, 0.0)
                .map_err(|e| e.to_string())?;
            context.paint().map_err(|error| error.to_string())?;
        }
        state.width = width;
        state.height = height;
        state.surface = Some(surface);
        if let Some(area) = &state.area {
            area.set_content_width(width);
            area.set_content_height(height);
        }
        if let Some(window) = &state.window {
            window.set_default_size(width, height);
        }
        drop(state);
        self.queue_draw();
        Ok(())
    }

    fn clear_surface(&self) -> Result<(), String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let surface = state.surface.as_ref().unwrap();
        let context = gtk::cairo::Context::new(surface).map_err(|error| error.to_string())?;
        context.set_operator(gtk::cairo::Operator::Source);
        let (red, green, blue, _) = rgba(state.color, 1.0);
        context.set_source_rgb(red, green, blue);
        context.paint().map_err(|error| error.to_string())?;
        surface.flush();
        drop(state);
        self.queue_draw();
        Ok(())
    }

    fn queue_draw(&self) {
        if let Some(area) = &self.state.borrow().area {
            area.queue_draw();
        }
    }

    fn with_draw_context(
        &self,
        draw: impl FnOnce(&gtk::cairo::Context, &GraphicsState) -> Result<(), String>,
    ) -> Result<(), String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let surface = state.surface.as_ref().unwrap();
        let context = gtk::cairo::Context::new(surface).map_err(|error| error.to_string())?;
        if state.rotation.rem_euclid(360) != 0 {
            context.translate(f64::from(state.width) / 2.0, f64::from(state.height) / 2.0);
            context.rotate((state.rotation as f64).to_radians());
            context.translate(
                -f64::from(state.width) / 2.0,
                -f64::from(state.height) / 2.0,
            );
        }
        draw(&context, &state)?;
        surface.flush();
        drop(state);
        self.queue_draw();
        Ok(())
    }

    fn draw_text(&self, x: f64, y: f64, text: &str) -> Result<(), String> {
        self.with_draw_context(|context, state| {
            set_graphics_source(context, state)?;
            select_font(context, state);
            context.move_to(x, y + state.text_size);
            context.show_text(text).map_err(|error| error.to_string())?;
            if state.underline {
                let width = context
                    .text_extents(text)
                    .map_err(|error| error.to_string())?
                    .width();
                context.move_to(x, y + state.text_size + 2.0);
                context.line_to(x + width, y + state.text_size + 2.0);
                context.stroke().map_err(|error| error.to_string())?;
            }
            Ok(())
        })
    }

    fn measure_context(&self) -> Result<gtk::cairo::Context, String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let context = gtk::cairo::Context::new(state.surface.as_ref().unwrap())
            .map_err(|error| error.to_string())?;
        select_font(&context, &state);
        Ok(context)
    }

    fn image_operation(
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
                check_pixel_count(width, height)?;
                let surface = self.state.borrow().surface.as_ref().unwrap().clone();
                let mut pixbuf = surface_to_pixbuf(&surface)?;
                if width != pixbuf.width() || height != pixbuf.height() {
                    pixbuf = pixbuf
                        .scale_simple(width, height, gtk::gdk_pixbuf::InterpType::Bilinear)
                        .ok_or_else(|| "não foi possível redimensionar a captura".to_owned())?;
                }
                Ok(HostValue::Integer(
                    self.insert_image(ImageAsset::still(pixbuf))?,
                ))
            }
            "carregar_imagem" => {
                let path = safe_project_file(root, &string_argument(arguments, 0)?)?;
                let image = load_image_asset(&path)?;
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
                let pixbuf = source
                    .scale_simple(width, height, interpolation)
                    .ok_or_else(|| "não foi possível redimensionar a imagem".to_owned())?;
                Ok(HostValue::Integer(
                    self.insert_image(ImageAsset::still(pixbuf))?,
                ))
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
                self.state.borrow_mut().images.remove(&handle);
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
        let mut state = self.state.borrow_mut();
        let handle = state.next_image_handle;
        state.next_image_handle = state
            .next_image_handle
            .checked_add(1)
            .ok_or_else(|| "não há mais identificadores de imagem disponíveis".to_owned())?;
        state.images.insert(handle, image);
        Ok(handle)
    }

    fn image_pixbuf(&self, handle: i64) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
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
            context.set_source_pixbuf(pixbuf, x, y);
            context
                .paint_with_alpha(state.opacity as f64 / 255.0)
                .map_err(|error| error.to_string())
        })
    }

    fn ensure_surface(&self) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        ensure_surface(&mut state)
    }

    fn ensure_window(&self, title: &str) -> Result<(), String> {
        if self.state.borrow().window.is_some() {
            if let Some(window) = &self.state.borrow().window {
                window.present();
            }
            return Ok(());
        }

        self.ensure_surface()?;
        let area = gtk::DrawingArea::new();
        let (width, height) = {
            let state = self.state.borrow();
            (state.width, state.height)
        };
        let state_for_draw = self.state.clone();
        area.set_content_width(width);
        area.set_content_height(height);
        area.set_focusable(true);
        area.set_draw_func(move |_, context, _, _| {
            let state = state_for_draw.borrow();
            if let Some(surface) = &state.surface {
                let _ = context.set_source_surface(surface, 0.0, 0.0);
                let _ = context.paint();
            }
        });
        let window = gtk::Window::builder()
            .title(title)
            .default_width(width)
            .default_height(height)
            .transient_for(&self.parent)
            .destroy_with_parent(true)
            .child(&area)
            .build();

        install_input_controllers(&area, &window, self.state.clone());
        let state_for_close = self.state.clone();
        let on_close = self.on_close.clone();
        window.connect_close_request(move |window| {
            let mut state = state_for_close.borrow_mut();
            state.pressed_keys.clear();
            state.pressed_buttons.clear();
            for response in state.pending_key_reads.drain(..) {
                let _ = response.send(Ok(HostValue::Integer(-1)));
            }
            for response in state.pending_button_reads.drain(..) {
                let _ = response.send(Ok(HostValue::Integer(-1)));
            }
            window.set_visible(false);
            on_close();
            gtk::glib::Propagation::Stop
        });

        let mut state = self.state.borrow_mut();
        state.window = Some(window.clone());
        state.area = Some(area.clone());
        if let Some(surface) = &state.surface {
            let _ = surface.flush();
        }
        drop(state);
        window.present();
        area.grab_focus();
        if let Some(icon) = self.state.borrow().icon.clone() {
            set_window_icon(&window, &icon)?;
        }
        Ok(())
    }
}

fn ensure_surface(state: &mut GraphicsState) -> Result<(), String> {
    if state.surface.is_some() {
        return Ok(());
    }
    check_pixel_count(state.width, state.height)?;
    let surface =
        gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, state.width, state.height)
            .map_err(|error| format!("não foi possível criar a tela gráfica: {error}"))?;
    let context = gtk::cairo::Context::new(&surface)
        .map_err(|error| format!("não foi possível preparar a tela gráfica: {error}"))?;
    context.set_source_rgb(0.0, 0.0, 0.0);
    context.paint().map_err(|error| error.to_string())?;
    state.surface = Some(surface);
    Ok(())
}

fn set_window_icon(window: &gtk::Window, pixbuf: &gtk::gdk_pixbuf::Pixbuf) -> Result<(), String> {
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

fn install_input_controllers(
    area: &gtk::DrawingArea,
    window: &gtk::Window,
    state: Rc<RefCell<GraphicsState>>,
) {
    let keys = gtk::EventControllerKey::new();
    let key_state = state.clone();
    keys.connect_key_pressed(move |_, key, hardware_keycode, _| {
        if let Some(code) = java_key_code(key) {
            key_state
                .borrow_mut()
                .pressed_keys
                .insert(hardware_keycode, code);
        }
        gtk::glib::Propagation::Proceed
    });
    let key_state = state.clone();
    keys.connect_key_released(move |_, key, hardware_keycode, _| {
        if let Some(code) = java_key_code(key) {
            let mut state = key_state.borrow_mut();
            let code = state.pressed_keys.remove(&hardware_keycode).unwrap_or(code);
            let reads = std::mem::take(&mut state.pending_key_reads);
            drop(state);
            for response in reads {
                let _ = response.send(Ok(HostValue::Integer(code)));
            }
        }
    });
    area.add_controller(keys);

    let pointer = gtk::EventControllerMotion::new();
    let motion_state = state.clone();
    pointer.connect_motion(move |_, x, y| {
        let mut state = motion_state.borrow_mut();
        state.mouse_x = x.round() as i64;
        state.mouse_y = y.round() as i64;
    });
    area.add_controller(pointer);

    let click = gtk::GestureClick::new();
    click.set_button(0);
    let press_state = state.clone();
    click.connect_pressed(move |gesture, _, x, y| {
        if let Some(button) = portugol_mouse_button(gesture.current_button()) {
            let mut state = press_state.borrow_mut();
            state.pressed_buttons.insert(button);
            state.mouse_x = x.round() as i64;
            state.mouse_y = y.round() as i64;
        }
    });
    let release_state = state.clone();
    click.connect_released(move |gesture, _, x, y| {
        if let Some(button) = portugol_mouse_button(gesture.current_button()) {
            let mut state = release_state.borrow_mut();
            state.pressed_buttons.remove(&button);
            state.mouse_x = x.round() as i64;
            state.mouse_y = y.round() as i64;
            let reads = std::mem::take(&mut state.pending_button_reads);
            drop(state);
            for response in reads {
                let _ = response.send(Ok(HostValue::Integer(button)));
            }
        }
    });
    area.add_controller(click);

    let focus = gtk::EventControllerFocus::new();
    let focus_state = state;
    focus.connect_leave(move |_| {
        let mut state = focus_state.borrow_mut();
        state.pressed_keys.clear();
        state.pressed_buttons.clear();
        for response in state.pending_key_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
        for response in state.pending_button_reads.drain(..) {
            let _ = response.send(Ok(HostValue::Integer(-1)));
        }
    });
    area.add_controller(focus);

    let area_for_window = area.clone();
    window.connect_is_active_notify(move |window| {
        if window.is_active() {
            area_for_window.grab_focus();
        }
    });
}

fn portugol_mouse_button(button: u32) -> Option<i64> {
    match button {
        1 => Some(0),
        3 => Some(1),
        2 => Some(2),
        _ => None,
    }
}

fn java_key_code(key: gtk::gdk::Key) -> Option<i64> {
    let name = key.name()?.to_string();
    let simple = match name.as_str() {
        "Return" => 10,
        "KP_Enter" => 10,
        "BackSpace" => 8,
        "Tab" | "ISO_Left_Tab" => 9,
        "Cancel" => 3,
        "Clear" => 12,
        "Shift_L" | "Shift_R" => 16,
        "Control_L" | "Control_R" => 17,
        "Alt_L" | "Alt_R" => 18,
        "Pause" => 19,
        "Caps_Lock" => 20,
        "Escape" => 27,
        "space" => 32,
        "Page_Up" => 33,
        "Page_Down" => 34,
        "End" => 35,
        "Home" => 36,
        "Left" => 37,
        "Up" => 38,
        "Right" => 39,
        "Down" => 40,
        "comma" => 44,
        "minus" => 45,
        "period" => 46,
        "slash" => 47,
        "semicolon" => 59,
        "equal" => 61,
        "KP_Equal" => 61,
        "bracketleft" => 91,
        "backslash" => 92,
        "bracketright" => 93,
        "KP_0" => 96,
        "KP_1" => 97,
        "KP_2" => 98,
        "KP_3" => 99,
        "KP_4" => 100,
        "KP_5" => 101,
        "KP_6" => 102,
        "KP_7" => 103,
        "KP_8" => 104,
        "KP_9" => 105,
        "KP_Multiply" => 106,
        "KP_Add" => 107,
        "KP_Separator" => 108,
        "KP_Subtract" => 109,
        "KP_Decimal" => 110,
        "KP_Divide" => 111,
        "Delete" => 127,
        "Num_Lock" => 144,
        "Scroll_Lock" => 145,
        "Print" => 154,
        "Insert" => 155,
        "Help" => 156,
        "Meta_L" | "Meta_R" => 157,
        "grave" => 192,
        "apostrophe" => 222,
        "ampersand" => 150,
        "asterisk" => 151,
        "quotedbl" => 152,
        "less" => 153,
        "greater" => 160,
        "braceleft" => 161,
        "braceright" => 162,
        "at" => 512,
        "colon" => 513,
        "asciicircum" => 514,
        "dollar" => 515,
        "EuroSign" => 516,
        "exclam" => 517,
        "exclamdown" => 518,
        "parenleft" => 519,
        "numbersign" => 520,
        "plus" => 521,
        "parenright" => 522,
        "underscore" => 523,
        "Super_L" | "Super_R" => 524,
        "Menu" => 525,
        "F1" => 112,
        "F2" => 113,
        "F3" => 114,
        "F4" => 115,
        "F5" => 116,
        "F6" => 117,
        "F7" => 118,
        "F8" => 119,
        "F9" => 120,
        "F10" => 121,
        "F11" => 122,
        "F12" => 123,
        "F13" => 61_440,
        "F14" => 61_441,
        "F15" => 61_442,
        "F16" => 61_443,
        "F17" => 61_444,
        "F18" => 61_445,
        "F19" => 61_446,
        "F20" => 61_447,
        "F21" => 61_448,
        "F22" => 61_449,
        "F23" => 61_450,
        "F24" => 61_451,
        _ => {
            if name.len() == 1 {
                let character = name.chars().next()?;
                if character.is_ascii_alphabetic() {
                    return Some(i64::from(character.to_ascii_uppercase() as u8));
                }
                if character.is_ascii_digit() {
                    return Some(i64::from(character as u8));
                }
            }
            return Some(0);
        }
    };
    Some(simple)
}

fn integer_argument(arguments: &[HostValue], index: usize) -> Result<i64, String> {
    match arguments.get(index) {
        Some(HostValue::Integer(value)) => Ok(*value),
        _ => Err(format!("argumento {} deve ser inteiro", index + 1)),
    }
}

fn i32_argument(arguments: &[HostValue], index: usize, message: &str) -> Result<i32, String> {
    i32::try_from(integer_argument(arguments, index)?).map_err(|_| message.to_owned())
}

fn logical_argument(arguments: &[HostValue], index: usize) -> Result<bool, String> {
    match arguments.get(index) {
        Some(HostValue::Logical(value)) => Ok(*value),
        _ => Err(format!("argumento {} deve ser lógico", index + 1)),
    }
}

fn string_argument(arguments: &[HostValue], index: usize) -> Result<String, String> {
    match arguments.get(index) {
        Some(HostValue::String(value)) => Ok(value.clone()),
        Some(HostValue::Character(value)) => Ok(value.to_string()),
        _ => Err(format!(
            "argumento {} deve ser cadeia de caracteres",
            index + 1
        )),
    }
}

fn real_argument(arguments: &[HostValue], index: usize) -> Result<f64, String> {
    match arguments.get(index) {
        Some(HostValue::Real(value)) => Ok(*value),
        Some(HostValue::Integer(value)) => Ok(*value as f64),
        _ => Err(format!("argumento {} deve ser numérico", index + 1)),
    }
}

fn checked_dimension(value: i64) -> Result<i32, String> {
    if !(1..=8192).contains(&value) {
        return Err("a dimensão deve estar entre 1 e 8192 pixels".to_owned());
    }
    i32::try_from(value).map_err(|_| "dimensão inválida".to_owned())
}

fn check_pixel_count(width: i32, height: i32) -> Result<(), String> {
    if i64::from(width) * i64::from(height) > 32 * 1024 * 1024 {
        return Err("a imagem excede o limite de 32 milhões de pixels".to_owned());
    }
    Ok(())
}

fn color_channel(value: i64) -> Result<u8, String> {
    u8::try_from(value).map_err(|_| "cada canal de cor deve estar entre 0 e 255".to_owned())
}

fn point_array_argument(arguments: &[HostValue], index: usize) -> Result<Vec<(f64, f64)>, String> {
    let Some(HostValue::Array(points)) = arguments.get(index) else {
        return Err("os pontos do polígono devem formar uma matriz de inteiros".to_owned());
    };
    points
        .iter()
        .map(|point| {
            let HostValue::Array(coordinates) = point else {
                return Err("cada ponto do polígono deve conter X e Y".to_owned());
            };
            if coordinates.len() != 2 {
                return Err("cada ponto do polígono deve conter X e Y".to_owned());
            }
            match (&coordinates[0], &coordinates[1]) {
                (HostValue::Integer(x), HostValue::Integer(y)) => Ok((*x as f64, *y as f64)),
                _ => Err("as coordenadas do polígono devem ser inteiras".to_owned()),
            }
        })
        .collect()
}

fn set_graphics_source(context: &gtk::cairo::Context, state: &GraphicsState) -> Result<(), String> {
    let alpha_scale = state.opacity as f64 / 255.0;
    if let Some((kind, first, second)) = state.gradient {
        let width = f64::from(state.width);
        let height = f64::from(state.height);
        let (x1, y1, x2, y2) = match kind {
            0 => (0.0, 0.0, width, 0.0),
            1 => (width, 0.0, 0.0, 0.0),
            2 => (0.0, height, 0.0, 0.0),
            3 => (0.0, 0.0, 0.0, height),
            4 => (0.0, 0.0, width, height),
            5 => (width, 0.0, 0.0, height),
            6 => (0.0, height, width, 0.0),
            7 => (width, height, 0.0, 0.0),
            _ => return Err("tipo de gradiente inválido".to_owned()),
        };
        let gradient = gtk::cairo::LinearGradient::new(x1, y1, x2, y2);
        let (r1, g1, b1, a1) = rgba(first, alpha_scale);
        let (r2, g2, b2, a2) = rgba(second, alpha_scale);
        gradient.add_color_stop_rgba(0.0, r1, g1, b1, a1);
        gradient.add_color_stop_rgba(1.0, r2, g2, b2, a2);
        context
            .set_source(&gradient)
            .map_err(|error| error.to_string())
    } else {
        let (red, green, blue, alpha) = rgba(state.color, alpha_scale);
        context.set_source_rgba(red, green, blue, alpha);
        Ok(())
    }
}

fn rgba(color: i64, alpha_scale: f64) -> (f64, f64, f64, f64) {
    let bytes = (color as u32).to_be_bytes();
    (
        f64::from(bytes[1]) / 255.0,
        f64::from(bytes[2]) / 255.0,
        f64::from(bytes[3]) / 255.0,
        f64::from(bytes[0]) / 255.0 * alpha_scale,
    )
}

fn select_font(context: &gtk::cairo::Context, state: &GraphicsState) {
    let slant = if state.italic {
        gtk::cairo::FontSlant::Italic
    } else {
        gtk::cairo::FontSlant::Normal
    };
    let weight = if state.bold {
        gtk::cairo::FontWeight::Bold
    } else {
        gtk::cairo::FontWeight::Normal
    };
    context.select_font_face(&state.font, slant, weight);
    context.set_font_size(state.text_size);
}

fn rounded_rectangle(
    context: &gtk::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) {
    let radius = radius.min(width.abs() / 2.0).min(height.abs() / 2.0);
    context.new_sub_path();
    context.arc(
        x + width - radius,
        y + radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    context.arc(
        x + width - radius,
        y + height - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    context.arc(
        x + radius,
        y + height - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    context.arc(
        x + radius,
        y + radius,
        radius,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    context.close_path();
}

fn safe_project_file(root: Option<&Path>, relative: &str) -> Result<PathBuf, String> {
    let root = root.ok_or_else(|| "abra um projeto para carregar recursos gráficos".to_owned())?;
    let root = std::fs::canonicalize(root)
        .map_err(|error| format!("pasta do projeto indisponível: {error}"))?;
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("o caminho deve permanecer dentro do projeto".to_owned());
    }
    let path = std::fs::canonicalize(root.join(relative_path))
        .map_err(|error| format!("arquivo do projeto indisponível: {error}"))?;
    if !path.starts_with(root) || !path.is_file() {
        return Err("o caminho deve apontar para um arquivo do projeto".to_owned());
    }
    Ok(path)
}

fn safe_project_output(root: Option<&Path>, relative: &str) -> Result<PathBuf, String> {
    let root = root.ok_or_else(|| "abra um projeto para salvar imagens".to_owned())?;
    let root = std::fs::canonicalize(root)
        .map_err(|error| format!("pasta do projeto indisponível: {error}"))?;
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("o caminho deve permanecer dentro do projeto".to_owned());
    }
    let file_name = relative_path
        .file_name()
        .ok_or_else(|| "o caminho do arquivo é inválido".to_owned())?;
    let relative_parent = relative_path.parent().unwrap_or_else(|| Path::new(""));
    let parent = std::fs::canonicalize(root.join(relative_parent))
        .map_err(|error| format!("pasta de destino indisponível: {error}"))?;
    if !parent.starts_with(&root) {
        return Err("a pasta de destino sai da pasta do projeto".to_owned());
    }
    let target = parent.join(file_name);
    match std::fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("o arquivo de destino não pode ser um link simbólico".to_owned());
        }
        Ok(_) => {
            let canonical = std::fs::canonicalize(&target)
                .map_err(|error| format!("arquivo de destino inválido: {error}"))?;
            if !canonical.starts_with(&root) || !canonical.is_file() {
                return Err("o arquivo de destino sai da pasta do projeto".to_owned());
            }
            Ok(canonical)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(target),
        Err(error) => Err(format!("arquivo de destino inválido: {error}")),
    }
}

fn load_image_asset(path: &Path) -> Result<ImageAsset, String> {
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

fn get_gif(state: &GraphicsState, handle: i64) -> Result<&ImageAsset, String> {
    let image = state
        .images
        .get(&handle)
        .ok_or_else(|| format!("endereço de imagem inválido ({handle})"))?;
    if !image.is_gif {
        return Err(format!("a imagem ({handle}) não é uma animação GIF"));
    }
    Ok(image)
}

fn get_gif_mut(state: &mut GraphicsState, handle: i64) -> Result<&mut ImageAsset, String> {
    let image = state
        .images
        .get_mut(&handle)
        .ok_or_else(|| format!("endereço de imagem inválido ({handle})"))?;
    if !image.is_gif {
        return Err(format!("a imagem ({handle}) não é uma animação GIF"));
    }
    Ok(image)
}

fn validate_crop(
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

fn pixbuf_pixel_color(pixbuf: &gtk::gdk_pixbuf::Pixbuf, x: i32, y: i32) -> Result<i64, String> {
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

fn surface_to_pixbuf(
    surface: &gtk::cairo::ImageSurface,
) -> Result<gtk::gdk_pixbuf::Pixbuf, String> {
    if surface.format() != gtk::cairo::Format::ARgb32 {
        return Err("formato de captura gráfica não suportado".to_owned());
    }
    check_pixel_count(surface.width(), surface.height())?;
    let width = surface.width();
    let height = surface.height();
    let stride = usize::try_from(surface.stride()).map_err(|_| "superfície inválida".to_owned())?;
    let mut rgba = vec![0; width as usize * height as usize * 4];
    surface
        .with_data(|source| {
            for y in 0..height as usize {
                for x in 0..width as usize {
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

fn apply_chroma_key(
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

fn transform_pixbuf(
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

fn gif_frame_delays(bytes: &[u8]) -> Result<Vec<i64>, String> {
    if bytes.len() < 13 || !bytes.starts_with(b"GIF87a") && !bytes.starts_with(b"GIF89a") {
        return Err("arquivo GIF inválido".to_owned());
    }
    let packed = bytes[10];
    let mut offset = 13usize;
    if packed & 0x80 != 0 {
        offset = offset
            .checked_add(3 * (1usize << (usize::from(packed & 0x07) + 1)))
            .ok_or_else(|| "arquivo GIF inválido".to_owned())?;
    }
    let mut next_delay = 100i64;
    let mut delays = Vec::new();
    while offset < bytes.len() {
        match bytes[offset] {
            0x3b => return Ok(delays),
            0x21 => {
                let label = *bytes
                    .get(offset + 1)
                    .ok_or_else(|| "extensão GIF incompleta".to_owned())?;
                offset += 2;
                if label == 0xf9 {
                    let size = *bytes
                        .get(offset)
                        .ok_or_else(|| "controle de quadro GIF incompleto".to_owned())?
                        as usize;
                    if size != 4 || offset + size + 2 > bytes.len() {
                        return Err("controle de quadro GIF inválido".to_owned());
                    }
                    let centiseconds = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                    next_delay = i64::from(centiseconds.max(1)) * 10;
                    offset += size + 2;
                } else {
                    skip_gif_sub_blocks(bytes, &mut offset)?;
                }
            }
            0x2c => {
                if offset + 10 > bytes.len() {
                    return Err("descritor de quadro GIF incompleto".to_owned());
                }
                let image_packed = bytes[offset + 9];
                offset += 10;
                if image_packed & 0x80 != 0 {
                    offset = offset
                        .checked_add(3 * (1usize << (usize::from(image_packed & 0x07) + 1)))
                        .ok_or_else(|| "arquivo GIF inválido".to_owned())?;
                }
                if offset >= bytes.len() {
                    return Err("dados de quadro GIF incompletos".to_owned());
                }
                offset += 1;
                skip_gif_sub_blocks(bytes, &mut offset)?;
                delays.push(next_delay);
                next_delay = 100;
            }
            _ => return Err("bloco GIF inválido".to_owned()),
        }
    }
    Err("arquivo GIF sem terminador".to_owned())
}

fn skip_gif_sub_blocks(bytes: &[u8], offset: &mut usize) -> Result<(), String> {
    loop {
        let size = *bytes
            .get(*offset)
            .ok_or_else(|| "bloco GIF incompleto".to_owned())? as usize;
        *offset += 1;
        if size == 0 {
            return Ok(());
        }
        *offset = offset
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| "bloco GIF incompleto".to_owned())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixbuf_2_by_1() -> gtk::gdk_pixbuf::Pixbuf {
        let pixbuf =
            gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 2, 1).unwrap();
        pixbuf.put_pixel(0, 0, 255, 0, 0, 255);
        pixbuf.put_pixel(1, 0, 0, 255, 0, 255);
        pixbuf
    }

    fn one_frame_gif() -> Vec<u8> {
        vec![
            b'G', b'I', b'F', b'8', b'9', b'a', 1, 0, 1, 0, 0x80, 0, 0, 0, 0, 0, 255, 255, 255,
            0x21, 0xf9, 4, 1, 10, 0, 0, 0, 0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 1, 0, 0x3b,
        ]
    }

    #[test]
    fn graphics_rotation_expands_bounds_and_chroma_key_makes_color_transparent() {
        let pixbuf = pixbuf_2_by_1();
        let rotated = transform_pixbuf(&pixbuf, false, false, 90, 0).unwrap();
        assert_eq!((rotated.width(), rotated.height()), (1, 2));

        let transparent = transform_pixbuf(&pixbuf, false, false, 0, -65_536).unwrap();
        assert_eq!(
            (pixbuf_pixel_color(&transparent, 0, 0).unwrap() as u32) >> 24,
            0
        );
        assert_eq!(
            (pixbuf_pixel_color(&transparent, 1, 0).unwrap() as u32) >> 24,
            255
        );
    }

    #[test]
    fn graphics_pixel_and_crop_operations_reject_out_of_bounds_access() {
        let pixbuf = pixbuf_2_by_1();
        assert!(pixbuf_pixel_color(&pixbuf, 2, 0).is_err());
        assert!(validate_crop(&pixbuf, 1, 0, 2, 1).is_err());
        assert!(validate_crop(&pixbuf, 1, 0, 1, 1).is_ok());
    }

    #[test]
    fn cairo_capture_restores_straight_alpha_pixels() {
        let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, 1, 1).unwrap();
        let context = gtk::cairo::Context::new(&surface).unwrap();
        context.set_operator(gtk::cairo::Operator::Source);
        context.set_source_rgba(1.0, 0.0, 0.0, 0.5);
        context.paint().unwrap();

        let pixbuf = surface_to_pixbuf(&surface).unwrap();
        assert_eq!(
            pixbuf_pixel_color(&pixbuf, 0, 0).unwrap() as u32,
            0x80ff0000
        );
    }

    #[test]
    fn gif_frame_parser_reads_delays_and_rejects_truncation() {
        let gif = one_frame_gif();
        assert_eq!(gif_frame_delays(&gif).unwrap(), [100]);
        assert!(gif_frame_delays(&gif[..gif.len() - 1]).is_err());
    }

    #[test]
    fn gif_loading_uses_content_even_when_the_filename_has_another_extension() {
        let root = std::env::temp_dir().join(format!(
            "potoo-gif-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("animation.data");
        let gif = one_frame_gif();
        std::fs::write(&path, gif).unwrap();

        let image = load_image_asset(&path).unwrap();
        assert_eq!(image.frame_delays, [100]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_graphics_paths_reject_traversal_and_output_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "potoo-graphics-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        assert!(safe_project_file(Some(&root), "../outside.png").is_err());
        assert!(safe_project_output(Some(&root), "../outside.png").is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let outside = root.with_extension("outside");
            std::fs::write(&outside, b"protected").unwrap();
            symlink(&outside, root.join("escape.png")).unwrap();
            assert!(safe_project_output(Some(&root), "escape.png").is_err());
            std::fs::remove_file(outside).unwrap();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
