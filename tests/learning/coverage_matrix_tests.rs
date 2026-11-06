use super::{
    LearningProblemKind, LearningProblemText, LearningSeverity, LearningTopic, learning_hint,
    learning_hint_with_runtime,
};
use crate::features::execution::{ExecutionDiagnostic, ExecutionEvent};
use crate::languages::model::{Language, SourceDiagnosticKind};
use std::time::Duration;

struct LexicalFixture {
    profile: &'static str,
    language: Language,
    source: &'static str,
    error_line: usize,
    valid_line: usize,
}

#[test]
fn lexical_diagnostics_stay_at_their_original_location_without_guessed_advice() {
    let fixtures = [
        LexicalFixture {
            profile: "Portugol Studio 2.0",
            language: Language::Portugol,
            source: "programa {\n @\n}",
            error_line: 2,
            valid_line: 1,
        },
        LexicalFixture {
            profile: "G-Portugol",
            language: Language::GPortugol,
            source: "algoritmo aula;\ninício\n@\nfim",
            error_line: 3,
            valid_line: 1,
        },
        LexicalFixture {
            profile: "Mapler",
            language: Language::Mapler,
            source: "variaveis\n x: inteiro;\ninicio\n@\nfim.",
            error_line: 4,
            valid_line: 1,
        },
        LexicalFixture {
            profile: "VisuAlg",
            language: Language::Visualg,
            source: "algoritmo \"aula\"\ninicio\n@\nfimalgoritmo",
            error_line: 3,
            valid_line: 1,
        },
        LexicalFixture {
            profile: "Calango",
            language: Language::Calango,
            source: "algoritmo aula;\nprincipal\n@\nfimPrincipal",
            error_line: 3,
            valid_line: 1,
        },
        LexicalFixture {
            profile: "Potigol sem Jerimum",
            language: Language::Potigol,
            source: "escreva \"ok\"\n@\n",
            error_line: 2,
            valid_line: 1,
        },
        LexicalFixture {
            profile: "Potigol com Jerimum",
            language: Language::Potigol,
            source: "use \"jerimum\"\nescreva \"ok\"\n@\n",
            error_line: 3,
            valid_line: 1,
        },
    ];

    for fixture in fixtures {
        let original = (fixture.language.profile().diagnostics)(fixture.source)
            .into_iter()
            .find(|diagnostic| {
                diagnostic.kind == SourceDiagnosticKind::Lexical
                    && diagnostic.line == fixture.error_line
            })
            .unwrap_or_else(|| {
                panic!(
                    "{} must emit its original lexical diagnostic",
                    fixture.profile
                )
            });
        let hint = learning_hint(fixture.source, fixture.error_line, fixture.language);
        let problem = hint
            .problems
            .iter()
            .find(|problem| {
                problem.text == LearningProblemText::InterpreterDiagnostic
                    && problem.line == original.line
                    && problem.column == original.column
            })
            .unwrap_or_else(|| panic!("{} must preserve the diagnostic", fixture.profile));

        assert_eq!(
            problem.kind,
            LearningProblemKind::Syntax,
            "{}",
            fixture.profile
        );
        assert_eq!(problem.message, original.message, "{}", fixture.profile);
        assert_eq!(problem.suggestion, None, "{}", fixture.profile);

        let valid_hint = learning_hint(fixture.source, fixture.valid_line, fixture.language);
        assert!(
            valid_hint.problems.is_empty(),
            "{} must not copy an error from another line: {:?}",
            fixture.profile,
            valid_hint.problems
        );
    }
}

