use super::{
    LearningProblemKind, LearningProblemText, LearningSeverity, LearningTopic, learning_hint,
    learning_hint_with_runtime,
};
use crate::features::execution::{DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic};
use crate::languages::model::Language;
use potoo_gportugol_interpreter::parse;

#[test]
fn learning_hint_tracks_active_line_and_updated_condition() {
    let source = "algoritmo \"votacao\"\nvar\nidade: inteiro\ninicio\nescreva(\"idade?\")\nleia(idade)\nse idade >= 18 entao\nescreval(\"sim\")\nfimse\nfimalgoritmo";
    let decision = learning_hint(source, 7, Language::Visualg);
    assert_eq!(decision.topic, LearningTopic::Decision);
    assert_eq!(decision.expression.as_deref(), Some("idade >= 18"));
    assert_eq!(
        decision.experiment_expression.as_deref(),
        Some("idade >= 16")
    );
    let input = learning_hint(source, 6, Language::Visualg);
    assert_eq!(input.topic, LearningTopic::Input);

    let edited = source.replace("18", "16");
    let changed = learning_hint(&edited, 7, Language::Visualg);
    assert_eq!(changed.expression.as_deref(), Some("idade >= 16"));
    assert_eq!(
        changed.experiment_expression.as_deref(),
        Some("idade >= 14")
    );
}

#[test]
fn visualg_learning_marks_an_unclosed_multiline_expression_incomplete() {
    let source = "algoritmo \"expressao\"\ninicio\nescreval(1 +\n @\n 2\nfimalgoritmo";
    let expected = potoo_visualg_interpreter::parse(source)
        .expect_err("the invalid token is inside an unclosed multiline expression")
        .remove(0);
    let hint = learning_hint(source, expected.line, Language::Visualg);

    assert_eq!(hint.topic, LearningTopic::Unrecognized);
    assert!(hint.problems.iter().any(|problem| {
        problem.text == LearningProblemText::InterpreterDiagnostic
            && problem.line == expected.line
            && problem.column == expected.column
    }));
    assert!(hint.problems.iter().any(|problem| {
        problem.kind == LearningProblemKind::Analysis
            && problem.severity == LearningSeverity::Warning
            && problem.text == LearningProblemText::AnalysisIncomplete
    }));
}

#[test]
fn loop_condition_experiment_always_changes_a_zero_threshold() {
    for (condition, expected) in [("idade > 0", "idade > 2"), ("idade > -1", "idade > -3")] {
        let source = format!(
            "algoritmo \"aula\"\nvar\nidade: inteiro\ninicio\nenquanto {condition} faca\n idade <- idade - 1\nfimenquanto\nfimalgoritmo"
        );
        let loop_hint = learning_hint(&source, 5, Language::Visualg);

        assert_eq!(loop_hint.topic, LearningTopic::Loop);
        assert_eq!(loop_hint.expression.as_deref(), Some(condition));
        assert_eq!(loop_hint.experiment_expression.as_deref(), Some(expected));
        assert_ne!(loop_hint.expression, loop_hint.experiment_expression);
        assert!(
            !loop_hint
                .experiment_expression
                .as_deref()
                .expect("this condition has an integer literal")
                .contains("--")
        );
    }
}

#[test]
fn learning_hint_recognizes_output_loops_and_unclassified_lines() {
    assert_eq!(
        learning_hint(
            "algoritmo \"aula\"\ninicio\nescreval(1)\nfimalgoritmo",
            3,
            Language::Visualg
        )
        .topic,
        LearningTopic::Output
    );
    assert_eq!(
            learning_hint(
                "algoritmo \"aula\"\ninicio\nenquanto x < 3 faca\n escreval(x)\nfimenquanto\nfimalgoritmo",
                3,
                Language::Visualg
            )
            .topic,
            LearningTopic::Loop
        );
    assert_eq!(
        learning_hint(
            "programa {\n funcao inicio() {\n  escreva(1)\n }\n}",
            3,
            Language::Portugol
        )
        .topic,
        LearningTopic::Output
    );
    assert_eq!(
        learning_hint("fimse", 1, Language::Visualg).topic,
        LearningTopic::Unrecognized
    );
}

