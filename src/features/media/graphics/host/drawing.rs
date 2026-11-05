use super::*;

impl GraphicsHost {
    pub(super) fn clear_surface(&self) -> Result<(), String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let surface = state.surface.as_ref().unwrap();
        let context = gtk::cairo::Context::new(surface).map_err(|error| error.to_string())?;
        context.set_operator(gtk::cairo::Operator::Source);
        let (red, green, blue, alpha) = rgba(state.color, state.opacity as f64 / 255.0);
        context.set_source_rgba(red, green, blue, alpha);
        context.paint().map_err(|error| error.to_string())?;
        surface.flush();
        Ok(())
    }

    pub(super) fn queue_draw(&self) {
        if let Some(area) = &self.state.borrow().area {
            area.queue_draw();
        }
        if let Some(area) = &self.preview_window_area {
            area.queue_draw();
        }
    }

    pub(super) fn with_draw_context(
        &self,
        draw: impl FnOnce(&gtk::cairo::Context, &GraphicsState) -> Result<(), String>,
    ) -> Result<(), String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let surface = state.surface.as_ref().unwrap();
        let context = gtk::cairo::Context::new(surface).map_err(|error| error.to_string())?;
        context.set_line_width(1.0);
        draw(&context, &state)?;
        surface.flush();
        Ok(())
    }

    pub(super) fn draw_text(&self, x: f64, y: f64, text: &str) -> Result<(), String> {
        self.with_draw_context(|context, state| {
            select_font(context, state);
            let extents = context
                .text_extents(text)
                .map_err(|error| error.to_string())?;
            let font_extents = context.font_extents().map_err(|error| error.to_string())?;
            let width = extents.x_advance();
            let height = font_extents.height();
            set_graphics_source(context, state, (x, y, width, height))?;
            rotate_about(context, state.rotation, x + width / 2.0, y + height / 2.0)?;
            context.move_to(x, y + font_extents.ascent());
            let result = context.show_text(text).map_err(|error| error.to_string());
            result?;
            if state.underline {
                let underline_y = y + font_extents.ascent() + 2.0;
                context.move_to(x, underline_y);
                context.line_to(x + width, underline_y);
                context.stroke().map_err(|error| error.to_string())?;
            }
            context.restore().map_err(|error| error.to_string())
        })
    }

    pub(super) fn measure_context(&self) -> Result<gtk::cairo::Context, String> {
        self.ensure_surface()?;
        let state = self.state.borrow();
        let context = gtk::cairo::Context::new(state.surface.as_ref().unwrap())
            .map_err(|error| error.to_string())?;
        select_font(&context, &state);
        Ok(context)
    }
}
