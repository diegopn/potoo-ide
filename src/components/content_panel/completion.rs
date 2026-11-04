#[cfg(test)]
mod tests {
    use super::{CompletionKind, suggestions};
    use crate::components::content_panel::Language;

    #[test]
    fn suggests_jerimum_in_potigol_import_and_uses_character_offsets() {
        let source = "escreva \"Olá\"\nuse \"jer";
        let cursor = source.chars().count();
        let result = suggestions(Language::Potigol, source, cursor, false)
            .expect("an open Potigol import should show library suggestions");

        assert!(result.items.iter().any(|item| item.label == "jerimum"));
        assert_eq!(result.replace_range, cursor - 3..cursor);
    }

    #[test]
    fn suggests_portugol_libraries_after_the_include_keyword() {
        let source = "inclua biblioteca Mat";
        let result = suggestions(Language::Portugol, source, source.chars().count(), false)
            .expect("the Portugol library context should show suggestions");

        assert!(result.items.iter().any(|item| item.label == "Matematica"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Library)
        );
    }

    #[test]
    fn suggests_library_members_after_a_declared_alias() {
        let source = "inclua biblioteca Matematica --> m\nm.po";
        let result = suggestions(Language::Portugol, source, source.chars().count(), false)
            .expect("an imported library alias should show its members");

        assert!(result.items.iter().any(|item| item.label == "potencia"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Member)
        );
    }

    #[test]
    fn suggests_runtime_members_only_after_the_potigol_library_import() {
        let source = "use \"jerimum\"\njogo.in";
        let result = suggestions(Language::Potigol, source, source.chars().count(), false)
            .expect("an imported Jerimum module should show its members");

        assert!(result.items.iter().any(|item| item.label == "iniciar"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Member)
        );
    }

    #[test]
    fn suggests_default_functions_without_an_import_directive() {
        let source = "ab";
        let result = suggestions(Language::GPortugol, source, source.chars().count(), false)
            .expect("G-Portugol base.gpt functions are available by default");

        assert!(result.items.iter().any(|item| item.label == "absoluto"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Function)
        );
    }

    #[test]
    fn does_not_suggest_an_unsupported_visualg_library_import() {
        let source = "inclua biblioteca ";
        assert!(suggestions(Language::Visualg, source, source.chars().count(), false).is_none());
    }

    #[test]
    fn does_not_suggest_inside_a_potigol_comment() {
        let source = "# escreva";
        assert!(suggestions(Language::Potigol, source, source.chars().count(), true).is_none());
    }

    #[test]
    fn does_not_suggest_inside_an_ordinary_string() {
        let source = "escreva \"escreva\"";
        let cursor = source.chars().count() - 1;
        assert!(suggestions(Language::Potigol, source, cursor, true).is_none());
    }

    #[test]
    fn offers_a_language_specific_snippet_when_opened_manually() {
        let result = suggestions(Language::Portugol, "", 0, true)
            .expect("manual completion should offer snippets without a prefix");

        assert!(
            result
                .items
                .iter()
                .any(|item| item.label == "programa · estrutura inicial")
        );
    }
}
use std::ops::Range;

use super::model::Language;
use crate::app::{UiLanguage, tr};
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{gio, glib, prelude::*};
use sourceview5::{CompletionActivation, CompletionCell, CompletionColumn, CompletionContext};
use sourceview5::{subclass::prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
};