#[test]
fn parser_syntax_diagnostics_keep_the_message_and_location_in_non_potigol_profiles() {
    let fixtures = [
        (
            "Portugol Studio 2.0",
            Language::Portugol,
            "programa {\n funcao inicio() {\n  inteiro = 1\n  escreva(\"depois\")\n }\n}",
            3,
        ),
        (
            "G-Portugol",
            Language::GPortugol,
            "algoritmo erro;\ninício\n  imprima(1 + );\n  imprima(2);\nfim",
            3,
        ),
        (
            "Mapler",
            Language::Mapler,
            "variaveis\ninicio\nescrever 1 + ;\nescrever \"depois\";\nfim.",
            3,
        ),
        (
            "VisuAlg",
            Language::Visualg,
            "algoritmo \"erro\"\ninicio\nescreval(1 + )\nescreval(2)\nfimalgoritmo",
            3,
        ),
        (
            "Calango",
            Language::Calango,
            "algoritmo erro;\nprincipal\nescreval(1 + );\nescreval(2);\nfimPrincipal",
            3,
        ),
    ];

    for (profile, language, source, line) in fixtures {
        let diagnostics = (language.profile().diagnostics)(source);
        let original = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.kind == SourceDiagnosticKind::Syntax && diagnostic.line == line
            })
            .unwrap_or_else(|| {
                panic!(
                    "{profile} must provide the syntax diagnostic on line {line}: {diagnostics:?}"
                )
            });
        let hint = learning_hint(source, line, language);
        let problem = hint
            .problems
            .iter()
            .find(|problem| {
                problem.kind == LearningProblemKind::Syntax
                    && problem.line == original.line
                    && problem.column == original.column
            })
            .unwrap_or_else(|| panic!("{profile} did not preserve the selected syntax diagnostic"));
        let expected_message = crate::features::diagnostics::adapt_diagnostic_message(
            language,
            source,
            crate::features::diagnostics::DiagnosticCategory::Syntax,
            &original.message,
            original.line,
            original.column,
        );

        assert_eq!(problem.message, expected_message, "{profile}");
        assert_eq!(problem.suggestion, None, "{profile}");
    }
}

struct ConditionFixture {
    profile: &'static str,
    language: Language,
    source: &'static str,
    line: usize,
}

#[test]
fn constant_condition_advice_has_near_misses_in_each_language_profile() {
    let fixtures = [
        ConditionFixture {
            profile: "Portugol Studio 2.0",
            language: Language::Portugol,
            source: "programa {\n funcao inicio() {\n  se (verdadeiro) {\n   escreva(1)\n  }\n }\n}",
            line: 3,
        },
        ConditionFixture {
            profile: "G-Portugol",
            language: Language::GPortugol,
            source: "algoritmo aula;\ninício\nse verdadeiro então\n imprima(1);\nfim-se\nfim",
            line: 3,
        },
        ConditionFixture {
            profile: "Mapler",
            language: Language::Mapler,
            source: "variaveis\ninicio\nse verdadeiro entao\n escrever 1;\nfim se;\nfim.",
            line: 3,
        },
        ConditionFixture {
            profile: "VisuAlg",
            language: Language::Visualg,
            source: "algoritmo \"aula\"\ninicio\nse verdadeiro entao\nescreval(1)\nfimse\nfimalgoritmo",
            line: 3,
        },
        ConditionFixture {
            profile: "Calango",
            language: Language::Calango,
            source: "algoritmo aula;\nprincipal\nse verdadeiro entao\nescreval(1);\nfimSe\nfimPrincipal",
            line: 3,
        },
        ConditionFixture {
            profile: "Potigol",
            language: Language::Potigol,
            source: "se verdadeiro entao\n  escreva 1\nfim",
            line: 1,
        },
    ];

    for fixture in fixtures {
        let positive = learning_hint(fixture.source, fixture.line, fixture.language);
        assert_eq!(
            positive.topic,
            LearningTopic::Decision,
            "{}",
            fixture.profile
        );
        assert!(
            positive.problems.iter().any(|problem| {
                problem.kind == LearningProblemKind::PossibleLogic
                    && problem.severity == LearningSeverity::Warning
                    && problem.text == LearningProblemText::ConstantCondition { always_true: true }
            }),
            "{} must report a literal true condition: {:?}",
            fixture.profile,
            positive.problems
        );

        let near_miss = fixture.source.replace("verdadeiro", "1 > 0");
        let negative = learning_hint(&near_miss, fixture.line, fixture.language);
        assert!(
            negative.problems.iter().all(|problem| !matches!(
                problem.text,
                LearningProblemText::ConstantCondition { .. }
            )),
            "{} must not label a comparison as a literal condition: {:?}",
            fixture.profile,
            negative.problems
        );
    }
}

