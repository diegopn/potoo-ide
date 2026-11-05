use super::*;
use crate::app::{UiLanguage, tr};
use crate::languages::model::Language;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{gio, glib, prelude::*};
use sourceview5::{CompletionActivation, CompletionCell, CompletionColumn, CompletionContext};
use sourceview5::{subclass::prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
};

#[derive(Clone)]
struct ProposalData {
    item: CompletionItem,
    replace_start: usize,
    append_closing_quote: bool,
}

mod proposal_imp {
    use super::*;
    use gtk::subclass::prelude::*;

    #[derive(Default)]
    pub(super) struct Proposal {
        pub(super) data: RefCell<Option<ProposalData>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Proposal {
        const NAME: &'static str = "PotooCompletionProposal";
        type Type = super::Proposal;
        type Interfaces = (CompletionProposal,);
    }

    impl ObjectImpl for Proposal {}
    impl CompletionProposalImpl for Proposal {}
}

glib::wrapper! {
    struct Proposal(ObjectSubclass<proposal_imp::Proposal>) @implements CompletionProposal;
}

impl Proposal {
    fn new(data: ProposalData) -> Self {
        let proposal: Self = glib::Object::new();
        proposal.imp().data.replace(Some(data));
        proposal
    }

    fn data(&self) -> Option<ProposalData> {
        self.imp().data.borrow().clone()
    }
}

pub(crate) mod provider_imp {
    use super::*;
    use gtk::subclass::prelude::*;

    #[derive(Default)]
    pub(crate) struct Provider {
        pub(super) language: Cell<Option<Language>>,
        pub(super) ui_language: Cell<Option<UiLanguage>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Provider {
        const NAME: &'static str = "PotooCompletionProvider";
        type Type = super::Provider;
        type Interfaces = (CompletionProvider,);
    }

    impl ObjectImpl for Provider {}

    impl CompletionProviderImpl for Provider {
        fn title(&self) -> Option<glib::GString> {
            Some(
                tr(
                    self.ui_language.get().unwrap_or(UiLanguage::Portuguese),
                    "Sugestões",
                )
                .into(),
            )
        }

        fn priority(&self, _context: &CompletionContext) -> i32 {
            100
        }

        fn is_trigger(&self, _iter: &gtk::TextIter, character: char) -> bool {
            character == '_' || character.is_alphanumeric() || matches!(character, '"' | '.' | ' ')
        }

        fn populate_future(
            &self,
            context: &CompletionContext,
        ) -> Pin<Box<dyn Future<Output = Result<gio::ListModel, glib::Error>> + 'static>> {
            let language = self.language.get();
            let Some(language) = language else {
                return Box::pin(async {
                    Ok(gio::ListStore::new::<Proposal>().upcast::<gio::ListModel>())
                });
            };
            let Some(buffer) = context.buffer() else {
                return Box::pin(async {
                    Ok(gio::ListStore::new::<Proposal>().upcast::<gio::ListModel>())
                });
            };
            if buffer.selection_bounds().is_some() {
                return Box::pin(async {
                    Ok(gio::ListStore::new::<Proposal>().upcast::<gio::ListModel>())
                });
            }
            let source = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), true)
                .to_string();
            let cursor = buffer.cursor_position().max(0) as usize;
            let manual = context.activation() == CompletionActivation::UserRequested;

            Box::pin(async move {
                let result =
                    gio::spawn_blocking(move || suggestions(language, &source, cursor, manual))
                        .await
                        .unwrap_or(None);
                let proposals = gio::ListStore::new::<Proposal>();
                if let Some(result) = result {
                    for item in result.items {
                        proposals.append(&Proposal::new(ProposalData {
                            item,
                            replace_start: result.replace_range.start,
                            append_closing_quote: result.append_closing_quote,
                        }));
                    }
                }
                Ok(proposals.upcast::<gio::ListModel>())
            })
        }