const SNIPPET_CURSOR: &str = "⟪cursor⟫";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompletionKind {
    Keyword,
    Function,
    Library,
    Member,
    Snippet,
    Identifier,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompletionItem {
    pub(crate) label: String,
    pub(crate) insert_text: String,
    pub(crate) detail: String,
    pub(crate) kind: CompletionKind,
    pub(crate) cursor_offset: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompletionResult {
    pub(crate) items: Vec<CompletionItem>,
    pub(crate) replace_range: Range<usize>,
    pub(crate) append_closing_quote: bool,
}

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

#[derive(Clone, Copy)]
struct Snippet {
    label: &'static str,
    text: &'static str,
}

pub(crate) fn suggestions(
    language: Language,
    source: &str,
    cursor_offset: usize,
    manual: bool,
) -> Option<CompletionResult> {
    let chars: Vec<char> = source.chars().collect();
    let cursor = cursor_offset.min(chars.len());
    if is_in_comment(language, &chars, cursor) {
        return None;
    }

    let (word_start, prefix) = current_word(&chars, cursor);
    let mut replace_range = word_start..cursor;
    let mut candidates = Vec::new();

    if language == Language::Potigol
        && let Some((start, library_prefix, needs_quote)) = potigol_library_context(&chars, cursor)
    {
        replace_range = start..cursor;
        for library in POTIGOL_LIBRARIES {
            push_item(
                &mut candidates,
                library,
                library,
                CompletionKind::Library,
                "Biblioteca Potigol",
                None,
            );
        }
        return filtered_result(candidates, library_prefix, replace_range, needs_quote, true);
    }

    if language == Language::Portugol
        && let Some((start, library_prefix)) = portugol_library_context(&chars, cursor)
    {
        replace_range = start..cursor;
        for library in PORTUGOL_LIBRARIES {
            push_item(
                &mut candidates,
                library,
                library,
                CompletionKind::Library,
                "Biblioteca Portugol 2.0",
                None,
            );
        }
        return filtered_result(candidates, library_prefix, replace_range, false, true);
    }

    if is_in_string(language, &chars, cursor) {
        return None;
    }

    if let Some((owner, member_prefix)) = dot_context(&chars, cursor) {
        let members = match language {
            Language::Portugol => portugol_imports(source, cursor)
                .into_iter()
                .find(|(_, alias)| alias.eq_ignore_ascii_case(&owner))
                .map(|(library, _)| portugol_library_members(&library))
                .unwrap_or(&[]),
            Language::Potigol if potigol_jerimum_imported(source, cursor) => {
                potigol_members(&owner)
            }
            Language::Visualg
            | Language::GPortugol
            | Language::Potigol
            | Language::Calango
            | Language::Mapler => &[],
        };
        if !members.is_empty() {
            for member in members {
                push_item(
                    &mut candidates,
                    member,
                    member,
                    CompletionKind::Member,
                    &format!("Membro de {owner}"),
                    None,
                );
            }
            return filtered_result(candidates, member_prefix, replace_range, false, true);
        }
    }

    if !manual && prefix.chars().count() < 2 {
        return None;
    }

    let (keywords, functions, snippets) = catalog(language);
    for keyword in language.keywords().iter().chain(keywords.iter()) {
        if snippets
            .iter()
            .any(|snippet| snippet.label.eq_ignore_ascii_case(*keyword))
        {
            continue;
        }
        let kind = if functions.contains(keyword) {
            CompletionKind::Function
        } else {
            CompletionKind::Keyword
        };
        push_item(
            &mut candidates,
            keyword,
            keyword,
            kind,
            description_for_kind(kind),
            None,
        );
    }
    for function in functions {
        if snippets
            .iter()
            .any(|snippet| snippet.label.eq_ignore_ascii_case(*function))
        {
            continue;
        }
        push_item(
            &mut candidates,
            function,
            function,
            CompletionKind::Function,
            "Função disponível nesta linguagem",
            None,
        );
    }
    for snippet in snippets {
        let Some(marker) = snippet.text.find(SNIPPET_CURSOR) else {
            continue;
        };
        let insertion = snippet.text.replace(SNIPPET_CURSOR, "");
        push_item(
            &mut candidates,
            snippet.label,
            &insertion,
            CompletionKind::Snippet,
            "Snippet de código",
            Some(snippet.text[..marker].chars().count()),
        );
    }
    for identifier in declared_identifiers(language, source) {
        push_item(
            &mut candidates,
            &identifier,
            &identifier,
            CompletionKind::Identifier,
            "Declarado neste arquivo",
            None,
        );
    }

    filtered_result(candidates, prefix, replace_range, false, manual)
}

fn filtered_result(
    candidates: Vec<CompletionItem>,
    prefix: String,
    replace_range: Range<usize>,
    append_closing_quote: bool,
    contextual: bool,
) -> Option<CompletionResult> {
    let folded_prefix = prefix.to_lowercase();
    let items: Vec<_> = candidates
        .into_iter()
        .filter(|item| item.label.to_lowercase().starts_with(&folded_prefix))
        .collect();
    if items.is_empty() || (!contextual && folded_prefix.chars().count() < 2) {
        return None;
    }
    Some(CompletionResult {
        items,
        replace_range,
        append_closing_quote,
    })
}

fn push_item(
    items: &mut Vec<CompletionItem>,
    label: &str,
    insert_text: &str,
    kind: CompletionKind,
    detail: &str,
    cursor_offset: Option<usize>,
) {
    if items.iter().any(|item| item.label == label) {
        return;
    }
    items.push(CompletionItem {
        label: label.to_owned(),
        insert_text: insert_text.to_owned(),
        detail: detail.to_owned(),
        kind,
        cursor_offset,
    });
}

fn description_for_kind(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Keyword => "Palavra-chave",
        CompletionKind::Function => "Função disponível nesta linguagem",
        CompletionKind::Library => "Biblioteca disponível nesta linguagem",
        CompletionKind::Member => "Membro de biblioteca",
        CompletionKind::Snippet => "Snippet de código",
        CompletionKind::Identifier => "Declarado neste arquivo",
    }
}

fn catalog(
    language: Language,
) -> (
    &'static [&'static str],
    &'static [&'static str],
    &'static [Snippet],
) {
    match language {
        Language::Portugol => (
            &[
                "inclua",
                "biblioteca",
                "cadeia",
                "caracter",
                "logico",
                "real",
                "vazio",
                "const",
                "enquanto",
                "faca",
                "para",
                "escolha",
                "caso",
                "pare",
                "retorne",
            ],
            &["escreva", "leia", "limpa"],
            PORTUGOL_SNIPPETS,
        ),
        Language::Visualg => (
            &[
                "algoritmo",
                "var",
                "inicio",
                "fimalgoritmo",
                "inteiro",
                "real",
                "logico",
                "caractere",
                "literal",
                "entao",
                "fimse",
                "enquanto",
                "fimenquanto",
                "faca",
                "para",
                "fimpara",
                "repita",
                "ate",
                "escolha",
                "caso",
                "outrocaso",
                "fimescolha",
                "fimfuncao",
                "fimprocedimento",
                "procedimento",
                "funcao",
                "arquivo",
                "interrompa",
                "aleatorio",
                "timer",
                "pausa",
                "debug",
                "eco",
                "cronometro",
                "limpatela",
                "retorne",
            ],
            &["escreva", "escreval", "leia"],
            VISUALG_SNIPPETS,
        ),
        Language::Potigol => (
            &[
                "def", "const", "Nada", "Inteiro", "Real", "Texto", "Lógico", "Logico", "Lista",
                "Matriz", "Cubo", "faça", "até", "senãose", "abstrato",
            ],
            &[
                "escreva",
                "imprima",
                "leia_texto",
                "leia_inteiro",
                "leia_real",
                "leia_logico",
                "leia_caracter",
                "abs",
                "sqrt",
                "sen",
                "cos",
                "tan",
                "tg",
                "arcsen",
                "arccos",
                "arctg",
                "raiz",
                "leia_numero",
                "leia_número",
                "leia_inteiros",
                "leia_reais",
                "leia_numeros",
                "leia_números",
                "leia_textos",
                "eof",
                "PI",
                "min",
                "menor",
                "max",
                "maior",
                "log",
                "log10",
            ],
            POTIGOL_SNIPPETS,
        ),
        Language::GPortugol => (
            &[
                "variaveis",
                "variáveis",
                "fim-variaveis",
                "fim-variáveis",
                "início",
                "fim",
                "função",
                "inteiro",
                "reais",
                "caractere",
                "literais",
                "lógico",
                "enquanto",
                "repita",
                "até",
                "para",
                "de",
                "passo",
            ],
            &[
                "imprima",
                "leia",
                "absoluto",
                "arredonda",
                "potencia",
                "raiz_quadrada",
            ],
            GPORTUGOL_SNIPPETS,
        ),
        Language::Calango => (
            &[
                "algoritmo",
                "principal",
                "fimPrincipal",
                "inteiro",
                "real",
                "logico",
                "caracter",
                "texto",
                "funcao",
                "procedimento",
                "retorna",
                "se",
                "entao",
                "senaoSe",
                "senao",
                "fimSe",
                "escolha",
                "caso",
                "outroCaso",
                "fimEscolha",
                "enquanto",
                "faca",
                "fimEnquanto",
                "para",
                "de",
                "ate",
                "passo",
                "fimPara",
                "interrompa",
                "verdadeiro",
                "falso",
                "e",
                "ou",
                "nao",
            ],
            &[
                "escreva",
                "escreval",
                "leia",
                "leiaCaracter",
                "limpaTela",
                "abs",
                "aleatorio",
                "asciiCaracter",
                "caracterAscii",
                "caracterTexto",
                "comparaTexto",
                "convCaractTexto",
                "convTextoCaract",
                "copia",
                "exp",
                "maiusculo",
                "maiusculoCaracter",
                "minusculo",
                "minusculoCaracter",
                "pi",
                "raizQuadrada",
                "tamanhoTexto",
            ],
            &[],
        ),
        Language::Mapler => (&[], &[], &[]),
    }
}

