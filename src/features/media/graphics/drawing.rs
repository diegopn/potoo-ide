use super::*;

pub(super) fn color_channel(value: i64) -> Result<u8, String> {
    u8::try_from(value).map_err(|_| "cada canal de cor deve estar entre 0 e 255".to_owned())
}

pub(super) fn bounds_from_points(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in points {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

pub(super) fn rotate_about(
    context: &gtk::cairo::Context,
    degrees: i64,
    center_x: f64,
    center_y: f64,
) -> Result<(), String> {
    context.save().map_err(|error| error.to_string())?;
    if degrees.rem_euclid(360) != 0 {
        context.translate(center_x, center_y);
        context.rotate((degrees as f64).to_radians());
        context.translate(-center_x, -center_y);
    }
    Ok(())
}

pub(super) fn set_graphics_source(
    context: &gtk::cairo::Context,
    state: &GraphicsState,
    (x, y, shape_width, shape_height): (f64, f64, f64, f64),
) -> Result<(), String> {
    let alpha_scale = state.opacity as f64 / 255.0;
    if let Some((kind, first, second)) = state.gradient {
        let left = x.min(x + shape_width);
        let top = y.min(y + shape_height);
        let width = shape_width.abs().max(1.0);
        let height = shape_height.abs().max(1.0);
        let (x1, y1, x2, y2) = gradient_endpoints(kind, (left, top, width, height))?;
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

pub(super) fn gradient_endpoints(
    kind: i64,
    (x, y, width, height): (f64, f64, f64, f64),
) -> Result<(f64, f64, f64, f64), String> {
    match kind {
        0 => Ok((x, y, x + width, y)),
        1 => Ok((x + width, y, x, y)),
        2 => Ok((x, y + height, x, y)),
        3 => Ok((x, y, x, y + height)),
        4 => Ok((x, y, x + width, y + height)),
        5 => Ok((x + width, y, x, y + height)),
        6 => Ok((x, y + height, x + width, y)),
        7 => Ok((x + width, y + height, x, y)),
        _ => Err("tipo de gradiente inválido".to_owned()),
    }
}

pub(super) fn draw_ellipse(
    context: &gtk::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    filled: bool,
) -> Result<(), String> {
    context.save().map_err(|error| error.to_string())?;
    context.translate(x + width / 2.0, y + height / 2.0);
    context.scale(width / 2.0, height / 2.0);
    context.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
    if filled {
        let result = context.fill().map_err(|error| error.to_string());
        context.restore().map_err(|error| error.to_string())?;
        result
    } else {
        context.restore().map_err(|error| error.to_string())?;
        context.set_line_width(1.0);
        context.stroke().map_err(|error| error.to_string())
    }
}

pub(super) fn draw_polygon(
    context: &gtk::cairo::Context,
    points: &[(f64, f64)],
    filled: bool,
) -> Result<(), String> {
    context.move_to(points[0].0, points[0].1);
    for (x, y) in points.iter().skip(1) {
        context.line_to(*x, *y);
    }
    context.close_path();
    if filled {
        context.fill().map_err(|error| error.to_string())
    } else {
        context.stroke().map_err(|error| error.to_string())
    }
}

pub(super) fn rgba(color: i64, alpha_scale: f64) -> (f64, f64, f64, f64) {
    let bytes = (color as u32).to_be_bytes();
    (
        f64::from(bytes[1]) / 255.0,
        f64::from(bytes[2]) / 255.0,
        f64::from(bytes[3]) / 255.0,
        alpha_scale,
    )
}

pub(super) fn select_font(context: &gtk::cairo::Context, state: &GraphicsState) {
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

pub(super) fn text_advance(context: &gtk::cairo::Context, text: &str) -> Result<f64, String> {
    context
        .text_extents(text)
        .map(|extents| extents.x_advance())
        .map_err(|error| error.to_string())
}

pub(super) fn text_height(context: &gtk::cairo::Context) -> Result<f64, String> {
    context
        .font_extents()
        .map(|extents| extents.height() - extents.descent())
        .map_err(|error| error.to_string())
}

pub(super) fn rounded_rectangle(
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
