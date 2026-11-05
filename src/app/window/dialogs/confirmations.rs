use super::*;

pub(in crate::app::window) fn show_project_switch_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "Alterações não salvas",
        "Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar.",
        vec!["Cancelar", "Descartar e abrir"],
        Some(1),
        None,
        move |response| sender.input(MainWindowMsg::ProjectSwitchDecision(response == 1)),
    );
}

pub(in crate::app::window) fn show_close_document_confirmation(
    parent: &gtk::ApplicationWindow,
    id: u64,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "O arquivo aberto tem alterações que ainda não foram salvas. O que você gostaria de fazer?",
        "Salve o arquivo antes de fechar ou descarte as alterações para continuar.",
        vec![
            "Continuar editando",
            "Descartar alterações",
            "Salvar e fechar",
        ],
        Some(2),
        None,
        move |response| match response {
            1 => sender.input(MainWindowMsg::CloseDocumentDecision(id, true)),
            2 => sender.input(MainWindowMsg::SaveAndCloseDocument(id)),
            _ => sender.input(MainWindowMsg::CloseDocumentDecision(id, false)),
        },
    );
}

pub(in crate::app::window) fn show_close_app_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "Alterações não salvas",
        "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE.",
        vec!["Continuar editando", "Descartar e sair"],
        Some(1),
        None,
        move |response| sender.input(MainWindowMsg::CloseAppDecision(response == 1)),
    );
}

pub(in crate::app::window) fn show_potoo_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    title: &'static str,
    message: &'static str,
    detail: &'static str,
    actions: Vec<&'static str>,
    primary_action: Option<usize>,
    destructive_action: Option<usize>,
    on_response: impl Fn(usize) + 'static,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(language, title)));
    dialog.set_default_size(520, -1);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, language, title);

    let surface = gtk::Box::new(gtk::Orientation::Vertical, 0);
    surface.add_css_class("potoo-dialog-surface");

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    content.add_css_class("potoo-confirm-content");
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(22);
    content.set_margin_bottom(22);

    let mark = gtk::Label::new(Some("!"));
    mark.add_css_class("potoo-confirm-mark");
    mark.set_valign(gtk::Align::Start);
    content.append(&mark);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 6);
    copy.set_valign(gtk::Align::Center);
    let message_label = gtk::Label::new(Some(tr(language, message)));
    message_label.set_xalign(0.0);
    message_label.set_wrap(true);
    message_label.add_css_class("potoo-confirm-message");
    let detail_label = gtk::Label::new(Some(tr(language, detail)));
    detail_label.set_xalign(0.0);
    detail_label.set_wrap(true);
    detail_label.add_css_class("potoo-confirm-detail");
    copy.append(&message_label);
    copy.append(&detail_label);
    content.append(&copy);
    surface.append(&content);

    let actions_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions_row.add_css_class("potoo-dialog-actions");
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions_row.append(&spacer);

    let response_sent = Rc::new(Cell::new(false));
    let on_response: Rc<dyn Fn(usize)> = Rc::new(on_response);
    for (index, label) in actions.into_iter().enumerate() {
        let button = gtk::Button::with_label(tr(language, label));
        button.add_css_class(if Some(index) == destructive_action {
            "potoo-destructive-button"
        } else if Some(index) == primary_action {
            "potoo-run-button"
        } else {
            "potoo-project-cancel-button"
        });
        let dialog_for_button = dialog.clone();
        let response_sent_for_button = response_sent.clone();
        let on_response_for_button = on_response.clone();
        button.connect_clicked(move |_| {
            if !response_sent_for_button.replace(true) {
                on_response_for_button(index);
            }
            dialog_for_button.close();
        });
        actions_row.append(&button);
    }
    surface.append(&actions_row);
    dialog.set_child(Some(&surface));

    let response_sent_for_close = response_sent;
    dialog.connect_close_request(move |_| {
        if !response_sent_for_close.replace(true) {
            on_response(0);
        }
        gtk::glib::Propagation::Proceed
    });
    dialog.present();
}