const PORTUGOL_LIBRARIES: &[&str] = &[
    "Arquivos",
    "Calendario",
    "Graficos",
    "Internet",
    "Matematica",
    "Mouse",
    "Objetos",
    "ServicosWeb",
    "Sons",
    "Teclado",
    "Texto",
    "Tipos",
    "Util",
];

const POTIGOL_LIBRARIES: &[&str] = &["jerimum"];

const PORTUGOL_SNIPPETS: &[Snippet] = &[
    Snippet {
        label: "programa · estrutura inicial",
        text: "programa {\n  funcao inicio() {\n    escreva(\"⟪cursor⟫\")\n  }\n}",
    },
    Snippet {
        label: "se · condição",
        text: "se (⟪cursor⟫) {\n  \n} senao {\n  \n}",
    },
    Snippet {
        label: "enquanto · laço",
        text: "enquanto (⟪cursor⟫) {\n  \n}",
    },
    Snippet {
        label: "escreva",
        text: "escreva(\"⟪cursor⟫\")",
    },
];

const VISUALG_SNIPPETS: &[Snippet] = &[
    Snippet {
        label: "algoritmo · estrutura inicial",
        text: "algoritmo \"exemplo\"\nvar\n  ⟪cursor⟫: inteiro\ninicio\n  escreval(\"Olá!\")\nfimalgoritmo",
    },
    Snippet {
        label: "se · condição",
        text: "se ⟪cursor⟫ entao\n  \nsenao\n  \nfimse",
    },
    Snippet {
        label: "enquanto · laço",
        text: "enquanto ⟪cursor⟫ faca\n  \nfimenquanto",
    },
    Snippet {
        label: "escreval",
        text: "escreval(\"⟪cursor⟫\")",
    },
];