#[test]
fn learning_hint_uses_ast_for_all_supported_languages_and_keeps_unknown_lines_unknown() {
    let potigol = learning_hint("escreva \"Olá\"", 1, Language::Potigol);
    assert_eq!(potigol.topic, LearningTopic::Output);

    let calango = "algoritmo aula;\nprincipal\n  escreval(\"Olá\");\nfimPrincipal";
    assert_eq!(
        learning_hint(calango, 3, Language::Calango).topic,
        LearningTopic::Output
    );

    let mapler = "variaveis\n  x: inteiro;\ninicio\n  escrever \"Olá\";\nfim.";
    assert_eq!(
        learning_hint(mapler, 4, Language::Mapler).topic,
        LearningTopic::Output
    );

    let unknown = learning_hint("??", 1, Language::Potigol);
    assert_eq!(unknown.topic, LearningTopic::Unrecognized);
}

#[test]
fn blank_and_comment_lines_have_no_construct_or_problem_in_every_dialect() {
    let cases = [
        ("# comentário\n", Language::Potigol),
        ("// comentário\n", Language::Portugol),
        ("// comentário\n", Language::Visualg),
        ("// comentário\n", Language::GPortugol),
        ("// comentário\n", Language::Calango),
        ("// comentário\n", Language::Mapler),
    ];
    for (source, language) in cases {
        let hint = learning_hint(source, 1, language);
        assert_eq!(hint.topic, LearningTopic::Unrecognized, "{language:?}");
        assert!(hint.problems.is_empty(), "{language:?}: {source}");
        assert!(hint.call.is_none(), "{language:?}: {source}");
    }

    let incomplete_potigol_comment = learning_hint("# comentário", 1, Language::Potigol);
    assert!(
        !incomplete_potigol_comment.problems.is_empty(),
        "keep the Potigol parser's EOF diagnostic for an unterminated comment"
    );
}

#[test]
fn learning_hint_recognizes_jerimum_calls_only_when_the_library_is_imported() {
    let imported = "use \"jerimum\"\ndesenhe_personagem(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim";
    let hint = learning_hint(imported, 3, Language::Potigol);
    assert_eq!(hint.topic, LearningTopic::LibraryCall);
    let call = hint
        .call
        .expect("the method call should be recovered from the AST");
    assert_eq!(call.receiver.as_deref(), Some("g"));
    assert_eq!(call.member, "setColor");
    assert_eq!(call.library.as_deref(), Some("jerimum"));
    assert_eq!(call.arguments, ["Cor.VERDE.color"]);

    let classic = learning_hint("g.setColor(Cor.VERDE.color)", 1, Language::Potigol);
    assert_eq!(classic.topic, LearningTopic::Call);
    assert_eq!(
        classic.call.and_then(|call| call.library),
        None,
        "a Jerimum API must not be inferred without its import"
    );

    let imported_but_untyped = "use \"jerimum\"\ng.setColor(Cor.VERDE.color)";
    let hint = learning_hint(imported_but_untyped, 2, Language::Potigol);
    assert_eq!(hint.topic, LearningTopic::Call);
    assert_eq!(hint.call.and_then(|call| call.library), None);
}

#[test]
fn potigol_input_builtins_are_all_explained_as_input() {
    for name in [
        "leia_inteiro",
        "leia_real",
        "leia_numero",
        "leia_número",
        "leia_texto",
        "leia_caractere",
        "leia_logico",
        "leia_inteiros",
        "leia_reais",
        "leia_numeros",
        "leia_números",
        "leia_textos",
    ] {
        let source = format!("var valor := {name}()\n");
        let hint = learning_hint(&source, 1, Language::Potigol);
        assert_eq!(hint.topic, LearningTopic::Input, "{name}");
        assert_eq!(
            hint.call.as_ref().map(|call| call.member.as_str()),
            Some(name)
        );
    }
}