struct BoundedArrayFixture {
    profile: &'static str,
    language: Language,
    source: &'static str,
    invalid_line: usize,
    invalid_column: usize,
    invalid_suggestion: &'static str,
    valid_line: usize,
    declaration_line: usize,
}

#[test]
fn currently_supported_array_bounds_have_valid_and_invalid_index_cases() {
    let fixtures = [
        BoundedArrayFixture {
            profile: "Portugol Studio 2.0",
            language: Language::Portugol,
            source: "programa {\n funcao inicio() {\n  inteiro valores[3]\n  escreva(valores[3])\n  escreva(valores[2])\n }\n}",
            invalid_line: 4,
            invalid_column: 11,
            invalid_suggestion: "Use um índice entre 0 e 2 em `valores`.",
            valid_line: 5,
            declaration_line: 3,
        },
        BoundedArrayFixture {
            profile: "VisuAlg",
            language: Language::Visualg,
            source: "algoritmo \"limites\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\nescreval(dados[1])\nescreval(dados[5])\nfimalgoritmo",
            invalid_line: 5,
            invalid_column: 10,
            invalid_suggestion: "Use um índice entre 2 e 5 em `dados`.",
            valid_line: 6,
            declaration_line: 3,
        },
        BoundedArrayFixture {
            profile: "Mapler",
            language: Language::Mapler,
            source: "variaveis\n dados: vetor[2..5] de inteiro;\n indice: inteiro;\ninicio\n escrever dados[1];\n escrever dados[5];\n escrever dados[indice];\n escrever \"dados[1]\";\nfim.",
            invalid_line: 5,
            invalid_column: 11,
            invalid_suggestion: "Use um índice entre 2 e 5 em `dados`.",
            valid_line: 6,
            declaration_line: 2,
        },
        BoundedArrayFixture {
            profile: "Calango",
            language: Language::Calango,
            source: "algoritmo limites;\nprincipal\ninteiro valores[3];\nescreva(valores[3]);\nescreva(valores[2]);\nfimPrincipal",
            invalid_line: 4,
            invalid_column: 9,
            invalid_suggestion: "Use um índice entre 0 e 2 em `valores`.",
            valid_line: 5,
            declaration_line: 3,
        },
    ];

    for fixture in fixtures {
        let invalid = learning_hint(fixture.source, fixture.invalid_line, fixture.language);
        let problem = invalid
            .problems
            .iter()
            .find(|problem| {
                problem.kind == LearningProblemKind::Logic
                    && problem.severity == LearningSeverity::Error
                    && problem.suggestion.as_deref() == Some(fixture.invalid_suggestion)
            })
            .unwrap_or_else(|| {
                panic!(
                    "{} must explain its out-of-range literal: {:?}",
                    fixture.profile, invalid.problems
                )
            });
        assert_eq!(
            (problem.line, problem.column),
            (fixture.invalid_line, fixture.invalid_column),
            "{}",
            fixture.profile
        );

        for line in [fixture.valid_line, fixture.declaration_line] {
            let hint = learning_hint(fixture.source, line, fixture.language);
            assert!(
                hint.problems
                    .iter()
                    .all(|problem| problem.kind != LearningProblemKind::Logic),
                "{} must not flag valid indexes or declarations: {:?}",
                fixture.profile,
                hint.problems
            );
        }
    }
}

#[test]
fn portugol_array_bounds_do_not_guess_for_variable_indexes_or_string_contents() {
    let source = "programa {\n funcao inicio() {\n  inteiro valores[3]\n  inteiro indice = 3\n  escreva(valores[indice])\n  escreva(\"valores[3]\")\n }\n}";

    for line in [5, 6] {
        let hint = learning_hint(source, line, Language::Portugol);
        assert!(
            hint.problems.iter().all(|problem| {
                !matches!(problem.text, LearningProblemText::ConstantIndex { .. })
            }),
            "variable indexes and text contents must not trigger constant-bound advice: {:?}",
            hint.problems
        );
    }
}