const POTIGOL_SNIPPETS: &[Snippet] = &[
    Snippet {
        label: "escreva",
        text: "escreva \"⟪cursor⟫\"",
    },
    Snippet {
        label: "se · condição",
        text: "se ⟪cursor⟫ então\n  escreva \"sim\"\nsenão\n  escreva \"não\"\nfim",
    },
    Snippet {
        label: "def · função",
        text: "def nome: Nada\n  ⟪cursor⟫\nfim",
    },
    Snippet {
        label: "use · biblioteca",
        text: "use \"⟪cursor⟫\"",
    },
];

const GPORTUGOL_SNIPPETS: &[Snippet] = &[
    Snippet {
        label: "algoritmo · estrutura inicial",
        text: "algoritmo exemplo;\ninício\n  imprima(\"⟪cursor⟫\");\nfim",
    },
    Snippet {
        label: "se · condição",
        text: "se ⟪cursor⟫ então\n  \nsenão\n  \nfim-se",
    },
    Snippet {
        label: "enquanto · laço",
        text: "enquanto ⟪cursor⟫ faça\n  \nfim-enquanto",
    },
    Snippet {
        label: "imprima",
        text: "imprima(\"⟪cursor⟫\");",
    },
];

fn current_word(chars: &[char], cursor: usize) -> (usize, String) {
    let mut start = cursor;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }
    (start, chars[start..cursor].iter().collect())
}

