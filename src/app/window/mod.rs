mod analysis;
mod component;
mod dialogs;
mod documents;
mod execution_events;
mod learning;
mod localization;
mod messages;
mod persistence;
mod project;
mod settings;
mod view;

#[cfg(test)]
use self::analysis::{CurrentLearningAnalysis, LearningAnalysisIdentity, merge_editor_diagnostics};
use self::analysis::{
    DiagnosticNavigation, DiagnosticsAnalysisWorker, LearningAnalysisWorker,
    invalidate_learning_hint, project_scan_is_current,
};
use self::dialogs::*;
use self::learning::*;
use self::localization::{ThemePreference, theme_is_dark};
#[cfg(test)]
use self::localization::{UI_TRANSLATION_KEYS, translate_known_text};
pub(crate) use self::localization::{UiLanguage, tr};
pub(crate) use self::messages::{MainWindowMsg, RightPanelSection, should_persist_after};
use self::persistence::*;
use self::project::*;
#[cfg(test)]
use self::view::input_cancel_controller;

use crate::features::diagnostics::{self, AnalysisIdentity, EditorDiagnostic};
use crate::features::execution::{
    self, ControlAction, DebuggerSnapshot, ExecutionEvent, InputPrompt,
};
use crate::features::learning::{
    LearningHint, LearningSymbol, LearningTopic, assignment_operator, catalog,
    learning_hint_with_runtime_cancellable,
};
use crate::languages::model::{Language, ProjectTemplate};
use crate::ui::{
    components::{
        content_panel::{self, SourceDocument, SyntaxTags},
        header, sidebar,
        sidebar::ProjectFile,
        start_page,
    },
    helpers::icon,
};
use gtk::prelude::*;
use relm4::prelude::*;
use sourceview5::prelude::*;
use std::io::{self, Write};
use std::{
    cell::{Cell, RefCell},
    fs,
    fs::OpenOptions,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::Sender,
    },
    time::Duration,
};

const MAX_EXECUTION_EVENTS_PER_TICK: usize = 128;
pub(crate) struct MainWindow {
    window: gtk::ApplicationWindow,
    audio_host: crate::features::media::audio::AudioHost,
    graphics_host: crate::features::media::graphics::GraphicsHost,
    new_file_language: Language,
    documents: Vec<SourceDocument>,
    active_document_id: Option<u64>,
    next_document_id: u64,
    project_root: Option<PathBuf>,
    project_name: String,
    project_files: Vec<ProjectFile>,
    project_generation: u64,
    project_scan_generation: u64,
    pending_project: Option<PathBuf>,
    pending_project_scan: Option<(PathBuf, Vec<ProjectFile>)>,
    approved_project_switch: Option<PathBuf>,
    project_load_pending: bool,
    project_refresh_needed: bool,
    pending_close_document_id: Option<u64>,
    pending_close_after_save_id: Option<u64>,
    pending_save_as_id: Option<u64>,
    pending_ui_language: Option<UiLanguage>,
    pending_language_save_as_id: Option<u64>,
    reopen_settings_after_language_restart: bool,
    pending_close_app: bool,
    close_app_ready: bool,
    ui_language: UiLanguage,
    preferred_ui_language: UiLanguage,
    theme: ThemePreference,
    dark: bool,
    font_size: u32,
    line_numbers_visible: bool,
    word_wrap: bool,
    indent_width: u32,
    recovery_generation: u64,
    settings_visible: bool,
    output_text: String,
    output_status: String,
    problem_text: String,
    debugger_error: bool,
    game_preview_active: bool,
    next_execution_id: u64,
    active_execution_id: Option<u64>,
    debugging_active: bool,
    execution_document_id: Option<u64>,
    execution_line: Option<usize>,
    execution_project_root: Option<PathBuf>,
    debugger_document_id: Option<u64>,
    execution_input: Option<Sender<String>>,
    execution_control: Option<Sender<ControlAction>>,
    execution_cancellable: Option<gtk::gio::Cancellable>,
    file_selection_cancellable: Option<(u64, gtk::gio::Cancellable)>,
    input_requested: bool,
    input_prompt: Option<InputPrompt>,
    debugger_snapshot: Option<DebuggerSnapshot>,
    debugger_learning_hint: Option<LearningHint>,
    execution_diagnostics: Vec<execution::ExecutionDiagnostic>,
    execution_diagnostics_document_id: Option<u64>,
    execution_source_fingerprint: Option<u64>,
    diagnostics_generation: u64,
    diagnostics_worker: DiagnosticsAnalysisWorker,
    diagnostics_active_document_id: Option<u64>,
    diagnostics_pending: Option<AnalysisIdentity>,
    navigation_serial: u64,
    navigation_target: Option<DiagnosticNavigation>,
    learning_generation: u64,
    learning_revision: u64,
    learning_cursor_line: usize,
    learning_document_id: Option<u64>,
    learning_language: Option<Language>,
    learning_hint: Option<LearningHint>,
    learning_loading: bool,
    right_panel_section: RightPanelSection,
    right_panel_collapsed: bool,
    learning_worker: LearningAnalysisWorker,
    recent_paths: Vec<PathBuf>,
    start_page_visible: bool,
}

fn execution_reply_is_current(reply_execution_id: u64, active_execution_id: Option<u64>) -> bool {
    active_execution_id == Some(reply_execution_id)
}

#[cfg(test)]
mod ui_language_tests {
    use super::{
        UI_TRANSLATION_KEYS, UiLanguage, learning_copy, merge_editor_diagnostics, new_source_path,
        normalized_project_item_name, project_scan_is_current, supported_save_path, tr,
        translate_known_text,
    };
    use crate::features::learning::{LearningHint, LearningSymbol, LearningTopic, learning_hint};
    use crate::languages::model::Language;
    use std::path::{Path, PathBuf};

    #[test]
    fn detects_supported_system_locales() {
        assert_eq!(
            UiLanguage::from_locale_name("pt_BR.UTF-8"),
            Some(UiLanguage::Portuguese)
        );
        assert_eq!(
            UiLanguage::from_locale_name("es-MX"),
            Some(UiLanguage::Spanish)
        );
        assert_eq!(
            UiLanguage::from_locale_name("en_GB"),
            Some(UiLanguage::English)
        );
        assert_eq!(UiLanguage::from_locale_name("fr_FR"), None);
    }

    #[test]
    fn uses_first_supported_system_language_or_english() {
        assert_eq!(
            UiLanguage::from_language_names(["fr_FR", "es_MX", "pt_BR"]),
            UiLanguage::Spanish
        );
        assert_eq!(
            UiLanguage::from_language_names(["fr_FR", "C"]),
            UiLanguage::English
        );
    }

    #[test]
    fn saved_language_preference_takes_precedence_over_system_language() {
        assert_eq!(
            UiLanguage::from_preference(Some("en"), ["pt_BR"]),
            UiLanguage::English
        );
        assert_eq!(
            UiLanguage::from_preference(Some("pt-BR"), ["en_US"]),
            UiLanguage::Portuguese
        );
        assert_eq!(
            UiLanguage::from_preference(None, ["es_ES"]),
            UiLanguage::Spanish
        );
        assert_eq!(
            UiLanguage::from_preference(Some("invalid"), ["fr_FR"]),
            UiLanguage::English
        );
    }