        fn refilter(&self, context: &CompletionContext, model: &gio::ListModel) {
            let Some(proposals) = model.downcast_ref::<gio::ListStore>() else {
                return;
            };
            let prefix = context.word().to_lowercase();
            for index in (0..proposals.n_items()).rev() {
                let keep = proposals
                    .item(index)
                    .and_then(|item| item.downcast::<Proposal>().ok())
                    .and_then(|proposal| proposal.data())
                    .is_some_and(|data| data.item.label.to_lowercase().starts_with(&prefix));
                if !keep {
                    proposals.remove(index);
                }
            }
        }

        fn display(
            &self,
            _context: &CompletionContext,
            proposal: &CompletionProposal,
            cell: &CompletionCell,
        ) {
            let Some(data) = proposal.downcast_ref::<Proposal>().and_then(Proposal::data) else {
                return;
            };
            match cell.column() {
                CompletionColumn::TypedText => {
                    let label = gtk::Label::new(Some(&data.item.label));
                    label.set_xalign(0.0);
                    label.add_css_class("potoo-completion-name");
                    cell.set_widget(&label);
                }
                CompletionColumn::Comment => {
                    let language = self.ui_language.get().unwrap_or(UiLanguage::Portuguese);
                    let label =
                        gtk::Label::new(Some(completion_kind_label(data.item.kind, language)));
                    label.add_css_class("potoo-completion-detail");
                    cell.set_widget(&label);
                }
                CompletionColumn::Details => cell.set_text(Some(&data.item.detail)),
                _ => {}
            }
        }

        fn activate(&self, context: &CompletionContext, proposal: &CompletionProposal) {
            let Some(data) = proposal.downcast_ref::<Proposal>().and_then(Proposal::data) else {
                return;
            };
            let Some(buffer) = context.buffer() else {
                return;
            };
            let end_offset = buffer.cursor_position().max(0) as usize;
            if data.replace_start > end_offset {
                return;
            }

            let mut insert_text = data.item.insert_text.clone();
            if data.append_closing_quote && buffer_text_at_offset(&buffer, end_offset) != Some('"')
            {
                insert_text.push('"');
            }
            let mut start = buffer.iter_at_offset(data.replace_start as i32);
            let mut end = buffer.iter_at_offset(end_offset as i32);
            buffer.begin_user_action();
            buffer.delete(&mut start, &mut end);
            let mut insertion = start;
            buffer.insert(&mut insertion, &insert_text);
            let cursor_offset = data
                .item
                .cursor_offset
                .unwrap_or_else(|| data.item.insert_text.chars().count());
            let cursor = buffer.iter_at_offset((data.replace_start + cursor_offset) as i32);
            buffer.place_cursor(&cursor);
            buffer.end_user_action();
        }
    }
}

glib::wrapper! {
    pub(crate) struct Provider(ObjectSubclass<provider_imp::Provider>) @implements CompletionProvider;
}

impl Provider {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    pub(crate) fn set_language(&self, language: Option<Language>) {
        self.imp().language.set(language);
    }

    pub(crate) fn set_ui_language(&self, language: UiLanguage) {
        self.imp().ui_language.set(Some(language));
    }
}

fn completion_kind_label(kind: CompletionKind, language: UiLanguage) -> &'static str {
    match kind {
        CompletionKind::Keyword => tr(language, "palavra-chave"),
        CompletionKind::Function => tr(language, "função"),
        CompletionKind::Library => tr(language, "biblioteca"),
        CompletionKind::Member => tr(language, "membro"),
        CompletionKind::Snippet => tr(language, "snippet"),
        CompletionKind::Identifier => tr(language, "símbolo"),
    }
}

fn buffer_text_at_offset(buffer: &sourceview5::Buffer, offset: usize) -> Option<char> {
    let start = buffer.iter_at_offset(offset as i32);
    let mut end = start;
    end.forward_char()
        .then(|| buffer.text(&start, &end, true).chars().next())
        .flatten()
}