fn is_identifier_char(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

fn line_bounds(chars: &[char], cursor: usize) -> (usize, usize) {
    let start = chars[..cursor]
        .iter()
        .rposition(|character| *character == '\n')
        .map_or(0, |index| index + 1);
    let end = chars[cursor..]
        .iter()
        .position(|character| *character == '\n')
        .map_or(chars.len(), |index| cursor + index);
    (start, end)
}

fn potigol_library_context(chars: &[char], cursor: usize) -> Option<(usize, String, bool)> {
    let (line_start, line_end) = line_bounds(chars, cursor);
    let before: String = chars[line_start..cursor].iter().collect();
    let after: String = chars[cursor..line_end].iter().collect();
    let trimmed = before.trim_start();
    let rest = trimmed.strip_prefix("use")?;
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let whitespace = rest
        .chars()
        .take_while(|character| character.is_whitespace())
        .count();
    let after_use: String = rest.chars().skip(whitespace).collect();
    if !after_use.starts_with('"') || after_use[1..].contains('"') {
        return None;
    }
    let leading = before
        .chars()
        .take_while(|character| character.is_whitespace())
        .count();
    let quote = line_start + leading + 3 + whitespace;
    let start = quote + 1;
    let prefix: String = chars[start..cursor].iter().collect();
    Some((start, prefix, !after.contains('"')))
}

fn portugol_library_context(chars: &[char], cursor: usize) -> Option<(usize, String)> {
    let (line_start, _) = line_bounds(chars, cursor);
    let before: String = chars[line_start..cursor].iter().collect();
    let trimmed = before.trim_start();
    let rest = trimmed.strip_prefix("inclua biblioteca")?;
    if !rest.chars().next().is_some_and(char::is_whitespace) || rest.contains("-->") {
        return None;
    }
    let (word_start, prefix) = current_word(chars, cursor);
    if prefix.is_empty() && !rest.trim().is_empty() {
        return None;
    }
    Some((word_start, prefix))
}

fn dot_context(chars: &[char], cursor: usize) -> Option<(String, String)> {
    let (word_start, prefix) = current_word(chars, cursor);
    if word_start == 0 || chars[word_start - 1] != '.' {
        return None;
    }
    let owner_end = word_start - 1;
    let mut owner_start = owner_end;
    while owner_start > 0 && is_identifier_char(chars[owner_start - 1]) {
        owner_start -= 1;
    }
    (owner_start < owner_end).then(|| (chars[owner_start..owner_end].iter().collect(), prefix))
}

fn is_in_comment(language: Language, chars: &[char], cursor: usize) -> bool {
    let mut index = 0;
    while index < cursor {
        if chars[index] == '"' || chars[index] == '\'' {
            let quote = chars[index];
            index += 1;
            while index < cursor {
                if chars[index] == '\\' {
                    index = (index + 2).min(cursor);
                    continue;
                }
                let closes = chars[index] == quote;
                index += 1;
                if closes {
                    break;
                }
            }
            continue;
        }
        if chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            if index == cursor {
                return true;
            }
            index += 1;
            continue;
        }
        if matches!(
            language,
            Language::Portugol | Language::GPortugol | Language::Calango
        ) && chars[index] == '/'
            && chars.get(index + 1) == Some(&'*')
        {
            index += 2;
            while index + 1 < cursor && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            if index + 1 >= cursor {
                return true;
            }
            index += 2;
            continue;
        }
        if language == Language::Potigol && chars[index] == '#' {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            if index == cursor {
                return true;
            }
            index += 1;
            continue;
        }
        if language == Language::Visualg && chars[index] == '{' {
            while index < cursor && chars[index] != '}' {
                index += 1;
            }
            if index == cursor {
                return true;
            }
            index += 1;
            continue;
        }
        index += 1;
    }
    false
}