#[test]
fn portugol_vector_bounds_reject_a_negative_constant_index() {
    let source =
        "programa {\n funcao inicio() {\n  inteiro valores[3]\n  escreva(valores[-1])\n }\n}";
    let hint = learning_hint(source, 4, Language::Portugol);

    assert!(hint.problems.iter().any(|problem| {
        problem.text
            == LearningProblemText::ConstantIndex {
                index: -1,
                name: "valores".to_owned(),
                lower_bound: 0,
                upper_bound: 2,
            }
            && problem.suggestion.as_deref() == Some("Use um índice entre 0 e 2 em `valores`.")
    }));
}

#[test]
fn gportugol_open_block_comment_has_a_specific_warning_and_near_misses() {
    let source = "algoritmo aula;\ninício\n  /* comentário aberto";
    let diagnostic = (Language::GPortugol.profile().diagnostics)(source)
        .into_iter()
        .find(|diagnostic| {
            diagnostic.kind == SourceDiagnosticKind::StaticAnalysis
                && diagnostic.message == "Comentário de bloco sem fechamento"
        })
        .expect("the G-Portugol profile should identify its open block comment");
    let hint = learning_hint(source, diagnostic.line, Language::GPortugol);
    let problem = hint
        .problems
        .iter()
        .find(|problem| {
            problem.text == LearningProblemText::InterpreterDiagnostic
                && problem.line == diagnostic.line
                && problem.column == diagnostic.column
                && problem.message == diagnostic.message
        })
        .expect("the shared learning analysis should surface the profile warning");

    assert_eq!(problem.severity, LearningSeverity::Warning);
    assert_eq!(problem.kind, LearningProblemKind::StaticAnalysis);
    assert_eq!(
        problem.suggestion.as_deref(),
        Some("Adicione `*/` para fechar o comentário de bloco.")
    );

    for (near_miss, line) in [
        (
            "algoritmo aula;\ninício\n  /* comentário fechado */\n  imprima(1);\nfim",
            3,
        ),
        (
            "algoritmo aula;\ninício\n  // /* marcador comentado\n  imprima(1);\nfim",
            3,
        ),
        (
            "algoritmo aula;\ninício\n  imprima(\"/* marcador em texto\");\nfim",
            3,
        ),
    ] {
        let hint = learning_hint(near_miss, line, Language::GPortugol);
        assert!(
            hint.problems
                .iter()
                .all(|problem| problem.message != "Comentário de bloco sem fechamento"),
            "a closed comment, line comment, or string literal must not receive the warning: {:?}",
            hint.problems
        );
    }
}

#[test]
fn gportugol_declared_arrays_use_the_shared_constant_index_rule() {
    let source = "algoritmo vetores;\nvariáveis valores: matriz[3] de inteiros; indice: inteiro; fim-variáveis\ninício\nimprima(valores[3]);\nimprima(valores[2]);\nimprima(valores[indice]);\nimprima(\"valores[3]\");\nfim";
    let diagnostics = (Language::GPortugol.profile().diagnostics)(source);
    assert!(
        diagnostics.is_empty(),
        "the G-Portugol array fixture must be valid before checking bounds: {diagnostics:?}"
    );

    let invalid = learning_hint(source, 4, Language::GPortugol);
    assert!(
        invalid.problems.iter().any(|problem| {
            problem.text
                == LearningProblemText::ConstantIndex {
                    index: 3,
                    name: "valores".to_owned(),
                    lower_bound: 0,
                    upper_bound: 2,
                }
                && problem.suggestion.as_deref() == Some("Use um índice entre 0 e 2 em `valores`.")
        }),
        "G-Portugol should explain a proven out-of-range constant index: {:?}",
        invalid.problems
    );

    for line in [5, 6, 7] {
        let hint = learning_hint(source, line, Language::GPortugol);
        assert!(
            hint.problems.iter().all(|problem| {
                !matches!(problem.text, LearningProblemText::ConstantIndex { .. })
            }),
            "valid, variable, and string indexes must not receive constant-bound advice: {:?}",
            hint.problems
        );
    }

    let ambiguous = "algoritmo vetores;\nvariáveis valores: matriz[3] de inteiros; valores: matriz[4] de inteiros; fim-variáveis\ninício\nimprima(valores[3]);\nfim";
    let hint = learning_hint(ambiguous, 4, Language::GPortugol);
    assert!(
        hint.problems
            .iter()
            .all(|problem| { !matches!(problem.text, LearningProblemText::ConstantIndex { .. }) }),
        "duplicate declarations must not be assigned one guessed bound: {:?}",
        hint.problems
    );
}