#[test]
fn output_hints_preserve_the_dialect_command_name() {
    let cases = [
        ("escreva \"Olá\"", 1, Language::Potigol, "escreva"),
        ("imprima \"Olá\"", 1, Language::Potigol, "imprima"),
        (
            "algoritmo \"aula\"\ninicio\nescreva(\"Olá\")\nfimalgoritmo",
            3,
            Language::Visualg,
            "escreva",
        ),
        (
            "algoritmo \"aula\"\ninicio\nescreval(\"Olá\")\nfimalgoritmo",
            3,
            Language::Visualg,
            "escreval",
        ),
        (
            "programa {\n funcao inicio() {\n escreva(\"Olá\")\n }\n}",
            3,
            Language::Portugol,
            "escreva",
        ),
        (
            "algoritmo aula;\ninício\n imprima(\"Olá\");\nfim",
            3,
            Language::GPortugol,
            "imprima",
        ),
        (
            "algoritmo aula;\nprincipal\n escreval(\"Olá\");\nfimPrincipal",
            3,
            Language::Calango,
            "escreval",
        ),
        (
            "variaveis\n x: inteiro;\ninicio\n escrever \"Olá\";\nfim.",
            4,
            Language::Mapler,
            "escrever",
        ),
    ];

    for (source, line, language, command) in cases {
        let hint = learning_hint(source, line, language);
        assert_eq!(hint.topic, LearningTopic::Output, "{language:?}: {source}");
        assert_eq!(
            hint.name.as_deref(),
            Some(command),
            "{language:?}: {source}"
        );
    }
}

#[test]
fn learning_hint_maps_dialect_specific_routine_and_array_facts() {
    let portugol = "programa {\n funcao vazio registrar(inteiro valor) {}\n funcao inicio() {\n  inteiro valores[3]\n }\n}";
    let routine = learning_hint(portugol, 2, Language::Portugol);
    assert_eq!(routine.topic, LearningTopic::Function);
    assert_eq!(routine.name.as_deref(), Some("registrar"));
    assert_eq!(routine.return_type.as_deref(), Some("vazio"));
    assert_eq!(routine.parameters[0].name, "valor");
    assert_eq!(routine.parameters[0].type_name, "inteiro");

    let visualg =
        "algoritmo \"analise\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\nfimalgoritmo\n";
    let vector = learning_hint(visualg, 3, Language::Visualg);
    assert_eq!(vector.topic, LearningTopic::Variable);
    assert_eq!(vector.variables[0].name, "dados");
    assert_eq!(vector.variables[0].type_name, "inteiro");
    assert_eq!(vector.variables[0].dimensions[0].lower_bound, Some(2));
    assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(5));
}

#[test]
fn gportugol_learning_hint_identifies_constructs_and_symbols() {
    let source = "algoritmo aula;\n\
variáveis\n\
  idade: inteiro;\n\
  valores: matriz[3] de inteiros;\n\
fim-variáveis\n\
início\n\
  idade := 17;\n\
  idade := leia();\n\
  imprima(idade);\n\
  enquanto idade > 0 faça\n\
    idade := idade - 1;\n\
  fim-enquanto\n\
  se idade >= 18 então\n\
    imprima(\"maior\");\n\
  fim-se\n\
fim\n\
função soma(a: inteiro, b: real): real\n\
início\n\
  retorne a + b;\n\
fim";

    let variable = learning_hint(source, 3, Language::GPortugol);
    assert_eq!(variable.topic, LearningTopic::Variable);
    assert_eq!(variable.variables[0].name, "idade");
    assert_eq!(variable.variables[0].type_name, "inteiro");

    let vector = learning_hint(source, 4, Language::GPortugol);
    assert_eq!(vector.variables[0].dimensions[0].lower_bound, Some(0));
    assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(2));

    assert_eq!(
        learning_hint(source, 7, Language::GPortugol).topic,
        LearningTopic::Assignment
    );
    assert_eq!(
        learning_hint(source, 8, Language::GPortugol).topic,
        LearningTopic::Input
    );

    let output = learning_hint(source, 9, Language::GPortugol);
    assert_eq!(output.topic, LearningTopic::Output);
    assert_eq!(
        learning_hint(source, 10, Language::GPortugol).topic,
        LearningTopic::Loop
    );

    let decision = learning_hint(source, 13, Language::GPortugol);
    assert_eq!(decision.topic, LearningTopic::Decision);
    assert_eq!(decision.expression.as_deref(), Some("idade >= 18"));
    assert_eq!(
        decision.experiment_expression.as_deref(),
        Some("idade >= 16")
    );

    let function = learning_hint(source, 17, Language::GPortugol);
    assert_eq!(function.topic, LearningTopic::Function);
    assert_eq!(function.name.as_deref(), Some("soma"));
    assert_eq!(function.return_type.as_deref(), Some("real"));
    assert_eq!(function.parameters[0].name, "a");
    assert_eq!(function.parameters[0].type_name, "inteiro");
    assert_eq!(function.parameters[1].name, "b");
    assert_eq!(function.parameters[1].type_name, "real");

    let return_value = learning_hint(source, 19, Language::GPortugol);
    assert_eq!(return_value.topic, LearningTopic::Return);
    assert_eq!(return_value.return_type.as_deref(), Some("real"));
    assert_eq!(return_value.expression.as_deref(), Some("a + b"));
}