fn is_in_string(language: Language, chars: &[char], cursor: usize) -> bool {
    let mut index = 0;
    let mut quote = None;
    while index < cursor {
        if let Some(active_quote) = quote {
            if chars[index] == '\\' {
                index = (index + 2).min(cursor);
                continue;
            }
            if chars[index] == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            index += usize::from(index < cursor);
            continue;
        }
        if matches!(
            language,
            Language::Portugol | Language::GPortugol | Language::Calango
        ) && chars[index] == '/'
            && chars.get(index + 1) == Some(&'*')
        {
            index += 2;
            while index + 1 < cursor && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            index = (index + 2).min(cursor);
            continue;
        }
        if language == Language::Potigol && chars[index] == '#' {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            index += usize::from(index < cursor);
            continue;
        }
        if language == Language::Visualg && chars[index] == '{' {
            while index < cursor && chars[index] != '}' {
                index += 1;
            }
            if index < cursor {
                index += 1;
            }
            continue;
        }
        if matches!(chars[index], '"' | '\'') {
            quote = Some(chars[index]);
        }
        index += 1;
    }
    quote.is_some()
}

fn portugol_imports(source: &str, cursor: usize) -> Vec<(String, String)> {
    let prefix: String = source.chars().take(cursor).collect();
    prefix
        .lines()
        .filter_map(|line| {
            let line = line.split("//").next().unwrap_or_default().trim();
            let declaration = line.strip_prefix("inclua biblioteca ")?;
            let mut parts = declaration.split_whitespace();
            let library = parts.next()?.trim_end_matches(';');
            if !PORTUGOL_LIBRARIES.contains(&library) {
                return None;
            }
            let remaining: Vec<_> = parts.collect();
            let alias = remaining
                .windows(2)
                .find(|parts| parts[0] == "-->")
                .and_then(|parts| parts.get(1).copied())
                .unwrap_or(library);
            Some((library.to_owned(), alias.trim_end_matches(';').to_owned()))
        })
        .collect()
}