#[test]
fn calango_array_bounds_do_not_use_malformed_declarations() {
    let source = "algoritmo limites;\nprincipal\ninteiro valores[3] inválido;\nescreva(valores[3]);\nfimPrincipal";
    let syntax_diagnostics = (Language::Calango.profile().diagnostics)(source);
    assert!(
        syntax_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == SourceDiagnosticKind::Syntax),
        "the malformed declaration must produce a parser diagnostic: {syntax_diagnostics:?}"
    );

    let hint = learning_hint(source, 4, Language::Calango);
    assert!(
        hint.problems
            .iter()
            .all(|problem| { !matches!(problem.text, LearningProblemText::ConstantIndex { .. }) }),
        "a bound from a malformed declaration must not be guessed: {:?}",
        hint.problems
    );
}

#[test]
fn calango_array_bounds_use_unicode_columns_and_ignore_near_misses() {
    let source = "algoritmo limites;\nprincipal\ninteiro valores[3];\ninteiro indice;\ninteiro ação;\n  escreval(ação + valores[3]);\n  escreval(valores[2]);\n  escreval(valores[0]); // valores[3]\n  escreval(valores[indice]);\n  escreval(\"valores[3]\");\n  escreval(valores[0]); /* valores[3] */\nfimPrincipal";
    let diagnostics = (Language::Calango.profile().diagnostics)(source);
    assert!(
        diagnostics.is_empty(),
        "the Calango array fixture must be valid: {diagnostics:?}"
    );

    let hint = learning_hint(source, 6, Language::Calango);
    let problem = hint
        .problems
        .iter()
        .find(|problem| {
            problem.text
                == LearningProblemText::ConstantIndex {
                    index: 3,
                    name: "valores".to_owned(),
                    lower_bound: 0,
                    upper_bound: 2,
                }
        })
        .expect("Calango should flag the proven upper-bound violation");
    assert_eq!(problem.kind, LearningProblemKind::Logic);
    assert_eq!(problem.severity, LearningSeverity::Error);
    assert_eq!((problem.line, problem.column), (6, 19));
    assert_eq!(
        problem.suggestion.as_deref(),
        Some("Use um índice entre 0 e 2 em `valores`.")
    );

    for line in [7, 8, 9, 10, 11] {
        let hint = learning_hint(source, line, Language::Calango);
        assert!(
            hint.problems.iter().all(|problem| {
                !matches!(problem.text, LearningProblemText::ConstantIndex { .. })
            }),
            "valid, variable, string, or commented indexes must stay silent: {:?}",
            hint.problems
        );
    }

    let ambiguous = "algoritmo ambiguo;\nprincipal\ninteiro valores[3];\ninteiro valores[5];\nescreva(valores[6]);\nfimPrincipal";
    let hint = learning_hint(ambiguous, 5, Language::Calango);
    assert!(
        hint.problems
            .iter()
            .all(|problem| { !matches!(problem.text, LearningProblemText::ConstantIndex { .. }) }),
        "duplicate declarations must not select a guessed bound: {:?}",
        hint.problems
    );
}

#[test]
fn calango_array_bounds_include_nested_declarations_and_static_parameters() {
    let source = "algoritmo escopos;\nprocedimento verificar(inteiro parametros[3])\nescreva(parametros[3]);\nfimProcedimento\nprincipal\nse (verdadeiro) entao\ninteiro locais[2];\nescreva(locais[2]);\nfimSe\nfimPrincipal";
    let diagnostics = (Language::Calango.profile().diagnostics)(source);
    assert!(
        diagnostics.is_empty(),
        "the Calango scope fixture must be valid: {diagnostics:?}"
    );

    for (line, name, index, upper_bound) in [(3, "parametros", 3, 2), (8, "locais", 2, 1)] {
        let hint = learning_hint(source, line, Language::Calango);
        assert!(
            hint.problems.iter().any(|problem| {
                problem.text
                    == LearningProblemText::ConstantIndex {
                        index,
                        name: name.to_owned(),
                        lower_bound: 0,
                        upper_bound,
                    }
                    && problem.suggestion
                        == Some(format!(
                            "Use um índice entre 0 e {upper_bound} em `{name}`."
                        ))
            }),
            "Calango should use the proven bounds for `{name}`: {:?}",
            hint.problems
        );
    }
}