#[test]
fn gportugol_learning_hint_reports_syntax_errors_on_the_selected_line() {
    let source =
        "algoritmo aula;\ninício\n  se idade >= então\n    imprima(\"ok\");\n  fim-se\nfim";
    let hint = learning_hint(source, 3, Language::GPortugol);

    assert_eq!(hint.topic, LearningTopic::Unrecognized);
    assert!(hint.problems.iter().any(|problem| {
        problem.kind == LearningProblemKind::Syntax && problem.line == 3 && problem.column > 0
    }));
}

#[test]
fn gportugol_line_comment_with_bare_cr_does_not_hide_the_next_line_error() {
    let source = "algoritmo teste;\rinício\r// comentário\r/*\n@;\rimprima(1);\rfim";
    let expected = parse(source)
        .expect_err("the token after the line comment should be invalid")
        .remove(0);
    assert_eq!(expected.span.start.line, 5);
    assert_eq!(expected.span.start.column, 1);

    let hint = learning_hint(source, expected.span.start.line, Language::GPortugol);
    assert_eq!(hint.topic, LearningTopic::Unrecognized);
    assert!(
        hint.problems
            .iter()
            .any(|problem| problem.line == 5 && problem.column == 1),
        "the original lexical diagnostic must stay visible: {:?}",
        hint.problems
    );
}

#[test]
fn gportugol_multiline_character_literal_does_not_hide_the_following_error() {
    let source = "algoritmo teste;\ninício\nimprima('\n');\n@;\nimprima(1);\nfim";
    let expected = parse(source)
        .expect_err("the standalone invalid token should be diagnosed")
        .remove(0);
    assert_eq!(expected.span.start.line, 5);
    assert_eq!(expected.span.start.column, 1);

    let hint = learning_hint(source, expected.span.start.line, Language::GPortugol);
    assert_eq!(hint.topic, LearningTopic::Unrecognized);
    assert!(
        hint.problems.iter().any(|problem| {
            problem.line == expected.span.start.line && problem.column == expected.span.start.column
        }),
        "the original diagnostic must survive recovery: {:?}",
        hint.problems
    );
}

#[test]
fn gportugol_bom_keeps_first_line_diagnostics_aligned_during_recovery() {
    let source = "\u{feff}algoritmo teste; início imprima(1); @; imprima(2); fim";
    let expected = parse(source)
        .expect_err("the invalid token should be diagnosed")
        .remove(0);
    let hint = learning_hint(source, expected.span.start.line, Language::GPortugol);

    assert_eq!(expected.span.start.line, 1);
    assert_eq!(hint.topic, LearningTopic::Unrecognized);
    assert!(
        hint.problems.iter().any(|problem| {
            problem.line == expected.span.start.line && problem.column == expected.span.start.column
        }),
        "the BOM must not shift the original diagnostic: {:?}",
        hint.problems
    );
}