    #[test]
    fn translates_existing_interface_copy_between_supported_languages() {
        for key in UI_TRANSLATION_KEYS {
            for from in [
                UiLanguage::Portuguese,
                UiLanguage::English,
                UiLanguage::Spanish,
            ] {
                for to in [
                    UiLanguage::Portuguese,
                    UiLanguage::English,
                    UiLanguage::Spanish,
                ] {
                    let source_text = tr(from, key);
                    let canonical_key = UI_TRANSLATION_KEYS
                        .iter()
                        .find(|candidate| tr(from, candidate) == source_text)
                        .expect("every source translation must have a catalog key");
                    assert_eq!(
                        translate_known_text(from, to, source_text),
                        Some(tr(to, canonical_key).to_string()),
                        "failed translating {key:?} from {from:?} to {to:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn gportugol_creation_save_and_rename_preserve_dialect_extensions() {
        let root = Path::new("/tmp/potoo-project");
        let (created, language) = new_source_path(
            root,
            "principal",
            Language::GPortugol,
            UiLanguage::Portuguese,
        )
        .expect("a G-Portugol starter file should be created with its own extension");
        assert_eq!(created, root.join("principal.gpt"));
        assert_eq!(language, Language::GPortugol);

        let (saved, language) = supported_save_path(
            PathBuf::from("/tmp/edited.gpt"),
            Language::Portugol,
            UiLanguage::Portuguese,
        )
        .expect("G-Portugol files should be saveable");
        assert_eq!(saved, PathBuf::from("/tmp/edited.gpt"));
        assert_eq!(language, Language::GPortugol);
        assert_eq!(
            normalized_project_item_name(&created, "renamed", false, UiLanguage::Portuguese),
            Ok("renamed.gpt".to_string())
        );
        assert_eq!(
            new_source_path(
                root,
                "main.por",
                Language::GPortugol,
                UiLanguage::Portuguese
            )
            .expect("the .por extension must remain associated with Portugol Studio 2.0")
            .1,
            Language::Portugol
        );
    }

    #[test]
    fn learning_copy_teaches_dialect_types_and_array_shapes() {
        let portugol = "programa {\n funcao vazio registrar(inteiro valor) {}\n funcao cadeia nome(inteiro valor) { retorne \"Ana\" }\n funcao vazio inicio() {\n  inteiro valores[3]\n  inteiro matriz[2][3]\n }\n}";
        let void_function = learning_hint(portugol, 2, Language::Portugol);
        let (void_text, _, _) = learning_copy(&void_function, UiLanguage::Portuguese);
        assert!(void_text.contains("vazio"));
        assert!(void_text.contains("não devolve um valor"));
        let (_, _, void_experiment) = learning_copy(&void_function, UiLanguage::Portuguese);
        assert!(void_experiment.contains("valor"));
        assert!(void_experiment.contains("ações"));
        assert!(!void_experiment.contains("resultado devolvido"));

        let text_function = learning_hint(portugol, 3, Language::Portugol);
        let (text_explanation, text_concept, _) =
            learning_copy(&text_function, UiLanguage::English);
        assert!(text_explanation.contains("cadeia"));
        assert!(text_concept.contains("text"));
        let (_, _, function_experiment) = learning_copy(&text_function, UiLanguage::Portuguese);
        assert!(function_experiment.contains("valor"));

        let vector = learning_hint(portugol, 5, Language::Portugol);
        let (vector_text, _, vector_experiment) = learning_copy(&vector, UiLanguage::Portuguese);
        assert!(vector_text.contains("vetor com 3 posições"));
        assert!(vector_experiment.contains("valores"));
        assert!(vector_experiment.contains("elemento"));

        let matrix = learning_hint(portugol, 6, Language::Portugol);
        let (matrix_text, _, _) = learning_copy(&matrix, UiLanguage::Portuguese);
        assert!(matrix_text.contains("matriz com 2 dimensões"));
        assert_eq!(matrix.topic, LearningTopic::Variable);

        let visualg =
            "algoritmo \"aula\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\nfimalgoritmo\n";
        let vector = learning_hint(visualg, 3, Language::Visualg);
        let (visualg_text, _, _) = learning_copy(&vector, UiLanguage::Portuguese);
        assert!(visualg_text.contains("índices de 2 até 5"));
    }

    #[test]
    fn shared_copy_composer_covers_every_topic_with_explicit_facts() {
        fn hint(topic: LearningTopic, name: Option<&str>) -> LearningHint {
            LearningHint {
                line: 1,
                column: 1,
                language: Language::Portugol,
                topic,
                name: name.map(str::to_owned),
                return_type: None,
                target: None,
                parameters: Vec::new(),
                variables: Vec::new(),
                expression: None,
                experiment_expression: None,
                call: None,
                problems: Vec::new(),
            }
        }

        fn symbol(name: &str, initializer: Option<&str>) -> LearningSymbol {
            LearningSymbol {
                name: name.to_owned(),
                type_name: "inteiro".to_owned(),
                is_constant: false,
                by_reference: false,
                dimensions: Vec::new(),
                initializer: initializer.map(str::to_owned),
            }
        }

        let mut function = hint(LearningTopic::Function, Some("somar"));
        function.return_type = Some("inteiro".to_owned());
        function.parameters.push(symbol("valor", None));

        let mut procedure = hint(LearningTopic::Procedure, Some("registrar"));
        procedure.parameters.push(symbol("valor", None));

        let mut decision = hint(LearningTopic::Decision, Some("se"));
        decision.expression = Some("idade >= 18".to_owned());
        decision.experiment_expression = Some("idade >= 16".to_owned());

        let mut loop_hint = hint(LearningTopic::Loop, Some("enquanto"));
        loop_hint.expression = Some("idade > 0".to_owned());
        loop_hint.experiment_expression = Some("idade > 2".to_owned());

        let mut input = hint(LearningTopic::Input, Some("leia"));
        input.target = Some("idade".to_owned());

        let mut output = hint(LearningTopic::Output, Some("escreva"));
        output.expression = Some("idade".to_owned());

        let mut variable = hint(LearningTopic::Variable, None);
        variable.variables.push(symbol("idade", Some("18")));

        let mut assignment = hint(LearningTopic::Assignment, Some("idade"));
        assignment.expression = Some("idade + 1".to_owned());

        let mut call = hint(LearningTopic::Call, Some("somar"));
        call.call = Some(crate::features::learning::SourceCall {
            receiver: None,
            member: "somar".to_owned(),
            arguments: vec!["idade".to_owned()],
            library: None,
        });

        let mut unknown_library = hint(LearningTopic::LibraryCall, Some("limpar"));
        unknown_library.call = Some(crate::features::learning::SourceCall {
            receiver: Some("tela".to_owned()),
            member: "limpar".to_owned(),
            arguments: Vec::new(),
            library: Some("biblioteca-desconhecida".to_owned()),
        });

        let import = hint(LearningTopic::Import, Some("jerimum"));

        let mut return_hint = hint(LearningTopic::Return, Some("retorne"));
        return_hint.return_type = Some("inteiro".to_owned());
        return_hint.expression = Some("valor * 2".to_owned());

        let mut program = hint(LearningTopic::Program, None);
        program.expression = Some("inicio".to_owned());

        let unrecognized = hint(LearningTopic::Unrecognized, None);

        let cases = [
            (
                "Function",
                function,
                "somar",
                Some("inteiro"),
                Some("valor"),
            ),
            (
                "Procedure",
                procedure,
                "registrar",
                Some("rotina"),
                Some("valor"),
            ),
            (
                "Decision",
                decision,
                "idade >= 18",
                Some("verdadeira ou falsa"),
                Some("idade >= 16"),
            ),
            (
                "Loop",
                loop_hint,
                "idade > 0",
                Some("idade > 0"),
                Some("idade > 2"),
            ),
            (
                "Input",
                input,
                "idade",
                Some("tipo solicitado"),
                Some("idade"),
            ),
            (
                "Output",
                output,
                "idade",
                Some("quebra de linha"),
                Some("idade"),
            ),
            (
                "Variable",
                variable,
                "idade",
                Some("tipo define"),
                Some("idade"),
            ),
            (
                "Assignment",
                assignment,
                "idade + 1",
                Some("expressão à direita"),
                Some("idade + 1"),
            ),
            (
                "Call",
                call,
                "somar(idade)",
                Some("transfere a execução"),
                Some("somar(idade)"),
            ),
            (
                "LibraryCall",
                unknown_library,
                "descrição verificada",
                None,
                None,
            ),
            ("Import", import, "jerimum", Some("biblioteca"), None),
            (
                "Return",
                return_hint,
                "valor * 2",
                Some("resultado da função"),
                Some("valor * 2"),
            ),
            ("Program", program, "inicio", None, None),
            (
                "Unrecognized",
                unrecognized,
                "não contém uma instrução reconhecida",
                None,
                None,
            ),
        ];

        for (topic, hint, explanation_fact, concept_fact, experiment_fact) in cases {
            let (explanation, concept, experiment) = learning_copy(&hint, UiLanguage::Portuguese);
            assert!(
                explanation.contains(explanation_fact),
                "{topic} explanation: {explanation}"
            );
            match concept_fact {
                Some(fact) => assert!(concept.contains(fact), "{topic} concept: {concept}"),
                None => assert!(concept.is_empty(), "{topic} concept: {concept}"),
            }
            match experiment_fact {
                Some(fact) => {
                    assert!(
                        experiment.contains(fact),
                        "{topic} experiment: {experiment}"
                    )
                }
                None => assert!(experiment.is_empty(), "{topic} experiment: {experiment}"),
            }
        }
    }

    const COPY_AUDIT_TOPICS: [LearningTopic; 14] = [
        LearningTopic::Function,
        LearningTopic::Procedure,
        LearningTopic::Decision,
        LearningTopic::Loop,
        LearningTopic::Input,
        LearningTopic::Output,
        LearningTopic::Variable,
        LearningTopic::Assignment,
        LearningTopic::Call,
        LearningTopic::LibraryCall,
        LearningTopic::Import,
        LearningTopic::Return,
        LearningTopic::Program,
        LearningTopic::Unrecognized,
    ];

    struct CopyTopicEvidence {
        topic: LearningTopic,
        test: fn(),
    }

    struct ProfileCopyAudit {
        fixture: &'static str,
        topic_evidence: &'static [CopyTopicEvidence],
        not_applicable_topics: &'static [LearningTopic],
        not_applicable_reason: &'static str,
    }

    const PROFILE_COPY_AUDIT: [ProfileCopyAudit; 7] = [
        ProfileCopyAudit {
            fixture: "Portugol Studio 2.0",
            topic_evidence: &[
                CopyTopicEvidence { topic: LearningTopic::Function, test: learning_copy_teaches_dialect_types_and_array_shapes },
                CopyTopicEvidence { topic: LearningTopic::Decision, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Loop, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Input, test: learning_copy_explains_potigol_input_types_and_dialects },
                CopyTopicEvidence { topic: LearningTopic::Output, test: learning_copy_explains_output_line_behavior_in_each_dialect },
                CopyTopicEvidence { topic: LearningTopic::Variable, test: learning_copy_teaches_dialect_types_and_array_shapes },
                CopyTopicEvidence { topic: LearningTopic::Assignment, test: learning_copy_uses_each_dialects_assignment_operator_and_target },
                CopyTopicEvidence { topic: LearningTopic::Call, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Return, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Program, test: learning_copy_preserves_nested_output_expression_and_describes_program_structure },
                CopyTopicEvidence { topic: LearningTopic::Unrecognized, test: learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes },
            ],
            not_applicable_topics: &[LearningTopic::Procedure, LearningTopic::LibraryCall, LearningTopic::Import],
            not_applicable_reason: "Studio 2.0 has no separate Procedure or Import fact, and its verified library-call catalog is empty.",
        },
        ProfileCopyAudit {
            fixture: "G-Portugol",
            topic_evidence: &[
                CopyTopicEvidence { topic: LearningTopic::Function, test: learning_copy_uses_gportugol_terms_and_types },
                CopyTopicEvidence { topic: LearningTopic::Decision, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Loop, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Input, test: learning_copy_explains_potigol_input_types_and_dialects },
                CopyTopicEvidence { topic: LearningTopic::Output, test: learning_copy_explains_output_line_behavior_in_each_dialect },
                CopyTopicEvidence { topic: LearningTopic::Variable, test: learning_copy_uses_gportugol_terms_and_types },
                CopyTopicEvidence { topic: LearningTopic::Assignment, test: learning_copy_uses_each_dialects_assignment_operator_and_target },
                CopyTopicEvidence { topic: LearningTopic::Call, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Return, test: learning_copy_uses_gportugol_terms_and_types },
                CopyTopicEvidence { topic: LearningTopic::Program, test: learning_copy_uses_gportugol_terms_and_types },
                CopyTopicEvidence { topic: LearningTopic::Unrecognized, test: learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes },
            ],
            not_applicable_topics: &[LearningTopic::Procedure, LearningTopic::LibraryCall, LearningTopic::Import],
            not_applicable_reason: "G-Portugol has no separate Procedure, Import, or verified library-call fact in its current profile.",
        },
        ProfileCopyAudit {
            fixture: "Mapler",
            topic_evidence: &[
                CopyTopicEvidence { topic: LearningTopic::Decision, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Loop, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Input, test: learning_copy_explains_potigol_input_types_and_dialects },
                CopyTopicEvidence { topic: LearningTopic::Output, test: learning_copy_explains_output_line_behavior_in_each_dialect },
                CopyTopicEvidence { topic: LearningTopic::Variable, test: mapler_declarations_use_mapler_type_spelling_and_explain_the_value },
                CopyTopicEvidence { topic: LearningTopic::Assignment, test: learning_copy_uses_each_dialects_assignment_operator_and_target },
                CopyTopicEvidence { topic: LearningTopic::Call, test: mapler_module_calls_use_semicolon_syntax_and_module_guidance },
                CopyTopicEvidence { topic: LearningTopic::Unrecognized, test: learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes },
            ],
            not_applicable_topics: &[LearningTopic::Program, LearningTopic::Function, LearningTopic::Procedure, LearningTopic::LibraryCall, LearningTopic::Import, LearningTopic::Return],
            not_applicable_reason: "Mapler's current profile does not represent program headers, routines, returns, imports, or a library-call catalog.",
        },
        ProfileCopyAudit {
            fixture: "VisuAlg",
            topic_evidence: &[
                CopyTopicEvidence { topic: LearningTopic::Function, test: visualg_copy_teaches_routines_and_each_loop_form },
                CopyTopicEvidence { topic: LearningTopic::Procedure, test: visualg_copy_teaches_routines_and_each_loop_form },
                CopyTopicEvidence { topic: LearningTopic::Decision, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Loop, test: visualg_copy_teaches_routines_and_each_loop_form },
                CopyTopicEvidence { topic: LearningTopic::Input, test: learning_copy_explains_potigol_input_types_and_dialects },
                CopyTopicEvidence { topic: LearningTopic::Output, test: learning_copy_explains_output_line_behavior_in_each_dialect },
                CopyTopicEvidence { topic: LearningTopic::Variable, test: learning_copy_teaches_dialect_types_and_array_shapes },
                CopyTopicEvidence { topic: LearningTopic::Assignment, test: learning_copy_uses_each_dialects_assignment_operator_and_target },
                CopyTopicEvidence { topic: LearningTopic::Call, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Return, test: visualg_copy_teaches_routines_and_each_loop_form },
                CopyTopicEvidence { topic: LearningTopic::Program, test: visualg_copy_teaches_routines_and_each_loop_form },
                CopyTopicEvidence { topic: LearningTopic::Unrecognized, test: learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes },
            ],
            not_applicable_topics: &[LearningTopic::LibraryCall, LearningTopic::Import],
            not_applicable_reason: "VisuAlg has no verified import or library-call catalog in its current profile.",
        },
        ProfileCopyAudit {
            fixture: "Calango",
            topic_evidence: &[
                CopyTopicEvidence { topic: LearningTopic::Function, test: calango_copy_uses_program_routine_and_statement_facts },
                CopyTopicEvidence { topic: LearningTopic::Procedure, test: calango_copy_uses_program_routine_and_statement_facts },
                CopyTopicEvidence { topic: LearningTopic::Decision, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Loop, test: calango_copy_uses_program_routine_and_statement_facts },
                CopyTopicEvidence { topic: LearningTopic::Input, test: calango_copy_uses_program_routine_and_statement_facts },
                CopyTopicEvidence { topic: LearningTopic::Output, test: calango_copy_uses_program_routine_and_statement_facts },
                CopyTopicEvidence { topic: LearningTopic::Variable, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Assignment, test: calango_copy_uses_program_routine_and_statement_facts },
                CopyTopicEvidence { topic: LearningTopic::Call, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Return, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Program, test: calango_copy_uses_program_routine_and_statement_facts },
                CopyTopicEvidence { topic: LearningTopic::Unrecognized, test: learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes },
            ],
            not_applicable_topics: &[LearningTopic::LibraryCall, LearningTopic::Import],
            not_applicable_reason: "Calango has no verified import or library-call catalog in its current profile.",
        },
        ProfileCopyAudit {
            fixture: "Potigol Classic",
            topic_evidence: &[
                CopyTopicEvidence { topic: LearningTopic::Function, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Decision, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Loop, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Input, test: learning_copy_explains_potigol_input_types_and_dialects },
                CopyTopicEvidence { topic: LearningTopic::Output, test: learning_copy_explains_output_line_behavior_in_each_dialect },
                CopyTopicEvidence { topic: LearningTopic::Variable, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Assignment, test: potigol_classic_and_jerimum_copy_remain_import_aware },
                CopyTopicEvidence { topic: LearningTopic::Call, test: potigol_classic_and_jerimum_copy_remain_import_aware },
                CopyTopicEvidence { topic: LearningTopic::Import, test: potigol_classic_and_jerimum_copy_remain_import_aware },
                CopyTopicEvidence { topic: LearningTopic::Return, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Unrecognized, test: learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes },
            ],
            not_applicable_topics: &[LearningTopic::Program, LearningTopic::Procedure, LearningTopic::LibraryCall],
            not_applicable_reason: "Classic has no Program or Procedure AST node, and its calls have no active Jerimum import.",
        },
        ProfileCopyAudit {
            fixture: "Potigol Jerimum",
            topic_evidence: &[
                CopyTopicEvidence { topic: LearningTopic::Function, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Decision, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Loop, test: learning_copy_uses_each_dialects_condition_and_loop_kind },
                CopyTopicEvidence { topic: LearningTopic::Input, test: learning_copy_explains_potigol_input_types_and_dialects },
                CopyTopicEvidence { topic: LearningTopic::Output, test: learning_copy_explains_output_line_behavior_in_each_dialect },
                CopyTopicEvidence { topic: LearningTopic::Variable, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Assignment, test: potigol_classic_and_jerimum_copy_remain_import_aware },
                CopyTopicEvidence { topic: LearningTopic::Call, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::LibraryCall, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Import, test: potigol_classic_and_jerimum_copy_remain_import_aware },
                CopyTopicEvidence { topic: LearningTopic::Return, test: learning_copy_derives_calls_returns_and_variables_from_real_profiles },
                CopyTopicEvidence { topic: LearningTopic::Unrecognized, test: learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes },
            ],
            not_applicable_topics: &[LearningTopic::Program, LearningTopic::Procedure],
            not_applicable_reason: "Jerimum has no explicit Program or separate Procedure AST node; its library-call topic is import-aware.",
        },
    ];

    #[test]
    fn profile_copy_matrix_covers_supported_topics_with_real_facts_and_shared_sections() {
        let expected_fixtures = [
            "Portugol Studio 2.0",
            "G-Portugol",
            "Mapler",
            "VisuAlg",
            "Calango",
            "Potigol Classic",
            "Potigol Jerimum",
        ];
        assert_eq!(PROFILE_COPY_AUDIT.len(), expected_fixtures.len());
        assert_eq!(
            PROFILE_COPY_AUDIT.map(|entry| entry.fixture).as_slice(),
            expected_fixtures.as_slice()
        );

        let mut executed_tests = Vec::<fn()>::new();
        for entry in PROFILE_COPY_AUDIT {
            assert!(entry.not_applicable_reason.len() > 30, "{}", entry.fixture);
            for topic in COPY_AUDIT_TOPICS {
                let has_evidence = entry
                    .topic_evidence
                    .iter()
                    .any(|evidence| evidence.topic == topic);
                let is_not_applicable = entry.not_applicable_topics.contains(&topic);
                assert_ne!(
                    has_evidence, is_not_applicable,
                    "{} must provide real fixture evidence or an explicit N/A reason for {topic:?}",
                    entry.fixture
                );
            }

            for evidence in entry.topic_evidence {
                if !executed_tests
                    .iter()
                    .any(|test| std::ptr::fn_addr_eq(*test, evidence.test))
                {
                    (evidence.test)();
                    executed_tests.push(evidence.test);
                }
            }
        }

        // Synthetic facts cover only the shared explanation/concept/experiment composer.
        shared_copy_composer_covers_every_topic_with_explicit_facts();
    }

    #[test]
    fn learning_copy_uses_gportugol_terms_and_types() {
        let source = "algoritmo aula;\n\
variáveis\n\
  texto: literal;\n\
  valores: matriz[4] de caracteres;\n\
fim-variáveis\n\
início\n\
  imprima(\"Olá\");\n\
fim\n\
função nome(): literal\n\
início\n\
  retorne \"Ana\";\n\
fim";

        let program = learning_hint(source, 1, Language::GPortugol);
        assert_eq!(program.topic, LearningTopic::Program);
        assert!(
            learning_copy(&program, UiLanguage::Portuguese)
                .0
                .contains("algoritmo")
        );

        let (text_variable, _, _) = learning_copy(
            &learning_hint(source, 3, Language::GPortugol),
            UiLanguage::English,
        );
        assert!(text_variable.contains("literal"));
        assert!(text_variable.contains("type used to store text"));

        let (vector, _, _) = learning_copy(
            &learning_hint(source, 4, Language::GPortugol),
            UiLanguage::Portuguese,
        );
        assert!(vector.contains("índices de 0 até 3"));

        let (output, _, _) = learning_copy(
            &learning_hint(source, 7, Language::GPortugol),
            UiLanguage::English,
        );
        assert!(output.contains("imprima"));
        assert!(!output.contains("escreva"));

        let (function, concept, _) = learning_copy(
            &learning_hint(source, 9, Language::GPortugol),
            UiLanguage::English,
        );
        assert!(function.contains("literal"));
        assert!(concept.contains("literal"));
        assert!(concept.contains("G-Portugol"));

        let (return_text, _, return_experiment) = learning_copy(
            &learning_hint(source, 11, Language::GPortugol),
            UiLanguage::Portuguese,
        );
        assert!(return_text.contains("\"Ana\""));
        assert!(return_experiment.contains("\"Ana\""));
    }

    #[test]
    fn learning_copy_derives_calls_returns_and_variables_from_real_profiles() {
        let cases = [
            (
                "Portugol Studio 2.0 return",
                "programa {\n funcao inteiro soma(inteiro a, inteiro b) {\n  retorne a + b\n }\n funcao inicio() {\n  inteiro total = 1\n  soma(total, 2)\n }\n}",
                3,
                Language::Portugol,
                LearningTopic::Return,
            ),
            (
                "Portugol Studio 2.0 variable",
                "programa {\n funcao inicio() {\n  inteiro total = 1\n }\n}",
                3,
                Language::Portugol,
                LearningTopic::Variable,
            ),
            (
                "Portugol Studio 2.0 call",
                "programa {\n funcao inicio() {\n  somar(1, 2)\n }\n}",
                3,
                Language::Portugol,
                LearningTopic::Call,
            ),
            (
                "G-Portugol call",
                "algoritmo aula;\ninício\nsomar(1, 2);\nfim\nfunção soma(a: inteiro, b: inteiro): inteiro\ninício\nretorne a + b;\nfim",
                3,
                Language::GPortugol,
                LearningTopic::Call,
            ),
            (
                "VisuAlg call",
                "algoritmo \"aula\"\nprocedimento mostrar(valor: inteiro)\ninicio\nescreval(valor)\nfimprocedimento\ninicio\nmostrar(1)\nfimalgoritmo",
                7,
                Language::Visualg,
                LearningTopic::Call,
            ),
            (
                "Calango call",
                "algoritmo aula;\nfuncao inteiro dobrar(inteiro valor)\nretorna valor * 2;\nfimFuncao\nprincipal\ninteiro pontos;\ndobrar(pontos);\nfimPrincipal",
                7,
                Language::Calango,
                LearningTopic::Call,
            ),
            (
                "Calango variable",
                "algoritmo aula;\nprincipal\ninteiro pontos;\nfimPrincipal",
                3,
                Language::Calango,
                LearningTopic::Variable,
            ),
            (
                "Calango return",
                "algoritmo aula;\nfuncao inteiro dobrar(inteiro valor)\nretorna valor * 2;\nfimFuncao\nprincipal\nfimPrincipal",
                3,
                Language::Calango,
                LearningTopic::Return,
            ),
            (
                "Potigol Classic function",
                "def dobrar(valor: Inteiro): Inteiro\n  retorne valor * 2\nfim",
                1,
                Language::Potigol,
                LearningTopic::Function,
            ),
            (
                "Potigol Classic return",
                "def dobrar(valor: Inteiro): Inteiro\n  retorne valor * 2\nfim",
                2,
                Language::Potigol,
                LearningTopic::Return,
            ),
            (
                "Potigol Classic variable",
                "var pontos := 2",
                1,
                Language::Potigol,
                LearningTopic::Variable,
            ),
            (
                "Potigol Classic call",
                "somar(2, 3)",
                1,
                Language::Potigol,
                LearningTopic::Call,
            ),
            (
                "VisuAlg return",
                "algoritmo \"aula\"\nfuncao dobrar(valor: inteiro): inteiro\ninicio\nretorne valor * 2\nfimfuncao\ninicio\nfimalgoritmo",
                4,
                Language::Visualg,
                LearningTopic::Return,
            ),
            (
                "Potigol Jerimum call",
                "use \"jerimum\"\ndef desenhe_personagem(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim",
                3,
                Language::Potigol,
                LearningTopic::LibraryCall,
            ),
        ];

        for (fixture, source, line, language, expected) in cases {
            let hint = learning_hint(source, line, language);
            assert_eq!(hint.topic, expected, "{fixture}: {:?}", hint.problems);
            let (explanation, _, _) = learning_copy(&hint, UiLanguage::Portuguese);
            assert!(!explanation.is_empty(), "{fixture} produced no explanation");
        }
    }

    #[test]
    fn learning_copy_keeps_unrecognized_lines_unverified_in_all_fixture_modes() {
        let cases = [
            (
                "Portugol Studio 2.0",
                "programa {\n funcao inicio() {\n  ??\n }\n}",
                3,
                Language::Portugol,
            ),
            (
                "G-Portugol",
                "algoritmo aula;\ninício\n??;\nfim",
                3,
                Language::GPortugol,
            ),
            (
                "Mapler",
                "variaveis\ninicio\n??;\nfim.",
                3,
                Language::Mapler,
            ),
            (
                "VisuAlg",
                "algoritmo \"aula\"\ninicio\n??\nfimalgoritmo",
                3,
                Language::Visualg,
            ),
            (
                "Calango",
                "algoritmo aula;\nprincipal\n??;\nfimPrincipal",
                3,
                Language::Calango,
            ),
            ("Potigol Classic", "??", 1, Language::Potigol),
            (
                "Potigol Jerimum",
                "use \"jerimum\"\n??",
                2,
                Language::Potigol,
            ),
        ];

        for (fixture, source, line, language) in cases {
            let hint = learning_hint(source, line, language);
            assert_eq!(hint.topic, LearningTopic::Unrecognized, "{fixture}");
            let (explanation, concept, experiment) = learning_copy(&hint, UiLanguage::Portuguese);
            assert!(
                explanation.contains("não reconheceu"),
                "{fixture}: {explanation}"
            );
            assert!(concept.is_empty(), "{fixture}: {concept}");
            assert!(experiment.is_empty(), "{fixture}: {experiment}");
        }
    }

    #[test]
    fn learning_copy_explains_output_line_behavior_in_each_dialect() {
        let cases = [
            ("escreva \"Olá\"", 1, Language::Potigol, "escreva", true),
            ("imprima \"Olá\"", 1, Language::Potigol, "imprima", false),
            (
                "algoritmo \"aula\"\ninicio\nescreva(\"Olá\")\nfimalgoritmo",
                3,
                Language::Visualg,
                "escreva",
                false,
            ),
            (
                "algoritmo \"aula\"\ninicio\nescreval(\"Olá\")\nfimalgoritmo",
                3,
                Language::Visualg,
                "escreval",
                true,
            ),
            (
                "programa {\n funcao inicio() {\n  escreva(\"Olá\")\n }\n}",
                3,
                Language::Portugol,
                "escreva",
                false,
            ),
            (
                "algoritmo aula;\ninício\n imprima(\"Olá\");\nfim",
                3,
                Language::GPortugol,
                "imprima",
                true,
            ),
            (
                "algoritmo aula;\nprincipal\n escreva(\"Olá\");\nfimPrincipal",
                3,
                Language::Calango,
                "escreva",
                false,
            ),
            (
                "variaveis\n x: inteiro;\ninicio\n escrever \"Olá\";\nfim.",
                4,
                Language::Mapler,
                "escrever",
                true,
            ),
        ];

        for (source, line, dialect, command, newline) in cases {
            let hint = learning_hint(source, line, dialect);
            assert_eq!(hint.topic, LearningTopic::Output, "{dialect:?}: {source}");
            assert_eq!(hint.name.as_deref(), Some(command), "{dialect:?}: {source}");
            let (explanation, concept, experiment) = learning_copy(&hint, UiLanguage::Portuguese);
            assert!(explanation.contains(command), "{dialect:?}: {explanation}");
            assert!(
                explanation.contains(if newline {
                    "encerra a linha"
                } else {
                    "sem encerrar a linha"
                }),
                "{dialect:?}: {explanation}"
            );
            assert!(explanation.contains("Olá"), "{dialect:?}: {explanation}");
            assert!(concept.contains("quebra de linha"));
            assert!(experiment.contains("instruções de saída"));
            assert!(experiment.contains("Olá"), "{dialect:?}: {experiment}");
        }
    }

    #[test]
    fn learning_copy_preserves_nested_output_expression_and_describes_program_structure() {
        let source = "programa {\n funcao inicio() {\n  inteiro idade = 18\n  escreva(calcular(idade))\n }\n}";
        let output = learning_hint(source, 4, Language::Portugol);
        assert_eq!(output.expression.as_deref(), Some("calcular(idade)"));
        let (explanation, _, experiment) = learning_copy(&output, UiLanguage::Portuguese);
        assert!(explanation.contains("calcular(idade)"));
        assert!(experiment.contains("calcular(idade)"));

        let program = learning_hint(source, 1, Language::Portugol);
        let (explanation, concept, experiment) = learning_copy(&program, UiLanguage::Portuguese);
        assert!(explanation.contains("programa {"));
        assert!(concept.is_empty());
        assert!(experiment.is_empty());
    }

    #[test]
    fn learning_copy_uses_each_dialects_assignment_operator_and_target() {
        let cases = [
            (
                "var pontos := 0\npontos := 1",
                2,
                Language::Potigol,
                ":=",
                "pontos",
            ),
            (
                "programa {\n funcao inicio() {\n  inteiro pontos\n  pontos = 1\n }\n}",
                4,
                Language::Portugol,
                "=",
                "pontos",
            ),
            (
                "algoritmo \"aula\"\nvar\npontos: inteiro\ninicio\npontos <- 1\nfimalgoritmo",
                5,
                Language::Visualg,
                "<-",
                "pontos",
            ),
            (
                "algoritmo aula;\nvariáveis pontos: inteiro; fim-variáveis\ninício\npontos := 1;\nfim",
                4,
                Language::GPortugol,
                ":=",
                "pontos",
            ),
            (
                "algoritmo aula;\nprincipal\ninteiro pontos;\npontos = 1;\nfimPrincipal",
                4,
                Language::Calango,
                "=",
                "pontos",
            ),
            (
                "variaveis\npontos: inteiro;\ninicio\npontos <- 1;\nfim.",
                4,
                Language::Mapler,
                "<-",
                "pontos",
            ),
        ];

        for (source, line, dialect, operator, target) in cases {
            let hint = learning_hint(source, line, dialect);
            assert_eq!(
                hint.topic,
                LearningTopic::Assignment,
                "{dialect:?}: {source}"
            );
            assert_eq!(hint.name.as_deref(), Some(target), "{dialect:?}: {source}");
            let (explanation, concept, experiment) = learning_copy(&hint, UiLanguage::Portuguese);
            assert!(explanation.contains(&format!("`{operator}`")));
            assert!(explanation.contains(target));
            assert!(!concept.is_empty());
            assert!(experiment.contains(target));
            assert!(experiment.contains("1"), "{dialect:?}: {experiment}");
        }
    }

    #[test]
    fn visualg_copy_teaches_routines_and_each_loop_form() {
        let source = "algoritmo \"rotinas\"\nvar\ni: inteiro\nprocedimento mostrar(valor: inteiro)\ninicio\nescreval(valor)\nfimprocedimento\nfuncao dobrar(valor: inteiro): inteiro\ninicio\nretorne valor * 2\nfimfuncao\ninicio\npara i de 1 ate 2 faca\nescreval(i)\nfimpara\nrepita\ni <- i - 1\nate i <= 0\nfimalgoritmo";

        let program = learning_hint(source, 1, Language::Visualg);
        assert_eq!(program.topic, LearningTopic::Program);
        assert!(
            learning_copy(&program, UiLanguage::Portuguese)
                .0
                .contains("algoritmo")
        );

        let variable = learning_hint(source, 3, Language::Visualg);
        assert_eq!(variable.topic, LearningTopic::Variable);
        assert!(
            learning_copy(&variable, UiLanguage::Portuguese)
                .0
                .contains("`i`")
        );

        let procedure = learning_hint(source, 4, Language::Visualg);
        assert_eq!(procedure.topic, LearningTopic::Procedure);
        assert_eq!(procedure.name.as_deref(), Some("mostrar"));
        assert_eq!(procedure.parameters[0].name, "valor");
        let (procedure_text, procedure_concept, procedure_experiment) =
            learning_copy(&procedure, UiLanguage::Portuguese);
        assert!(procedure_text.contains("`mostrar`"));
        assert!(procedure_text.contains("procedimento"));
        assert!(procedure_concept.contains("Uma rotina"));
        assert!(procedure_experiment.contains("`valor`"));

        let function = learning_hint(source, 8, Language::Visualg);
        assert_eq!(function.topic, LearningTopic::Function);
        assert_eq!(function.name.as_deref(), Some("dobrar"));
        assert_eq!(function.return_type.as_deref(), Some("inteiro"));
        let (function_text, _, function_experiment) =
            learning_copy(&function, UiLanguage::Portuguese);
        assert!(function_text.contains("`dobrar`"));
        assert!(function_experiment.contains("`dobrar`"));

        let return_hint = learning_hint(source, 10, Language::Visualg);
        assert_eq!(return_hint.topic, LearningTopic::Return);
        assert!(
            learning_copy(&return_hint, UiLanguage::Portuguese)
                .0
                .contains("valor * 2")
        );

        let for_loop = learning_hint(source, 13, Language::Visualg);
        assert_eq!(for_loop.topic, LearningTopic::Loop);
        assert_eq!(for_loop.name.as_deref(), Some("para i"));
        let (for_text, _, for_experiment) = learning_copy(&for_loop, UiLanguage::Portuguese);
        assert!(for_text.contains("percorre os valores"));
        assert!(for_experiment.contains("limite do `para`"));

        let repeat_loop = learning_hint(source, 16, Language::Visualg);
        assert_eq!(repeat_loop.topic, LearningTopic::Loop);
        assert_eq!(repeat_loop.name.as_deref(), Some("repita"));
        let (repeat_text, _, repeat_experiment) =
            learning_copy(&repeat_loop, UiLanguage::Portuguese);
        assert!(repeat_text.contains("depois de executar o bloco"));
        assert!(repeat_experiment.contains("antes da verificação"));
    }

    #[test]
    fn calango_copy_uses_program_routine_and_statement_facts() {
        let source = "algoritmo aula;\nprocedimento mostrar(texto nome)\nescreval(nome);\nfimProcedimento\nfuncao inteiro dobrar(inteiro valor)\nretorna valor * 2;\nfimFuncao\nprincipal\ninteiro pontos;\npontos = 2;\nleia(pontos);\nescreval(pontos);\nse (pontos > 0) entao\npontos = pontos + 1;\nfimSe\npara (pontos de 1 ate 2 passo 1) faca\nescreval(pontos);\nfimPara\nfaca\npontos = pontos - 1;\nenquanto pontos > 0;\nfimPrincipal";

        let program = learning_hint(source, 1, Language::Calango);
        assert_eq!(program.topic, LearningTopic::Program);
        assert!(
            learning_copy(&program, UiLanguage::Portuguese)
                .0
                .contains("programa")
        );

        let procedure = learning_hint(source, 2, Language::Calango);
        assert_eq!(procedure.topic, LearningTopic::Procedure);
        let (procedure_text, _, procedure_experiment) =
            learning_copy(&procedure, UiLanguage::Portuguese);
        assert!(procedure_text.contains("`mostrar`"));
        assert!(procedure_text.contains("procedimento"));
        assert!(procedure_experiment.contains("`nome`"));

        let function = learning_hint(source, 5, Language::Calango);
        assert_eq!(function.topic, LearningTopic::Function);
        let (function_text, _, _) = learning_copy(&function, UiLanguage::Portuguese);
        assert!(function_text.contains("`dobrar`"));
        assert!(function_text.contains("`inteiro`"));

        let assignment = learning_hint(source, 10, Language::Calango);
        let (assignment_text, _, assignment_experiment) =
            learning_copy(&assignment, UiLanguage::Portuguese);
        assert_eq!(assignment.topic, LearningTopic::Assignment);
        assert!(assignment_text.contains("`=`"));
        assert!(assignment_text.contains("`pontos`"));
        assert!(assignment_experiment.contains("`pontos`"));

        let input = learning_hint(source, 11, Language::Calango);
        assert_eq!(input.topic, LearningTopic::Input);
        assert!(
            learning_copy(&input, UiLanguage::Portuguese)
                .0
                .contains("`leia`")
        );
        assert!(
            learning_copy(&input, UiLanguage::Portuguese)
                .0
                .contains("`pontos`")
        );

        let output = learning_hint(source, 12, Language::Calango);
        assert_eq!(output.topic, LearningTopic::Output);
        let (output_text, _, output_experiment) = learning_copy(&output, UiLanguage::Portuguese);
        assert!(output_text.contains("`escreval`"));
        assert!(output_text.contains("encerra a linha"));
        assert!(output_experiment.contains("`pontos`"));

        let for_loop = learning_hint(source, 16, Language::Calango);
        assert_eq!(for_loop.topic, LearningTopic::Loop);
        let (for_text, _, for_experiment) = learning_copy(&for_loop, UiLanguage::Portuguese);
        assert!(for_text.contains("`para`"));
        assert!(for_experiment.contains("limite do `para`"));

        let do_while = learning_hint(source, 19, Language::Calango);
        assert_eq!(do_while.topic, LearningTopic::Loop);
        assert_eq!(do_while.name.as_deref(), Some("faca"));
        let (do_while_text, _, do_while_experiment) =
            learning_copy(&do_while, UiLanguage::Portuguese);
        assert!(do_while_text.contains("`faca`"));
        assert!(do_while_text.contains("depois de executar"));
        assert!(do_while_experiment.contains("antes da verificação"));

        let unknown = learning_hint(
            "algoritmo aula;\nprincipal\n@@\nfimPrincipal",
            3,
            Language::Calango,
        );
        let (_, unknown_concept, unknown_experiment) =
            learning_copy(&unknown, UiLanguage::Portuguese);
        assert_eq!(unknown.topic, LearningTopic::Unrecognized);
        assert!(unknown_concept.is_empty());
        assert!(unknown_experiment.is_empty());
    }

    #[test]
    fn mapler_declarations_use_mapler_type_spelling_and_explain_the_value() {
        let source = "variaveis\nativo: logico;\ninicio\nfim.";
        let hint = learning_hint(source, 2, Language::Mapler);

        assert_eq!(hint.topic, LearningTopic::Variable);
        assert_eq!(hint.variables[0].type_name, "logico");
        let (explanation, _, _) = learning_copy(&hint, UiLanguage::Portuguese);
        assert!(explanation.contains("`logico`"), "{explanation}");
        assert!(
            explanation.contains("valor verdadeiro ou falso"),
            "{explanation}"
        );
        let (english, _, _) = learning_copy(&hint, UiLanguage::English);
        assert!(english.contains("`logico`"), "{english}");
        assert!(english.contains("true or false value"), "{english}");
        let (spanish, _, _) = learning_copy(&hint, UiLanguage::Spanish);
        assert!(spanish.contains("`logico`"), "{spanish}");
        assert!(spanish.contains("valor verdadero o falso"), "{spanish}");
    }

    #[test]
    fn mapler_module_calls_use_semicolon_syntax_and_module_guidance() {
        let source = "variaveis executar: modulo;\ninicio\nexecutar;\nfim.\nmodulo executar;\nescrever \"feito\";\nfim modulo;";
        let hint = learning_hint(source, 3, Language::Mapler);

        assert_eq!(hint.topic, LearningTopic::Call);
        assert_eq!(
            hint.call.as_ref().map(|call| call.member.as_str()),
            Some("executar")
        );
        assert!(hint.call.as_ref().unwrap().arguments.is_empty());
        let (explanation, concept, experiment) = learning_copy(&hint, UiLanguage::Portuguese);
        assert!(explanation.contains("`executar;`"), "{explanation}");
        assert!(!explanation.contains("executar()"), "{explanation}");
        assert!(concept.contains("módulo"), "{concept}");
        assert!(experiment.contains("módulo"), "{experiment}");
        let (english_explanation, english_concept, english_experiment) =
            learning_copy(&hint, UiLanguage::English);
        assert!(english_explanation.contains("calls the module"));
        assert!(english_concept.contains("In Mapler"));
        assert!(english_experiment.contains("instruction in the module"));
        let (spanish_explanation, spanish_concept, spanish_experiment) =
            learning_copy(&hint, UiLanguage::Spanish);
        assert!(spanish_explanation.contains("llama al módulo"));
        assert!(spanish_concept.contains("En Mapler"));
        assert!(spanish_experiment.contains("instrucción del módulo"));
    }

    #[test]
    fn learning_copy_explains_potigol_input_types_and_dialects() {
        for (command, description) in [
            ("leia_inteiro", "inteiro"),
            ("leia_real", "número real"),
            ("leia_texto", "texto"),
            ("leia_caractere", "caractere"),
            ("leia_logico", "valor lógico"),
            ("leia_inteiros", "lista de inteiros"),
            ("leia_reais", "lista de números reais"),
            ("leia_textos", "lista de textos"),
        ] {
            let source = format!("var valor := {command}()\n");
            let hint = learning_hint(&source, 1, Language::Potigol);
            assert_eq!(hint.topic, LearningTopic::Input, "{command}");
            let (explanation, concept, experiment) = learning_copy(&hint, UiLanguage::Portuguese);
            assert!(explanation.contains(command));
            assert!(explanation.contains(description));
            assert!(concept.contains("tipo solicitado"));
            assert!(experiment.contains("Digite outro valor"));
            assert!(explanation.contains("valor"), "{command}: {explanation}");
            assert!(experiment.contains("valor"), "{command}: {experiment}");
        }

        let cases = [
            (
                "programa {\n funcao inicio() {\n  inteiro valor\n  leia(valor)\n }\n}",
                4,
                Language::Portugol,
                "leia",
            ),
            (
                "algoritmo \"aula\"\nvar\nvalor: inteiro\ninicio\nleia(valor)\nfimalgoritmo",
                5,
                Language::Visualg,
                "leia",
            ),
            (
                "algoritmo aula;\nvariáveis valor: inteiro; fim-variáveis\ninício\nvalor := leia();\nfim",
                4,
                Language::GPortugol,
                "leia",
            ),
            (
                "algoritmo aula;\nprincipal\ninteiro valor;\nleia(valor);\nfimPrincipal",
                4,
                Language::Calango,
                "leia",
            ),
            (
                "variaveis valor: inteiro;\ninicio\nler valor;\nfim.",
                3,
                Language::Mapler,
                "ler",
            ),
        ];
        for (source, line, dialect, command) in cases {
            let hint = learning_hint(source, line, dialect);
            assert_eq!(hint.topic, LearningTopic::Input, "{dialect:?}: {source}");
            let (explanation, _, _) = learning_copy(&hint, UiLanguage::Portuguese);
            assert!(explanation.contains(command), "{dialect:?}: {explanation}");
            assert!(explanation.contains("valor"), "{dialect:?}: {explanation}");
        }
    }

    #[test]
    fn learning_copy_uses_each_dialects_condition_and_loop_kind() {
        let cases = [
            (
                "var idade := 18\nse idade >= 18 então\n  escreva \"maior\"\nsenão\n  escreva \"menor\"\nfim\nenquanto idade > 0 faça\n  idade := idade - 1\nfim",
                2,
                7,
                Language::Potigol,
            ),
            (
                "programa {\n funcao inicio() {\n  inteiro idade = 18\n  se (idade >= 18) {\n   escreva(\"maior\")\n  } senao {\n   escreva(\"menor\")\n  }\n  enquanto (idade > 0) {\n   idade--\n  }\n }\n}",
                4,
                9,
                Language::Portugol,
            ),
            (
                "algoritmo \"aula\"\nvar\nidade: inteiro\ninicio\nse idade >= 18 entao\n escreval(\"maior\")\nsenao\n escreval(\"menor\")\nfimse\nenquanto idade > 0 faca\n idade <- idade - 1\nfimenquanto\nfimalgoritmo",
                5,
                10,
                Language::Visualg,
            ),
            (
                "algoritmo aula;\nvariáveis idade: inteiro; fim-variáveis\ninício\nse idade >= 18 então imprima(\"maior\"); senão imprima(\"menor\"); fim-se\nenquanto idade > 0 faça idade := idade - 1; fim-enquanto\nfim",
                4,
                5,
                Language::GPortugol,
            ),
            (
                "algoritmo aula;\nprincipal\ninteiro idade;\nse (idade >= 18) entao\nescreva(\"maior\");\nsenao\nescreva(\"menor\");\nfimSe\nenquanto (idade > 0) faca\nidade = idade - 1;\nfimEnquanto\nfimPrincipal",
                4,
                9,
                Language::Calango,
            ),
            (
                "variaveis idade: inteiro;\ninicio\nse idade >= 18 entao\nescrever \"maior\";\nsenao\nescrever \"menor\";\nfim se;\nenquanto idade > 0 faca\n idade <- idade - 1;\nfim enquanto;\nfim.",
                3,
                8,
                Language::Mapler,
            ),
        ];

        for (source, decision_line, loop_line, dialect) in cases {
            let decision = learning_hint(source, decision_line, dialect);
            assert_eq!(
                decision.topic,
                LearningTopic::Decision,
                "{dialect:?}: {:?}",
                decision.problems
            );
            assert!(
                decision
                    .expression
                    .as_deref()
                    .is_some_and(|condition| condition.contains(">=")),
                "{dialect:?}: {:?}",
                decision.expression
            );
            let (decision_text, _, decision_experiment) =
                learning_copy(&decision, UiLanguage::Portuguese);
            assert!(
                decision_text.contains(">= 18"),
                "{dialect:?}: {decision_text}"
            );
            assert!(!decision_experiment.is_empty(), "{dialect:?}");

            let loop_hint = learning_hint(source, loop_line, dialect);
            assert_eq!(loop_hint.topic, LearningTopic::Loop, "{dialect:?}");
            assert_eq!(loop_hint.name.as_deref(), Some("enquanto"), "{dialect:?}");
            let (loop_text, _, loop_experiment) = learning_copy(&loop_hint, UiLanguage::Portuguese);
            assert!(
                loop_text.contains("testa a condição"),
                "{dialect:?}: {loop_text}"
            );
            assert!(loop_text.contains("idade > 0"), "{dialect:?}: {loop_text}");
            assert!(
                loop_experiment.contains("idade > 0"),
                "{dialect:?}: {loop_experiment}"
            );
            assert!(!loop_experiment.is_empty(), "{dialect:?}");
        }
    }

    #[test]
    fn potigol_classic_and_jerimum_copy_remain_import_aware() {
        let assignment = learning_copy(
            &learning_hint("pontos := pontos + 1", 1, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(assignment.0.contains("`:=`"));
        assert!(assignment.0.contains("`pontos`"));

        let typed_input =
            learning_hint("var idade: Inteiro := leia_inteiro()", 1, Language::Potigol);
        assert_eq!(typed_input.topic, LearningTopic::Input);
        let input_copy = learning_copy(&typed_input, UiLanguage::Portuguese);
        assert!(input_copy.0.contains("`leia_inteiro`"));
        assert!(input_copy.0.contains("`idade`"));

        let unimported = learning_hint("g.setColor(Cor.VERDE.color)", 1, Language::Potigol);
        assert_eq!(unimported.topic, LearningTopic::Call);
        assert_eq!(
            unimported
                .call
                .as_ref()
                .and_then(|call| call.library.as_deref()),
            None
        );
        let unimported_copy = learning_copy(&unimported, UiLanguage::Portuguese);
        assert!(!unimported_copy.0.contains("biblioteca Jerimum"));
        assert!(!unimported_copy.1.to_lowercase().contains("jerimum"));
        assert!(unimported_copy.2.contains("`g.setColor(Cor.VERDE.color)`"));

        let imported =
            "use \"jerimum\"\ndef desenhe(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim";
        let import_copy = learning_copy(
            &learning_hint(imported, 1, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(import_copy.0.contains("jerimum"));

        let jerimum = learning_copy(
            &learning_hint(imported, 3, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(jerimum.0.contains("setColor define"));
        assert!(jerimum.1.contains("contexto gráfico"));
        assert!(jerimum.2.contains("Troque o argumento de cor"));
    }

    #[test]
    fn learning_copy_avoids_claims_for_unknown_lines_and_uses_neutral_procedure_concept() {
        let potigol = learning_copy(
            &learning_hint("soma = 1 + 2", 1, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(!potigol.0.contains("heurística"));

        let unknown = learning_copy(
            &learning_hint("??", 1, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(unknown.0.contains("não reconheceu"));
        assert!(unknown.1.is_empty());
        assert!(unknown.2.is_empty());

        let comment = learning_copy(
            &learning_hint("# comentário\n", 1, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(comment.0.contains("não contém uma instrução reconhecida"));
        assert!(comment.1.is_empty());
        assert!(comment.2.is_empty());

        let import = learning_copy(
            &learning_hint("use \"jerimum\"", 1, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(import.0.contains("jerimum"));
        assert!(!import.1.is_empty());
        assert!(import.2.is_empty());

        let call_hint = learning_hint("somar(2, 3)", 1, Language::Potigol);
        assert_eq!(call_hint.topic, LearningTopic::Call);
        let call = learning_copy(&call_hint, UiLanguage::Portuguese);
        assert!(call.0.contains("somar(2, 3)"));
        assert!(call.2.contains("somar(2, 3)"));

        let jerimum = learning_copy(
            &learning_hint(
                "use \"jerimum\"\ndesenhe_personagem(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim",
                3,
                Language::Potigol,
            ),
            UiLanguage::English,
        );
        assert!(jerimum.0.contains("sets the color"));
        assert!(jerimum.1.contains("graphics context"));
        assert!(jerimum.2.contains("another Cor constant"));

        let fill_rect = learning_copy(
            &learning_hint(
                "use \"jerimum\"\ndesenhe(g: Graphics2D): Nada\n  g.fillRect(1, 2, 32, 32)\nfim",
                3,
                Language::Potigol,
            ),
            UiLanguage::English,
        );
        assert!(fill_rect.0.contains("filled rectangle"));
        assert!(fill_rect.2.contains("last two arguments"));

        let mut unknown_library_hint = learning_hint("g.mystery(1)", 1, Language::Potigol);
        unknown_library_hint.topic = LearningTopic::LibraryCall;
        unknown_library_hint
            .call
            .as_mut()
            .expect("the call should be recovered")
            .library = Some("jerimum".to_owned());
        let unknown_library_call = learning_copy(&unknown_library_hint, UiLanguage::Portuguese);
        assert!(
            unknown_library_call
                .0
                .contains("a IDE ainda não tem uma descrição verificada")
        );
        assert!(unknown_library_call.1.is_empty());
        assert!(unknown_library_call.2.is_empty());

        let game_start = learning_copy(
            &learning_hint(
                "use \"jerimum\"\njogo.iniciar(\"Jogo\", 320, 200, atualize, desenhe, 60)",
                2,
                Language::Potigol,
            ),
            UiLanguage::English,
        );
        assert!(game_start.0.contains("opens the game window"));
        assert!(game_start.1.contains("game loop"));
        assert!(game_start.2.contains("width and height"));

        let procedure_hint = super::LearningHint {
            line: 1,
            column: 1,
            language: Language::Portugol,
            topic: LearningTopic::Procedure,
            name: Some("registrar".to_owned()),
            return_type: None,
            target: None,
            parameters: vec![LearningSymbol {
                name: "valor".to_owned(),
                type_name: "inteiro".to_owned(),
                is_constant: false,
                by_reference: false,
                dimensions: Vec::new(),
                initializer: None,
            }],
            variables: Vec::new(),
            expression: None,
            experiment_expression: None,
            call: None,
            problems: Vec::new(),
        };
        let procedure = learning_copy(&procedure_hint, UiLanguage::Portuguese);
        assert!(procedure.1.contains("rotina"));
        assert!(!procedure.1.contains("VisuAlg"));
        assert!(procedure.2.contains("valor"));
    }

    #[test]
    fn learning_panel_receives_only_editor_errors_on_the_selected_line() {
        let source = "use \"jerimum\"\njogo.titulo = \"teste\"\n";
        let diagnostics = crate::features::diagnostics::analyze(
            1,
            "principal.poti",
            None,
            Language::Potigol,
            source,
        );

        let mut selected = learning_hint(source, 2, Language::Potigol);
        merge_editor_diagnostics(&mut selected, &diagnostics);
        assert!(selected.problems.iter().any(|problem| {
            problem.line == 2
                && problem.kind == crate::features::learning::LearningProblemKind::Syntax
                && problem.message.contains("usa ':='")
        }));

        let mut other_line = learning_hint(source, 1, Language::Potigol);
        merge_editor_diagnostics(&mut other_line, &diagnostics);
        assert!(other_line.problems.is_empty());
    }

    #[test]
    fn learning_panel_keeps_one_gportugol_open_comment_warning_after_editor_merge() {
        let source = "algoritmo aula;\ninício\n  /* comentário aberto";
        let diagnostics = crate::features::diagnostics::analyze(
            2,
            "principal.gpt",
            None,
            Language::GPortugol,
            source,
        );
        let mut hint = learning_hint(source, 3, Language::GPortugol);
        merge_editor_diagnostics(&mut hint, &diagnostics);

        let warnings = hint
            .problems
            .iter()
            .filter(|problem| {
                problem.kind == crate::features::learning::LearningProblemKind::StaticAnalysis
                    && problem.message == "Comentário de bloco sem fechamento"
            })
            .collect::<Vec<_>>();
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].suggestion.as_deref(),
            Some("Adicione `*/` para fechar o comentário de bloco.")
        );
    }

    #[test]
    fn atomic_save_replaces_file_contents_without_leaving_temporary_files() {
        let directory = std::env::temp_dir().join(format!(
            "potoo-atomic-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("programa.por");
        std::fs::write(&path, "versão antiga").unwrap();

        super::atomic_write(&path, "versão nova".as_bytes()).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "versão nova");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn project_scan_results_are_rejected_when_the_workspace_generation_is_stale() {
        let old_root = Path::new("/tmp/old-project");
        let new_root = Path::new("/tmp/new-project");
        assert!(project_scan_is_current(
            4,
            9,
            Some(old_root),
            4,
            9,
            new_root,
            true
        ));
        assert!(!project_scan_is_current(
            5,
            9,
            Some(old_root),
            4,
            9,
            new_root,
            true
        ));
        assert!(!project_scan_is_current(
            4,
            10,
            Some(old_root),
            4,
            9,
            old_root,
            false
        ));
        assert!(!project_scan_is_current(
            4,
            9,
            Some(old_root),
            4,
            9,
            new_root,
            false
        ));
    }
}

#[cfg(test)]
mod input_cancel_tests {
    use super::input_cancel_controller;
    use gtk::prelude::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    #[ignore = "requires a GTK display; run with xvfb-run"]
    fn escape_clears_pending_input_and_requests_cancellation() {
        gtk::init().expect("GTK should initialize under Xvfb");
        let entry = gtk::Entry::new();
        entry.set_text("entrada não enviada");
        let cancel_count = Rc::new(Cell::new(0));
        let cancel_count_for_callback = cancel_count.clone();
        let controller = input_cancel_controller(&entry, move || {
            cancel_count_for_callback.set(cancel_count_for_callback.get() + 1);
        });
        entry.add_controller(controller.clone());

        let normal_key = controller.emit_by_name::<bool>(
            "key-pressed",
            &[&gtk::gdk::Key::a, &0_u32, &gtk::gdk::ModifierType::empty()],
        );
        assert!(!normal_key);
        assert_eq!(entry.text(), "entrada não enviada");
        assert_eq!(cancel_count.get(), 0);

        let escape_key = controller.emit_by_name::<bool>(
            "key-pressed",
            &[
                &gtk::gdk::Key::Escape,
                &0_u32,
                &gtk::gdk::ModifierType::empty(),
            ],
        );
        assert!(escape_key);
        assert_eq!(entry.text(), "");
        assert_eq!(cancel_count.get(), 1);
    }
}

#[cfg(test)]
mod file_picker_execution_tests {
    use super::execution_reply_is_current;

    #[test]
    fn old_file_picker_reply_is_rejected_after_a_new_execution_starts() {
        assert!(!execution_reply_is_current(41, Some(42)));
        assert!(execution_reply_is_current(42, Some(42)));
    }
}

#[cfg(test)]
#[path = "../../../tests/learning/app_state_tests.rs"]
mod learning_state_tests;