fn portugol_library_members(library: &str) -> &'static [&'static str] {
    match library {
        "Matematica" => &[
            "PI",
            "arredondar",
            "potencia",
            "raiz",
            "logaritmo",
            "maior_numero",
            "menor_numero",
            "seno",
            "cosseno",
            "tangente",
            "valor_absoluto",
        ],
        "Texto" => &[
            "numero_caracteres",
            "caixa_alta",
            "caixa_baixa",
            "substituir",
            "preencher_a_esquerda",
            "obter_caracter",
            "posicao_texto",
            "extrair_subtexto",
        ],
        "Tipos" => &[
            "cadeia_e_real",
            "cadeia_e_logico",
            "cadeia_e_caracter",
            "cadeia_para_caracter",
            "cadeia_para_real",
            "cadeia_para_logico",
            "cadeia_e_inteiro",
            "cadeia_para_inteiro",
            "inteiro_e_caracter",
            "inteiro_para_caracter",
            "inteiro_para_logico",
            "inteiro_para_real",
            "inteiro_para_cadeia",
            "caracter_e_inteiro",
            "caracter_e_logico",
            "caracter_para_cadeia",
            "caracter_para_inteiro",
            "caracter_para_logico",
            "logico_para_cadeia",
            "logico_para_inteiro",
            "logico_para_caracter",
            "real_para_cadeia",
            "real_para_inteiro",
        ],
        "Util" => &[
            "numero_elementos",
            "numero_linhas",
            "numero_colunas",
            "sorteia",
            "aguarde",
            "tempo_decorrido",
            "obter_diretorio_usuario",
        ],
        "Calendario" => &[
            "dia_mes_atual",
            "dia_semana_atual",
            "mes_atual",
            "ano_atual",
            "hora_atual",
            "minuto_atual",
            "segundo_atual",
            "milisegundo_atual",
            "dia_semana_completo",
            "dia_semana_curto",
            "dia_semana_abreviado",
        ],
        "Arquivos" => &[
            "abrir_arquivo",
            "escrever_linha",
            "fechar_arquivo",
            "fim_arquivo",
            "ler_linha",
            "substituir_texto",
            "arquivo_existe",
            "apagar_arquivo",
            "criar_pasta",
            "selecionar_arquivo",
            "listar_pastas",
            "listar_arquivos",
            "listar_arquivos_por_tipo",
        ],
        "Objetos" => &[
            "criar_objeto",
            "criar_objeto_via_json",
            "criar_objeto_via_xml",
            "liberar",
            "liberar_objeto",
            "obter_json",
            "atribuir_propriedade",
            "obter_propriedade_tipo_inteiro",
            "obter_propriedade_tipo_real",
            "obter_propriedade_tipo_logico",
            "obter_propriedade_tipo_caracter",
            "obter_propriedade_tipo_cadeia",
            "contem_propriedade",
            "tipo_propriedade",
        ],
        "Internet" => &[
            "definir_tempo_limite",
            "obter_texto",
            "endereco_disponivel",
            "baixar_imagem",
        ],
        "ServicosWeb" => &[
            "obter_dados",
            "excluir_dados",
            "abrir_conexao",
            "adicionar_parametros",
            "publicar_dados",
            "atualizar_dados",
            "adicionar_cabecalho",
            "fazer_requisicao",
        ],
        "Mouse" => &[
            "botao_pressionado",
            "algum_botao_pressionado",
            "ler_botao",
            "posicao_x",
            "posicao_y",
            "ocultar_cursor",
            "exibir_cursor",
        ],
        "Teclado" => &[
            "tecla_pressionada",
            "alguma_tecla_pressionada",
            "ler_tecla",
            "caracter_tecla",
        ],
        "Sons" => &[
            "carregar_som",
            "liberar_som",
            "pausar_som",
            "interromper_som",
            "definir_volume",
            "reproduzir_som",
            "obter_volume",
        ],
        "Graficos" => &[
            "iniciar_modo_grafico",
            "definir_dimensoes_janela",
            "definir_titulo_janela",
            "fechar_janela",
            "desenhar_texto",
            "desenhar_linha",
            "desenhar_retangulo",
            "desenhar_elipse",
            "desenhar_ponto",
            "desenhar_imagem",
        ],
        _ => &[],
    }
}

fn potigol_jerimum_imported(source: &str, cursor: usize) -> bool {
    source
        .chars()
        .take(cursor)
        .collect::<String>()
        .lines()
        .any(|line| line.trim() == "use \"jerimum\"")
}