#[test]
fn learning_hint_reports_line_specific_syntax_and_possible_logic_problems() {
    let source =
        "programa {\n funcao inicio() {\n  se (verdadeiro) {\n   escreva(\"ok\")\n  }\n }\n}";
    let hint = learning_hint(source, 3, Language::Portugol);
    assert!(hint.problems.iter().any(|problem| {
        problem.kind == LearningProblemKind::PossibleLogic
            && problem.severity == LearningSeverity::Warning
            && problem.line == 3
    }));
    let incomplete = learning_hint("programa {\n funcao inicio( {\n}", 2, Language::Portugol);
    assert_eq!(incomplete.topic, LearningTopic::Unrecognized);
    assert_eq!(incomplete.name, None);
    assert!(incomplete.parameters.is_empty());
    assert!(
        incomplete
            .problems
            .iter()
            .any(|problem| problem.kind == LearningProblemKind::Syntax)
    );
}

#[test]
fn parser_errors_stay_on_their_line_across_all_supported_dialects() {
    let cases = [
        ("escreva \"ok\"\n@\n", 2, Language::Potigol),
        ("programa {\n @\n}", 2, Language::Portugol),
        (
            "algoritmo \"aula\"\ninicio\n@\nfimalgoritmo",
            3,
            Language::Visualg,
        ),
        ("algoritmo aula;\ninício\n@\nfim", 3, Language::GPortugol),
        (
            "algoritmo aula;\nprincipal\n@\nfimPrincipal",
            3,
            Language::Calango,
        ),
        (
            "variaveis\n x: inteiro;\ninicio\n@\nfim.",
            4,
            Language::Mapler,
        ),
    ];
    for (source, error_line, language) in cases {
        let error = learning_hint(source, error_line, language);
        assert_eq!(error.topic, LearningTopic::Unrecognized, "{language:?}");
        assert!(
            error
                .problems
                .iter()
                .any(|problem| problem.line == error_line),
            "{language:?}: {source}"
        );
        let valid = learning_hint(source, 1, language);
        assert!(
            valid.problems.is_empty(),
            "an error on another line leaked into {language:?}: {source}"
        );
    }
}

#[test]
fn portugol_learning_recovers_ast_facts_around_a_lexical_error() {
    let source = r#"programa {
 funcao inicio() {
  escreva("antes")
  @
  escreva("depois")
 }
}"#;

    let before = learning_hint(source, 3, Language::Portugol);
    assert_eq!(before.topic, LearningTopic::Output);
    assert!(before.problems.is_empty());

    let error = learning_hint(source, 4, Language::Portugol);
    assert_eq!(error.topic, LearningTopic::Unrecognized);
    assert!(error.problems.iter().any(|problem| {
        problem.kind == LearningProblemKind::Syntax && problem.line == 4 && problem.column == 3
    }));

    let after = learning_hint(source, 5, Language::Portugol);
    assert_eq!(after.topic, LearningTopic::Output);
    assert!(after.problems.is_empty());
}

#[test]
fn portugol_learning_recovers_ast_facts_around_a_syntax_error() {
    let source = r#"programa {
 funcao inicio() {
  escreva("antes")
  inteiro = 1
  escreva("depois")
 }
}"#;

    let before = learning_hint(source, 3, Language::Portugol);
    assert_eq!(before.topic, LearningTopic::Output);
    assert!(before.problems.is_empty());

    let error = learning_hint(source, 4, Language::Portugol);
    assert_eq!(error.topic, LearningTopic::Unrecognized);
    assert!(
        error
            .problems
            .iter()
            .any(|problem| { problem.kind == LearningProblemKind::Syntax && problem.line == 4 })
    );

    let after = learning_hint(source, 5, Language::Portugol);
    assert_eq!(after.topic, LearningTopic::Output);
    assert!(after.problems.is_empty());
}