#[test]
fn visualg_array_bounds_check_both_edges_and_ignore_unproven_indexes() {
    let source = "algoritmo \"limites\"\nvar\n  dados: vetor[2..5] de inteiro\n  indice: inteiro\ninicio\n  escreval(dados[2])\n  escreval(dados[5])\n  escreval(dados[1]) // comentário com dados[6]\n  escreval(dados[6])\n  escreval(dados[-1])\n  escreval(dados[indice])\n  escreval(\"dados[6]\")\nfimalgoritmo";

    let diagnostics = (Language::Visualg.profile().diagnostics)(source);
    assert!(
        diagnostics.is_empty(),
        "the VisuAlg vector-bound fixture must be valid: {diagnostics:?}"
    );

    for (line, index) in [(8, 1), (9, 6), (10, -1)] {
        let hint = learning_hint(source, line, Language::Visualg);
        let problem = hint
            .problems
            .iter()
            .find(|problem| {
                problem.text
                    == LearningProblemText::ConstantIndex {
                        index,
                        name: "dados".to_owned(),
                        lower_bound: 2,
                        upper_bound: 5,
                    }
            })
            .unwrap_or_else(|| {
                panic!(
                    "VisuAlg should flag index {index} outside 2..=5 on line {line}: {:?}",
                    hint.problems
                )
            });
        assert_eq!(problem.kind, LearningProblemKind::Logic);
        assert_eq!(problem.severity, LearningSeverity::Error);
        assert_eq!(problem.line, line);
        assert_eq!(problem.column, 12);
        assert_eq!(
            problem.suggestion.as_deref(),
            Some("Use um índice entre 2 e 5 em `dados`.")
        );
    }

    for line in [6, 7, 11, 12] {
        let hint = learning_hint(source, line, Language::Visualg);
        assert!(
            hint.problems.iter().all(|problem| {
                !matches!(problem.text, LearningProblemText::ConstantIndex { .. })
            }),
            "valid, variable, and string indexes must not receive a bound warning: {:?}",
            hint.problems
        );
    }

    let ambiguous = "algoritmo \"ambiguo\"\nvar\n  dados: vetor[2..5] de inteiro\n  dados: vetor[0..10] de inteiro\ninicio\n  escreval(dados[6])\nfimalgoritmo";
    let hint = learning_hint(ambiguous, 6, Language::Visualg);
    assert!(
        hint.problems
            .iter()
            .all(|problem| { !matches!(problem.text, LearningProblemText::ConstantIndex { .. }) }),
        "duplicate declarations must not select a guessed bound: {:?}",
        hint.problems
    );
}

#[test]
fn visualg_possible_logic_keeps_uncertain_wording_and_does_not_flag_loop_conditions() {
    let decision =
        "algoritmo \"condicoes\"\ninicio\n  se falso entao\n    escreval(1)\n  fimse\nfimalgoritmo";
    let hint = learning_hint(decision, 3, Language::Visualg);
    let warning = hint
        .problems
        .iter()
        .find(|problem| {
            problem.text == LearningProblemText::ConstantCondition { always_true: false }
        })
        .expect("the literal decision condition should be identified");
    assert_eq!(warning.kind, LearningProblemKind::PossibleLogic);
    assert_eq!(warning.severity, LearningSeverity::Warning);
    assert!(warning.message.contains("pode nunca"));

    let loop_source = "algoritmo \"repeticao\"\ninicio\n  enquanto falso faca\n    escreval(1)\n  fimenquanto\nfimalgoritmo";
    let hint = learning_hint(loop_source, 3, Language::Visualg);
    assert_eq!(hint.topic, LearningTopic::Loop);
    assert!(
        hint.problems.iter().all(|problem| {
            !matches!(problem.text, LearningProblemText::ConstantCondition { .. })
        })
    );
}