fn potigol_members(owner: &str) -> &'static [&'static str] {
    match owner.to_lowercase().as_str() {
        "jogo" => &[
            "titulo",
            "largura",
            "altura",
            "fps",
            "iniciar",
            "distância",
            "distancia",
            "projeção_X",
            "projeção_Y",
        ],
        "teclado" => &[
            "TECLA_ESPACO",
            "TECLA_ENTER",
            "TECLA_PARA_CIMA",
            "TECLA_PARA_BAIXO",
            "TECLA_PARA_ESQUERDA",
            "TECLA_PARA_DIREITA",
            "TECLA_W",
            "TECLA_A",
            "TECLA_S",
            "TECLA_D",
        ],
        "mouse" => &["BOTAO_ESQUERDO", "BOTAO_DIREITO", "x", "y"],
        "relógio" | "relogio" => &["agora", "milisegundos"],
        "desenho" => &["incluir", "desenhe"],
        "cor" => &["vermelho", "verde", "azul", "color"],
        "imagem" => &["largura", "altura", "caminho", "buffer", "fatie"],
        "animacao" | "animação" => &["velocidade", "imagens", "imagem"],
        "fonte" => &["tamanho"],
        "som" => &["arquivo", "tocando", "pare"],
        "musica" | "música" => &[
            "arquivo", "tocando", "pausado", "pare", "pause", "continue", "feche",
        ],
        "tela" => &["frame", "canvas"],
        "graphics2d" => &[
            "setColor",
            "getColor",
            "drawLine",
            "drawRect",
            "fillRect",
            "drawOval",
            "fillOval",
            "drawString",
            "drawImage",
        ],
        _ => &[],
    }
}

fn declared_identifiers(language: Language, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let types = match language {
        Language::Portugol => &["inteiro", "real", "logico", "caracter", "cadeia"][..],
        Language::Visualg => &[
            "inteiro",
            "real",
            "logico",
            "caractere",
            "caracter",
            "literal",
        ][..],
        Language::Potigol => &["var", "const"][..],
        Language::GPortugol => &[
            "inteiro",
            "reais",
            "caractere",
            "literais",
            "lógico",
            "logico",
        ][..],
        Language::Calango => &["inteiro", "real", "logico", "caracter", "texto"][..],
        Language::Mapler => &[][..],
    };
    for line in source.lines() {
        let line = line
            .split("//")
            .next()
            .unwrap_or_default()
            .split('#')
            .next()
            .unwrap_or_default()
            .trim();
        if line.is_empty() {
            continue;
        }
        let declaration = match language {
            Language::Potigol => line
                .strip_prefix("var ")
                .or_else(|| line.strip_prefix("const "))
                .or_else(|| line.strip_prefix("def ")),
            Language::Visualg if line.contains(':') => line.split(':').next(),
            Language::Visualg => line
                .strip_prefix("funcao ")
                .or_else(|| line.strip_prefix("procedimento ")),
            Language::Mapler => line.split(':').next(),
            Language::Calango => {
                let mut parts = line.split_whitespace();
                let first = parts.next().unwrap_or_default();
                if types.contains(&first) {
                    parts.next()
                } else if first == "funcao" || first == "procedimento" {
                    parts.find(|part| !types.contains(part))
                } else {
                    None
                }
            }
            Language::Portugol | Language::GPortugol => {
                let mut parts = line.split_whitespace();
                let first = parts.next().unwrap_or_default();
                if first == "funcao" || first == "função" {
                    parts.next()
                } else if types.contains(&first) {
                    parts.next()
                } else {
                    None
                }
            }
        };
        let Some(declaration) = declaration else {
            continue;
        };
        let name = identifier_at_start(declaration);
        if !name.is_empty() && !names.iter().any(|existing| existing == name) {
            names.push(name.to_owned());
        }
    }
    names
}

fn identifier_at_start(source: &str) -> &str {
    let source = source.trim_start();
    let end = source
        .char_indices()
        .find_map(|(index, character)| (!is_identifier_char(character)).then_some(index))
        .unwrap_or(source.len());
    &source[..end]
}
