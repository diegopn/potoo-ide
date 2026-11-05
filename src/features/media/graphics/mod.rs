mod arguments;
mod drawing;
mod gif;
mod host;
mod images;
mod input;
mod paths;
mod render;

use self::arguments::*;
use self::drawing::*;
use self::gif::*;
pub(crate) use self::host::GraphicsHost;
use self::images::*;
use self::input::*;
use self::paths::*;
use self::render::*;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime};

use gtk::gdk::prelude::*;
use gtk::prelude::*;
use potoo_portugol_interpreter::HostValue;

use crate::features::execution::HostRequest;

type HostReply = Sender<Result<HostValue, String>>;
const MAX_IMAGE_MEMORY_BYTES: usize = 128 * 1024 * 1024;

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

fn empty_image_asset() -> Result<ImageAsset, String> {
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 200, 200)
        .ok_or_else(|| "não foi possível criar a imagem vazia".to_owned())?;
    pixbuf.fill(0);
    Ok(ImageAsset::still(pixbuf))
}

struct GraphicsState {
    window: Option<gtk::Window>,
    area: Option<gtk::DrawingArea>,
    surface: Option<gtk::cairo::ImageSurface>,
    presented_surface: Option<gtk::cairo::ImageSurface>,
    fullscreen_restore: Option<(i32, i32)>,
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
    image_bytes: usize,
    next_image_handle: i64,
    pressed_keys: HashMap<u32, i64>,
    pressed_buttons: HashSet<i64>,
    mouse_x: i64,
    mouse_y: i64,
    cursor_visible: bool,
    pending_key_reads: Vec<HostReply>,
    pending_button_reads: Vec<HostReply>,
    preview_active: bool,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            window: None,
            area: None,
            surface: None,
            presented_surface: None,
            fullscreen_restore: None,
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
            image_bytes: 0,
            next_image_handle: 1,
            pressed_keys: HashMap::new(),
            pressed_buttons: HashSet::new(),
            mouse_x: 0,
            mouse_y: 0,
            cursor_visible: true,
            pending_key_reads: Vec::new(),
            pending_button_reads: Vec::new(),
            preview_active: false,
        }
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

    fn pixel(surface: &gtk::cairo::ImageSurface, x: i32, y: i32) -> i64 {
        pixbuf_pixel_color(&surface_to_pixbuf(surface).unwrap(), x, y).unwrap()
    }

    fn paint_pixel(surface: &gtk::cairo::ImageSurface, x: f64, y: f64, rgba: (f64, f64, f64, f64)) {
        let context = gtk::cairo::Context::new(surface).unwrap();
        context.set_source_rgba(rgba.0, rgba.1, rgba.2, rgba.3);
        context.rectangle(x, y, 1.0, 1.0);
        context.fill().unwrap();
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
    fn graphics_compat_initialization_guards_leave_image_only_calls_available() {
        assert!(requires_initialized_graphics("largura_janela"));
        assert!(requires_initialized_graphics("desenhar_retangulo"));
        assert!(requires_initialized_graphics("desenhar_imagem"));
        assert!(requires_initialized_graphics("jerimum_desenhar_texto"));
        assert!(requires_initialized_graphics("jerimum_desenhar_imagem"));
        assert!(requires_initialized_graphics("renderizar"));
        assert!(!requires_initialized_graphics("renderizar_imagem"));
        assert!(!requires_initialized_graphics("carregar_imagem"));
        assert!(!requires_initialized_graphics("obter_numero_quadros_gif"));
    }

    #[test]
    fn graphics_compat_rendering_commits_one_frame_and_starts_the_next_empty() {
        let mut state = GraphicsState::default();
        state.width = 2;
        state.height = 1;
        state.surface = Some(transparent_surface(2, 1).unwrap());
        paint_pixel(
            state.surface.as_ref().unwrap(),
            0.0,
            0.0,
            (1.0, 0.0, 0.0, 1.0),
        );

        present_frame(&mut state).unwrap();
        assert_eq!(
            pixel(state.presented_surface.as_ref().unwrap(), 0, 0) as u32,
            0xffff0000
        );
        assert_eq!(pixel(state.surface.as_ref().unwrap(), 0, 0) as u32 >> 24, 0);

        paint_pixel(
            state.surface.as_ref().unwrap(),
            1.0,
            0.0,
            (0.0, 0.0, 1.0, 1.0),
        );
        assert_eq!(
            pixel(state.presented_surface.as_ref().unwrap(), 1, 0) as u32 >> 24,
            0
        );
        present_frame(&mut state).unwrap();
        assert_eq!(
            pixel(state.presented_surface.as_ref().unwrap(), 0, 0) as u32 >> 24,
            0
        );
        assert_eq!(
            pixel(state.presented_surface.as_ref().unwrap(), 1, 0) as u32,
            0xff0000ff
        );
    }

    #[test]
    fn graphics_compat_image_capture_keeps_coordinates_and_transparent_background() {
        let pending = transparent_surface(5, 2).unwrap();
        paint_pixel(&pending, 2.0, 1.0, (0.0, 1.0, 0.0, 1.0));
        let capture = capture_pending_pixbuf(&pending, 3, 2).unwrap();
        assert_eq!(
            pixbuf_pixel_color(&capture, 2, 1).unwrap() as u32,
            0xff00ff00
        );
        assert_eq!(pixbuf_pixel_color(&capture, 1, 1).unwrap() as u32 >> 24, 0);
        assert_eq!((capture.width(), capture.height()), (3, 2));
    }

    #[test]
    fn graphics_compat_rotation_uses_the_object_center() {
        let surface = transparent_surface(50, 50).unwrap();
        let context = gtk::cairo::Context::new(&surface).unwrap();
        rotate_about(&context, 90, 20.0, 15.0).unwrap();
        context.set_source_rgba(1.0, 0.0, 0.0, 1.0);
        context.rectangle(10.0, 10.0, 20.0, 10.0);
        context.fill().unwrap();
        context.restore().unwrap();
        assert_eq!(pixel(&surface, 20, 10) as u32, 0xffff0000);
        assert_eq!(pixel(&surface, 27, 15) as u32 >> 24, 0);
    }

    #[test]
    fn graphics_compat_ellipse_outline_uses_a_one_pixel_stroke() {
        let surface = transparent_surface(130, 130).unwrap();
        let context = gtk::cairo::Context::new(&surface).unwrap();
        context.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        draw_ellipse(&context, 10.0, 10.0, 100.0, 100.0, false).unwrap();
        assert_eq!(pixel(&surface, 60, 60) as u32 >> 24, 0);
        assert_ne!(pixel(&surface, 60, 10) as u32 >> 24, 0);
        assert_eq!(pixel(&surface, 60, 8) as u32 >> 24, 0);
    }

    #[test]
    fn graphics_compat_gradient_endpoints_are_relative_to_the_shape() {
        assert_eq!(
            gradient_endpoints(0, (100.0, 100.0, 20.0, 10.0)).unwrap(),
            (100.0, 100.0, 120.0, 100.0)
        );
        assert_eq!(
            gradient_endpoints(7, (100.0, 100.0, 20.0, 10.0)).unwrap(),
            (120.0, 110.0, 100.0, 100.0)
        );
    }

    #[test]
    fn graphics_compat_gradient_fills_the_shape_bounds() {
        let surface = transparent_surface(140, 120).unwrap();
        let context = gtk::cairo::Context::new(&surface).unwrap();
        let mut state = GraphicsState::default();
        state.gradient = Some((0, 0x00ff0000, 0x000000ff));
        set_graphics_source(&context, &state, (100.0, 100.0, 20.0, 10.0)).unwrap();
        context.rectangle(100.0, 100.0, 20.0, 10.0);
        context.fill().unwrap();
        let left = pixel(&surface, 100, 104) as u32;
        let right = pixel(&surface, 119, 104) as u32;
        assert!(((left >> 16) & 0xff) > (left & 0xff));
        assert!((right & 0xff) > ((right >> 16) & 0xff));
    }

    #[test]
    fn graphics_compat_raw_rgb_colors_are_opaque_and_opacity_is_separate() {
        assert_eq!(rgba(255, 1.0), (0.0, 0.0, 1.0, 1.0));
        assert_eq!(rgba(255, 0.5), (0.0, 0.0, 1.0, 0.5));
    }

    #[test]
    fn graphics_compat_unfilled_polygons_draw_the_closing_edge() {
        let surface = transparent_surface(24, 24).unwrap();
        let context = gtk::cairo::Context::new(&surface).unwrap();
        context.set_source_rgba(1.0, 0.0, 0.0, 1.0);
        context.set_line_width(1.0);
        draw_polygon(&context, &[(3.0, 3.0), (20.0, 3.0), (20.0, 20.0)], false).unwrap();
        assert_ne!(pixel(&surface, 10, 10) as u32 >> 24, 0);
    }

    #[test]
    fn graphics_compat_gif_resizing_preserves_all_frames_and_animation_state() {
        let first = pixbuf_2_by_1();
        let second =
            gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 2, 1).unwrap();
        second.put_pixel(0, 0, 0, 0, 255, 255);
        second.put_pixel(1, 0, 255, 255, 0, 255);
        let source = ImageAsset {
            frames: vec![first, second],
            frame_delays: vec![40, 80],
            current_frame: 1,
            is_gif: true,
        };
        let resized =
            resize_image_asset(&source, 4, 2, gtk::gdk_pixbuf::InterpType::Nearest).unwrap();
        assert_eq!(resized.frames.len(), 2);
        assert_eq!(resized.frame_delays, [40, 80]);
        assert_eq!(resized.current_frame, 1);
        assert!(resized.is_gif);
        assert_eq!(
            (resized.frames[1].width(), resized.frames[1].height()),
            (4, 2)
        );
    }

    #[test]
    fn graphics_compat_resizing_updates_reported_dimensions_and_surfaces() {
        let mut state = GraphicsState::default();
        state.surface = Some(transparent_surface(2, 1).unwrap());
        state.presented_surface = Some(transparent_surface(2, 1).unwrap());
        paint_pixel(
            state.surface.as_ref().unwrap(),
            1.0,
            0.0,
            (1.0, 0.0, 0.0, 1.0),
        );
        resize_graphics_state(&mut state, 4, 3).unwrap();
        assert_eq!((state.width, state.height), (4, 3));
        assert_eq!(
            (
                state.surface.as_ref().unwrap().width(),
                state.surface.as_ref().unwrap().height()
            ),
            (4, 3)
        );
        assert_eq!(
            pixel(state.surface.as_ref().unwrap(), 1, 0) as u32,
            0xffff0000
        );
    }

    #[test]
    fn graphics_compat_text_metrics_include_whitespace_advance_and_match_text_height() {
        let surface = transparent_surface(1, 1).unwrap();
        let context = gtk::cairo::Context::new(&surface).unwrap();
        context.select_font_face(
            "Sans",
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Normal,
        );
        context.set_font_size(14.0);
        let space = context.text_extents(" ").unwrap();
        let trailing_space = context.text_extents("x ").unwrap();
        let font = context.font_extents().unwrap();
        assert!(space.x_advance() > 0.0);
        assert!(trailing_space.x_advance() > trailing_space.width());
        assert_eq!(text_advance(&context, " ").unwrap(), space.x_advance());
        let measured_height = text_height(&context).unwrap();
        assert!(measured_height > 0.0);
        assert!(measured_height < font.height());
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

    #[test]
    fn image_memory_accounting_enforces_an_aggregate_budget() {
        let asset = ImageAsset::still(pixbuf_2_by_1());
        let expected_bytes =
            asset.frames[0].rowstride() as usize * asset.frames[0].height() as usize;
        assert_eq!(image_asset_memory_bytes(&asset).unwrap(), expected_bytes);
        assert_eq!(
            checked_image_memory(0, MAX_IMAGE_MEMORY_BYTES).unwrap(),
            MAX_IMAGE_MEMORY_BYTES
        );
        assert!(checked_image_memory(MAX_IMAGE_MEMORY_BYTES, 1).is_err());
        assert!(checked_image_memory(usize::MAX, 1).is_err());
    }
}