#[test]
fn mapler_array_bounds_ignore_variable_indexes_literals_comments_and_ambiguous_names() {
    let source = "variaveis\n dados: vetor[2..5] de inteiro;\n indice: inteiro;\ninicio\n escrever dados[indice];\n escrever \"dados[1]\";\n escrever dados[5]; // dados[1]\nfim.";

    for line in [5, 6, 7] {
        let hint = learning_hint(source, line, Language::Mapler);
        assert!(
            hint.problems.iter().all(|problem| {
                !matches!(problem.text, LearningProblemText::ConstantIndex { .. })
            }),
            "a variable index, string, or comment must not trigger a constant-bound warning: {:?}",
            hint.problems
        );
    }

    let ambiguous = "variaveis\n dados: vetor[2..5] de inteiro;\n dados: vetor[0..10] de inteiro;\ninicio\n escrever dados[6];\nfim.";
    let hint = learning_hint(ambiguous, 5, Language::Mapler);
    assert!(
        hint.problems
            .iter()
            .all(|problem| { !matches!(problem.text, LearningProblemText::ConstantIndex { .. }) }),
        "duplicate declarations must not select one guessed bound: {:?}",
        hint.problems
    );
}

#[test]
fn jerimum_import_advice_is_limited_to_known_typed_library_calls() {
    let expected_message = "A chamada pertence à biblioteca Jerimum, mas ela não foi importada. Adicione `use \"jerimum\"` no início do arquivo.";
    for (source, line, column) in [
        (
            "def desenhe(g: Graphics2D): Nada\n  g.fillRect(1, 2, 32, 32)\nfim",
            2,
            3,
        ),
        ("jogo.iniciar(\"Jogo\", 320, 200)", 1, 1),
        ("jogo.iniciar()", 1, 1),
        ("Desenho.incluir(1, desenhe)", 1, 1),
    ] {
        let hint = learning_hint(source, line, Language::Potigol);
        assert!(
            hint.problems.iter().any(|problem| {
                problem.kind == LearningProblemKind::StaticAnalysis
                    && problem.severity == LearningSeverity::Warning
                    && problem.line == line
                    && problem.column == column
                    && problem.message == expected_message
            }),
            "a known Jerimum call without its import should be identified at its location: {source}; {:?}",
            hint.problems
        );
    }

    for unknown_call in [
        "def desenhe(g: Graphics2D): Nada\n  g.mystery(1)\nfim",
        "g.fillRect(1, 2, 32, 32)",
        "# use \"jerimum\"\ng.fillRect(1, 2, 32, 32)",
        "Desenho.incluir(1)",
        "Desenho.incluir(1, desenhe, extra)",
        "jogo.iniciar(\"Jogo\", 320, 200, atualize, desenhe, 60, extra)",
    ] {
        let line = if unknown_call.starts_with("# ") {
            2
        } else if unknown_call.starts_with("def ") {
            2
        } else {
            1
        };
        let hint = learning_hint(unknown_call, line, Language::Potigol);
        assert!(
            hint.problems
                .iter()
                .all(|problem| !problem.message.contains("biblioteca Jerimum")),
            "an unknown or untyped call must not receive Jerimum advice: {unknown_call}; {:?}",
            hint.problems
        );
    }
}

#[test]
fn potigol_missing_output_advice_names_the_missing_value_only_in_that_context() {
    for (source, line, command) in [
        ("escreva\nescreva 2", 1, "escreva"),
        ("use \"jerimum\"\nescreva\nescreva 2", 2, "escreva"),
        ("imprima\nimprima 2", 1, "imprima"),
        ("use \"jerimum\"\nimprima\nimprima 2", 2, "imprima"),
    ] {
        let hint = learning_hint(source, line, Language::Potigol);
        let problem = hint
            .problems
            .iter()
            .find(|problem| {
                problem.kind == LearningProblemKind::Syntax
                    && problem.message == "esperava uma expressão"
            })
            .expect("the empty output command should retain its parser diagnostic");
        assert_eq!(problem.line, line);
        assert_eq!(problem.column, 1);
        assert_eq!(
            problem.suggestion.as_deref(),
            Some(
                format!("Informe o valor ou a expressão que será exibida após `{command}`.")
                    .as_str()
            )
        );
    }

    for (source, line) in [
        ("escreva 2", 1),
        ("use \"jerimum\"\nescreva 2", 2),
        ("imprima 2", 1),
        ("use \"jerimum\"\nimprima 2", 2),
    ] {
        let hint = learning_hint(source, line, Language::Potigol);
        assert!(hint.problems.iter().all(|problem| {
            problem.suggestion.as_deref()
                != Some("Informe o valor ou a expressão que será exibida após `escreva`.")
        }));
    }
}

