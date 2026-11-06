use super::profile_adapter;
use crate::languages::model::{
    SourceConstruct, SourceDiagnosticKind, SourceLineAnalysis, SourceLineAnalysisOutcome,
};

fn analyze_source_line(source: &str, line: usize) -> Option<SourceLineAnalysis> {
    match profile_adapter(source, line, &|| false) {
        SourceLineAnalysisOutcome::Complete {
            analysis,
            diagnostics,
        } => {
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            analysis
        }
        outcome => panic!("valid Calango source should be complete: {outcome:?}"),
    }
}

fn line_of(source: &str, prefix: &str) -> usize {
    source
        .lines()
        .position(|line| line.trim_start().starts_with(prefix))
        .expect("source line should exist")
        + 1
}

#[test]
fn analyzes_calango_program_main_routines_and_nested_statements() {
    let source = "algoritmo aula;\nprocedimento mostrar(texto nome)\nescreval(nome);\nfimProcedimento\nfuncao inteiro dobrar(inteiro valor)\nretorna valor * 2;\nfimFuncao\nprincipal\ninteiro total;\ntotal = leia();\nescreva(total);\nse (total > 0) entao\ntotal = total + 1;\nfimSe\npara (total de 1 ate 2 passo 1) faca\nescreval(total);\nfimPara\nfaca\ntotal = total - 1;\nenquanto total > 0;\nfimPrincipal";

    let program = analyze_source_line(source, line_of(source, "algoritmo"))
        .expect("program header should have a construct");
    assert_eq!(program.construct, SourceConstruct::Program);

    let procedure = analyze_source_line(source, line_of(source, "procedimento"))
        .expect("procedure declaration should be found");
    assert_eq!(procedure.construct, SourceConstruct::Procedure);
    assert_eq!(procedure.name.as_deref(), Some("mostrar"));
    assert_eq!(procedure.parameters[0].name, "nome");
    assert_eq!(procedure.parameters[0].type_name, "texto");

    let function = analyze_source_line(source, line_of(source, "funcao"))
        .expect("function declaration should be found");
    assert_eq!(function.construct, SourceConstruct::Function);
    assert_eq!(function.name.as_deref(), Some("dobrar"));
    assert_eq!(function.return_type.as_deref(), Some("inteiro"));

    let main = line_of(source, "principal");
    assert!(analyze_source_line(source, main).is_none());

    for (prefix, expected, name) in [
        ("inteiro total", SourceConstruct::VariableDeclaration, None),
        ("total = leia", SourceConstruct::Input, Some("leia")),
        ("escreva(total)", SourceConstruct::Output, Some("escreva")),
        ("se (", SourceConstruct::Decision, Some("se")),
        ("para (", SourceConstruct::Loop, Some("para")),
        ("faca", SourceConstruct::Loop, Some("faca")),
    ] {
        let analysis = analyze_source_line(source, line_of(source, prefix))
            .expect("statement should be found in the AST");
        assert_eq!(analysis.construct, expected, "line {prefix:?}");
        assert_eq!(analysis.name.as_deref(), name, "line {prefix:?}");
    }
}

#[test]
fn shared_recovery_preserves_calango_diagnostic_and_analyzes_a_later_line() {
    let source = "algoritmo recupera;\nprincipal\n@;\nescreval(\"depois\");\nfimPrincipal";
    let expected = potoo_calango_interpreter::parse(source)
        .expect_err("the first main-body statement is invalid")
        .remove(0);
    let SourceLineAnalysisOutcome::Complete {
        analysis: Some(analysis),
        diagnostics,
    } = profile_adapter(source, 4, &|| false)
    else {
        panic!("the valid output after the error should be recoverable");
    };

    assert_eq!(analysis.construct, SourceConstruct::Output);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(
        (diagnostics[0].line, diagnostics[0].column),
        (expected.line, expected.column)
    );
}

#[test]
fn shared_recovery_preserves_calango_syntax_diagnostic_and_analyzes_a_later_line() {
    let source = "algoritmo sintaxe;\nprincipal\nescreva(1, , 2);\nescreval(3);\nfimPrincipal";
    let expected = potoo_calango_interpreter::parse(source)
        .expect_err("the missing output argument is invalid syntax")
        .remove(0);
    assert_eq!(
        expected.kind,
        potoo_calango_interpreter::DiagnosticKind::Syntax
    );

    let SourceLineAnalysisOutcome::Complete {
        analysis: Some(analysis),
        diagnostics,
    } = profile_adapter(source, 4, &|| false)
    else {
        panic!("the valid output after the syntax error should be recoverable");
    };

    assert_eq!(analysis.construct, SourceConstruct::Output);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::Syntax);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(
        (diagnostics[0].line, diagnostics[0].column),
        (expected.line, expected.column)
    );
}

#[test]
fn shared_recovery_marks_an_unbalanced_expression_incomplete() {
    let source = "algoritmo incompleto;\nprincipal\nescreva(1 + @;\nfimPrincipal";
    let expected = potoo_calango_interpreter::parse(source)
        .expect_err("the invalid character is inside an unclosed call")
        .remove(0);
    let SourceLineAnalysisOutcome::Incomplete { diagnostics } =
        profile_adapter(source, expected.line, &|| false)
    else {
        panic!("an unbalanced statement range should fail closed");
    };

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(
        (diagnostics[0].line, diagnostics[0].column),
        (expected.line, expected.column)
    );
}

#[test]
fn shared_recovery_discards_cancelled_calango_work() {
    assert!(matches!(
        profile_adapter("algoritmo cancelado;\nprincipal\nfimPrincipal", 1, &|| true),
        SourceLineAnalysisOutcome::Cancelled
    ));
}
