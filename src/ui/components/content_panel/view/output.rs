use super::*;

pub(crate) fn build_output_panel(
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) -> OutputPanelUi {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("potoo-output-panel");
    panel.set_vexpand(true);

    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    tabs.add_css_class("potoo-output-tabs");
    let output_tab = gtk::Button::with_label(tr(language, "Saída"));
    let problems_tab = gtk::Button::with_label(tr(language, "Problemas"));
    let tab_buttons = [output_tab.clone(), problems_tab.clone()];
    output_tab.add_css_class("potoo-output-tab-selected");
    for button in &tab_buttons {
        button.add_css_class("potoo-output-tab");
        tabs.append(button);
    }

    let stack = gtk::Stack::new();
    stack.add_css_class("potoo-output-stack");
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::None);

    let output = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(14)
        .right_margin(14)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    output.add_css_class("potoo-output-view");
    output.add_css_class("potoo-output-placeholder-text");
    let output_buffer = output.buffer();
    output_buffer.set_text(tr(language, "A saída aparecerá aqui após a execução."));
    let output_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&output)
        .build();
    stack.add_named(&output_scroll, Some("output"));

    let problems = gtk::Box::new(gtk::Orientation::Vertical, 6);
    problems.add_css_class("potoo-problems-list");
    problems.set_margin_start(12);
    problems.set_margin_end(12);
    problems.set_margin_top(10);
    problems.set_margin_bottom(12);
    let empty_problems = gtk::Label::new(Some(tr(language, "Nenhum problema detectado")));
    empty_problems.set_xalign(0.0);
    empty_problems.add_css_class("potoo-problems-empty");
    problems.append(&empty_problems);
    let problems_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&problems)
        .build();
    stack.add_named(&problems_scroll, Some("problems"));

    let debugger = gtk::Box::new(gtk::Orientation::Vertical, 8);
    debugger.add_css_class("potoo-debugger-page");
    debugger.set_size_request(278, -1);
    debugger.set_vexpand(true);
    debugger.set_margin_start(14);
    debugger.set_margin_end(14);
    debugger.set_margin_top(12);
    debugger.set_margin_bottom(12);
    let debugger_controls = gtk::Stack::new();
    debugger_controls.set_transition_type(gtk::StackTransitionType::Crossfade);
    debugger_controls.set_transition_duration(200);
    debugger_controls.set_interpolate_size(true);
    debugger_controls.set_hexpand(true);
    let debugger_start = gtk::Button::new();
    let start_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    start_content.set_halign(gtk::Align::Center);
    start_content.append(&icon("debugger"));
    start_content.append(&gtk::Label::new(Some(tr(language, "Iniciar depuração"))));
    debugger_start.set_child(Some(&start_content));
    debugger_start.set_tooltip_text(Some(tr(language, "Iniciar depuração")));
    debugger_start.add_css_class("potoo-debugger-start");
    debugger_start.set_hexpand(true);

    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    controls.set_halign(gtk::Align::Fill);
    controls.set_hexpand(true);
    controls.set_homogeneous(true);
    let debugger_step = gtk::Button::new();
    debugger_step.set_child(Some(&icon("step")));
    debugger_step.set_tooltip_text(Some(tr(language, "Avançar um passo")));
    debugger_step.update_property(&[gtk::accessible::Property::Label(tr(
        language,
        "Avançar um passo",
    ))]);
    let debugger_stop = gtk::Button::new();
    let stop_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    stop_content.set_halign(gtk::Align::Center);
    stop_content.append(&icon("stop"));
    stop_content.append(&gtk::Label::new(Some(tr(language, "Parar depuração"))));
    debugger_stop.set_child(Some(&stop_content));
    debugger_stop.set_tooltip_text(Some(tr(language, "Parar depuração")));
    debugger_stop.update_property(&[gtk::accessible::Property::Label(tr(
        language,
        "Parar depuração",
    ))]);
    debugger_stop.add_css_class("potoo-debugger-stop");
    debugger_stop.set_hexpand(true);
    debugger_stop.set_visible(false);
    debugger_step.add_css_class("potoo-debugger-action");
    debugger_step.add_css_class("potoo-debugger-step");
    debugger_step.set_hexpand(true);
    controls.append(&debugger_step);
    let sender_for_start = sender.clone();
    connect_debugger_start(&debugger_start, &debugger_controls, move || {
        sender_for_start.input(MainWindowMsg::StartDebug);
    });
    let sender_for_step = sender.clone();
    debugger_step.connect_clicked(move |_| sender_for_step.input(MainWindowMsg::StepDebug));
    debugger_stop.connect_clicked(move |_| sender.input(MainWindowMsg::StopDebug));
    debugger_controls.add_named(&debugger_start, Some("start"));
    debugger_controls.add_named(&controls, Some("controls"));
    debugger_controls.set_visible_child_name("start");
    debugger.append(&debugger_controls);
    let debugger_position = gtk::Label::new(Some(tr(language, "Aguardando depuração")));
    debugger_position.set_xalign(0.0);
    debugger_position.set_margin_top(16);
    debugger_position.add_css_class("potoo-debugger-position");
    debugger.append(&debugger_position);
    let debugger_variables = gtk::TextBuffer::new(None);
    debugger_variables.set_text(tr(
        language,
        "Os valores das variáveis aparecerão durante a execução passo a passo.",
    ));
    let variables_view = gtk::TextView::builder()
        .buffer(&debugger_variables)
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(8)
        .right_margin(8)
        .top_margin(6)
        .bottom_margin(6)
        .build();
    variables_view.add_css_class("potoo-debugger-variables");
    let variables_scroll = gtk::ScrolledWindow::builder()
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&variables_view)
        .build();
    variables_scroll.add_css_class("potoo-debugger-output");
    variables_scroll.set_min_content_height(96);
    variables_scroll.set_max_content_height(96);
    variables_scroll.set_propagate_natural_height(false);
    variables_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    variables_scroll.set_vscrollbar_policy(gtk::PolicyType::Automatic);
    variables_scroll.set_margin_top(8);
    variables_scroll.set_vexpand(false);
    debugger.append(&variables_scroll);
    let debugger_learning = build_debugger_learning(language);
    debugger.append(&debugger_learning.analysis_stack);
    debugger.append(&debugger_stop);
    stack.set_visible_child_name("output");

    for (button, child_name) in tab_buttons.iter().zip(["output", "problems"]) {
        let button_for_switch = button.clone();
        let stack_for_switch = stack.clone();
        let buttons_for_selection = tab_buttons.clone();
        button.connect_clicked(move |_| {
            for candidate in &buttons_for_selection {
                candidate.remove_css_class("potoo-output-tab-selected");
            }
            button_for_switch.add_css_class("potoo-output-tab-selected");
            stack_for_switch.set_visible_child_name(child_name);
        });
    }

    panel.append(&tabs);
    panel.append(&stack);
    OutputPanelUi {
        panel,
        debugger_page: debugger,
        output_buffer,
        output_tab,
        problems_tab,
        output_view: output,
        problems_list: problems,
        debugger_controls,
        debugger_start,
        debugger_step,
        debugger_stop,
        debugger_position,
        debugger_variables,
        debugger_learning,
    }
}