fn execute_without_input(source: &str, language: Language) -> Result<(), Vec<ExecutionDiagnostic>> {
    let program = crate::features::execution::start(source.to_owned(), language, None, None, false);
    loop {
        match program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("execution should produce a completion event")
        {
            ExecutionEvent::Finished { result, .. } => return result,
            ExecutionEvent::Output(_)
            | ExecutionEvent::ClearOutput
            | ExecutionEvent::LineChanged(_)
            | ExecutionEvent::Debugger(_) => {}
            other => panic!("unexpected execution request in a no-input fixture: {other:?}"),
        }
    }
}

#[test]
fn division_by_zero_has_a_specific_common_next_step_and_nonzero_division_does_not() {
    let fixtures = [
        (
            "Portugol Studio 2.0",
            Language::Portugol,
            "programa {\n funcao inicio() {\n  escreva(1 / 0)\n }\n}",
            3,
        ),
        (
            "G-Portugol",
            Language::GPortugol,
            "algoritmo aula;\ninício\n  imprima(1 / 0);\nfim",
            3,
        ),
        (
            "Mapler",
            Language::Mapler,
            "variaveis\ninicio\n  escrever 1 / 0;\nfim.",
            3,
        ),
        (
            "VisuAlg",
            Language::Visualg,
            "algoritmo \"aula\"\ninicio\n  escreval(1 / 0)\nfimalgoritmo",
            3,
        ),
        (
            "Calango",
            Language::Calango,
            "algoritmo aula;\nprincipal\n  escreval(1 / 0);\nfimPrincipal",
            3,
        ),
        ("Potigol", Language::Potigol, "escreva (1 / 0)", 1),
    ];

    for (profile, language, source, line) in fixtures {
        let diagnostics = match execute_without_input(source, language) {
            Err(diagnostics) => diagnostics,
            Ok(()) => panic!("{profile} division by zero should fail at runtime"),
        };
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.kind == crate::features::execution::DiagnosticKind::Runtime
                    && matches!(
                        diagnostic.message.as_str(),
                        "divisão por zero"
                            | "divisão inteira por zero"
                            | "division by zero"
                            | "integer division by zero"
                    )
            })
            .unwrap_or_else(|| {
                panic!("{profile} did not report the expected runtime error: {diagnostics:?}")
            });
        assert_eq!(diagnostic.line, line, "{profile}");

        let hint = learning_hint_with_runtime(source, diagnostic.line, language, &diagnostics);
        let problem = hint
            .problems
            .iter()
            .find(|problem| {
                problem.text == LearningProblemText::InterpreterDiagnostic
                    && problem.line == diagnostic.line
                    && problem.message == diagnostic.message
            })
            .unwrap_or_else(|| panic!("{profile} original runtime diagnostic was not preserved"));
        assert_eq!(
            problem.suggestion.as_deref(),
            Some("Confira se o divisor pode ser zero antes de fazer a divisão."),
            "{profile}"
        );
        assert_eq!(problem.kind, LearningProblemKind::Runtime, "{profile}");
        assert_eq!(problem.line, diagnostic.line, "{profile}");
        assert_eq!(problem.column, diagnostic.column, "{profile}");
        assert_eq!(problem.message, diagnostic.message, "{profile}");

        let valid_source = source.replace("1 / 0", "1 / 2");
        let valid_result = execute_without_input(&valid_source, language);
        assert!(
            valid_result.is_ok(),
            "{profile} nonzero division should execute: {valid_result:?}"
        );
        let valid_hint = learning_hint_with_runtime(&valid_source, line, language, &[]);
        assert!(
            valid_hint.problems.iter().all(|problem| {
                problem.suggestion.as_deref()
                    != Some("Confira se o divisor pode ser zero antes de fazer a divisão.")
            }),
            "{profile} must not suggest a division-by-zero fix for valid division"
        );
    }
}