#[test]
fn portugol_learning_marks_ambiguous_recovery_incomplete_without_ast_facts() {
    let source = "programa {\n funcao inicio() {\n  escreva(\n }\n}";
    let hint = learning_hint(source, 4, Language::Portugol);

    assert_eq!(hint.topic, LearningTopic::Unrecognized);
    assert!(hint.name.is_none());
    assert!(hint.variables.is_empty());
    assert!(hint.problems.iter().any(|problem| {
        problem.kind == LearningProblemKind::Analysis
            && problem.severity == LearningSeverity::Warning
    }));
}

#[test]
fn potigol_field_assignment_advice_uses_potigol_syntax_in_the_learning_panel() {
    let source = "use \"jerimum\"\njogo.titulo = \"teste\"\n";
    let error = learning_hint(source, 2, Language::Potigol);
    assert!(error.problems.iter().any(|problem| {
        problem.line == 2
            && problem.kind == LearningProblemKind::Syntax
            && problem.message.contains("usa ':='")
    }));

    let import_line = learning_hint(source, 1, Language::Potigol);
    assert!(import_line.problems.is_empty());
}

#[test]
fn runtime_diagnostics_are_shown_only_on_their_source_line() {
    let diagnostics = [ExecutionDiagnostic {
        kind: ExecutionDiagnosticKind::Runtime,
        message: "índice fora dos limites".to_owned(),
        line: 2,
        column: 4,
    }];
    let source = "programa {\n inteiro x = 1\n}";
    let selected = learning_hint_with_runtime(source, 2, Language::Portugol, &diagnostics);
    let other_line = learning_hint_with_runtime(source, 3, Language::Portugol, &diagnostics);
    assert!(selected.problems.iter().any(|problem| {
        problem.kind == LearningProblemKind::Runtime && problem.message == "índice fora dos limites"
    }));
    assert!(
        !other_line
            .problems
            .iter()
            .any(|problem| problem.kind == LearningProblemKind::Runtime)
    );
}

#[test]
fn reports_only_proven_constant_indexes_outside_declared_bounds() {
    let portugol = "programa {\n funcao inicio() {\n  inteiro valores[3]\n  escreva(valores[3])\n  escreva(valores[2])\n }\n}";
    let out_of_range = learning_hint(portugol, 4, Language::Portugol);
    let declaration = learning_hint(portugol, 3, Language::Portugol);
    let valid = learning_hint(portugol, 5, Language::Portugol);
    assert!(out_of_range.problems.iter().any(|problem| {
        problem.kind == LearningProblemKind::Logic
            && problem.severity == LearningSeverity::Error
            && problem.suggestion.as_deref() == Some("Use um índice entre 0 e 2 em `valores`.")
    }));
    assert!(
        !valid
            .problems
            .iter()
            .any(|problem| problem.kind == LearningProblemKind::Logic)
    );
    assert!(
        !declaration
            .problems
            .iter()
            .any(|problem| problem.kind == LearningProblemKind::Logic)
    );

    let visualg = "algoritmo \"analise\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\n  escreva(dados[1])\nfimalgoritmo\n";
    let visualg_index = learning_hint(visualg, 5, Language::Visualg);
    assert!(
        visualg_index
            .problems
            .iter()
            .any(|problem| problem.kind == LearningProblemKind::Logic)
    );

    let commented_index = "programa {\n funcao inicio() {\n  inteiro valores[3]\n  escreva(valores[2] /* valores[3] */)\n }\n}";
    let commented_index = learning_hint(commented_index, 4, Language::Portugol);
    assert!(
        !commented_index
            .problems
            .iter()
            .any(|problem| problem.kind == LearningProblemKind::Logic),
        "indexes inside block comments must not produce learning problems"
    );
}

#[test]
fn type_diagnostics_keep_their_own_learning_category() {
    let diagnostics = [ExecutionDiagnostic {
        kind: ExecutionDiagnosticKind::Type,
        message: "tipos incompatíveis: esperava Inteiro, recebeu Lógico".to_owned(),
        line: 1,
        column: 5,
    }];
    let hint =
        learning_hint_with_runtime("valor = 1 + verdadeiro", 1, Language::Potigol, &diagnostics);
    assert!(
        hint.problems
            .iter()
            .any(|problem| problem.kind == LearningProblemKind::Type)
    );
}
