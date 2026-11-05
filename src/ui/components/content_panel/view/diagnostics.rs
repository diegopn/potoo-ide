use super::*;

pub(crate) fn show_diagnostics(
    container: &gtk::Box,
    diagnostics: &[EditorDiagnostic],
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    clear_box(container);
    if diagnostics.is_empty() {
        let empty = gtk::Label::new(Some(tr(language, "Nenhum problema detectado")));
        empty.set_xalign(0.0);
        empty.add_css_class("potoo-problems-empty");
        container.append(&empty);
        return;
    }

    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => ("Erro", "diagnostic-error", "potoo-problem-error"),
            DiagnosticSeverity::Warning => ("Aviso", "diagnostic-warning", "potoo-problem-warning"),
            DiagnosticSeverity::Information => (
                "Informação",
                "diagnostic-information",
                "potoo-problem-information",
            ),
            DiagnosticSeverity::Hint => ("Dica", "diagnostic-hint", "potoo-problem-hint"),
        };
        let category = match diagnostic.category {
            DiagnosticCategory::Lexical => tr(language, "Erro léxico"),
            DiagnosticCategory::Syntax => tr(language, "Erro de sintaxe"),
            DiagnosticCategory::Type => tr(language, "Erro de tipo"),
            DiagnosticCategory::Runtime => tr(language, "Erro de execução"),
            DiagnosticCategory::StaticAnalysis => tr(language, "Análise estática"),
        };
        let file = diagnostic
            .source_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| diagnostic.source_name.clone());

        let button = gtk::Button::new();
        button.set_halign(gtk::Align::Fill);
        button.set_hexpand(true);
        button.add_css_class("potoo-problem-row");
        button.add_css_class(severity.2);
        button.set_tooltip_text(Some(&diagnostic.message));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 5);
        content.set_margin_start(8);
        content.set_margin_end(8);
        content.set_margin_top(7);
        content.set_margin_bottom(7);
        let heading = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        heading.set_hexpand(true);
        let severity_icon = icon(severity.1);
        severity_icon.set_pixel_size(16);
        heading.append(&severity_icon);
        let severity_label = gtk::Label::new(Some(tr(language, severity.0)));
        severity_label.add_css_class("potoo-problem-severity");
        heading.append(&severity_label);
        let location = gtk::Label::new(Some(&format!(
            "{file} · {}:{} · {category} · {}",
            diagnostic.line,
            diagnostic.column,
            diagnostic.language.label()
        )));
        location.set_xalign(0.0);
        location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        location.set_hexpand(true);
        location.add_css_class("potoo-problem-location");
        heading.append(&location);
        content.append(&heading);

        let message = gtk::Label::new(Some(&diagnostic.message));
        message.set_xalign(0.0);
        message.set_wrap(true);
        message.set_selectable(true);
        message.add_css_class("potoo-problem-message");
        content.append(&message);
        button.set_child(Some(&content));

        let sender = sender.clone();
        let diagnostic = diagnostic.clone();
        button.connect_clicked(move |_| {
            sender.input(MainWindowMsg::NavigateToDiagnostic(diagnostic.clone()));
        });
        container.append(&button);
    }
}

pub(crate) fn install_diagnostic_markers(view: &sourceview5::View, dark: bool) {
    for (category, icon_name, priority, background) in [
        (
            "potoo-diagnostic-error",
            "potoo-diagnostic-error-symbolic",
            40,
            gtk::gdk::RGBA::new(
                if dark { 0.62 } else { 0.91 },
                if dark { 0.17 } else { 0.28 },
                if dark { 0.17 } else { 0.25 },
                if dark { 0.22 } else { 0.14 },
            ),
        ),
        (
            "potoo-diagnostic-warning",
            "potoo-diagnostic-warning-symbolic",
            30,
            gtk::gdk::RGBA::new(
                if dark { 0.58 } else { 0.93 },
                if dark { 0.42 } else { 0.68 },
                if dark { 0.13 } else { 0.17 },
                if dark { 0.2 } else { 0.16 },
            ),
        ),
        (
            "potoo-diagnostic-information",
            "potoo-diagnostic-information-symbolic",
            20,
            gtk::gdk::RGBA::new(0.23, 0.48, 0.72, 0.12),
        ),
        (
            "potoo-diagnostic-hint",
            "potoo-diagnostic-hint-symbolic",
            10,
            gtk::gdk::RGBA::new(0.42, 0.48, 0.44, 0.1),
        ),
    ] {
        let attributes = sourceview5::MarkAttributes::new();
        attributes.set_icon_name(icon_name);
        attributes.set_background(&background);
        attributes.connect_query_tooltip_text(|_, mark| {
            mark.name()
                .map_or_else(String::new, |name| name.to_string())
        });
        view.set_mark_attributes(category, &attributes, priority);
    }
}

pub(crate) fn apply_diagnostics(
    _view: &sourceview5::View,
    buffer: &gtk::TextBuffer,
    diagnostics: &[EditorDiagnostic],
    tags: &SyntaxTags,
    _dark: bool,
) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.remove_tag(&tags.diagnostic_error_line, &start, &end);
    buffer.remove_tag(&tags.diagnostic_warning_line, &start, &end);
    buffer.remove_tag(&tags.diagnostic_error_range, &start, &end);
    buffer.remove_tag(&tags.diagnostic_warning_range, &start, &end);

    let Some(source_buffer) = buffer.downcast_ref::<sourceview5::Buffer>() else {
        return;
    };
    for category in [
        "potoo-diagnostic-error",
        "potoo-diagnostic-warning",
        "potoo-diagnostic-information",
        "potoo-diagnostic-hint",
    ] {
        source_buffer.remove_source_marks(&start, &end, Some(category));
    }

    let mut by_line = BTreeMap::<usize, Vec<&EditorDiagnostic>>::new();
    for diagnostic in diagnostics {
        by_line
            .entry(diagnostic.line.max(1))
            .or_default()
            .push(diagnostic);
        let Some(range) = &diagnostic.range else {
            continue;
        };
        let limit = buffer.char_count().max(0) as usize;
        let first = range.start.min(limit);
        let last = range.end.min(limit);
        let tag = match diagnostic.severity {
            DiagnosticSeverity::Error => Some(&tags.diagnostic_error_range),
            DiagnosticSeverity::Warning => Some(&tags.diagnostic_warning_range),
            DiagnosticSeverity::Information | DiagnosticSeverity::Hint => None,
        };
        if let Some(tag) = tag {
            apply_tag_range(buffer, tag, first, last);
        }
    }

    for (line, diagnostics) in by_line {
        let line_index = line
            .saturating_sub(1)
            .min(buffer.line_count().saturating_sub(1) as usize) as i32;
        let Some(line_start) = buffer.iter_at_line(line_index) else {
            continue;
        };
        let mut line_end = line_start.clone();
        line_end.forward_to_line_end();
        let severity = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.severity)
            .max_by_key(|severity| severity_priority(*severity))
            .unwrap_or(DiagnosticSeverity::Hint);
        let line_tag = match severity {
            DiagnosticSeverity::Error => Some(&tags.diagnostic_error_line),
            DiagnosticSeverity::Warning => Some(&tags.diagnostic_warning_line),
            DiagnosticSeverity::Information | DiagnosticSeverity::Hint => None,
        };
        if let Some(tag) = line_tag {
            buffer.apply_tag(tag, &line_start, &line_end);
        }

        let category = marker_category(severity);
        let tooltip = diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}: {}",
                    severity_label(diagnostic.severity),
                    diagnostic.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        source_buffer.create_source_mark(Some(&tooltip), category, &line_start);
    }
}

fn severity_priority(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 4,
        DiagnosticSeverity::Warning => 3,
        DiagnosticSeverity::Information => 2,
        DiagnosticSeverity::Hint => 1,
    }
}

fn marker_category(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "potoo-diagnostic-error",
        DiagnosticSeverity::Warning => "potoo-diagnostic-warning",
        DiagnosticSeverity::Information => "potoo-diagnostic-information",
        DiagnosticSeverity::Hint => "potoo-diagnostic-hint",
    }
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "Erro",
        DiagnosticSeverity::Warning => "Aviso",
        DiagnosticSeverity::Information => "Informação",
        DiagnosticSeverity::Hint => "Dica",
    }
}
