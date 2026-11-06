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
        outcome => panic!("valid source should produce complete analysis: {outcome:?}"),
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
fn analyzes_visualg_procedures_functions_and_return_types() {
    let source = r#"algoritmo "analise"
procedimento mostrar(valor: inteiro)
inicio
  escreva(valor)
fimprocedimento
funcao nome(valor: inteiro): caractere
inicio
  retorne "Ana"
fimfuncao
inicio
  mostrar(1)
fimalgoritmo
"#;

    let procedure = analyze_source_line(source, line_of(source, "procedimento"))
        .expect("procedure declaration should be found");
    assert_eq!(procedure.construct, SourceConstruct::Procedure);
    assert_eq!(procedure.name.as_deref(), Some("mostrar"));
    assert_eq!(procedure.return_type, None);
    assert_eq!(procedure.parameters[0].name, "valor");
    assert_eq!(procedure.parameters[0].type_name, "inteiro");

    let function = analyze_source_line(source, line_of(source, "funcao"))
        .expect("function declaration should be found");
    assert_eq!(function.construct, SourceConstruct::Function);
    assert_eq!(function.name.as_deref(), Some("nome"));
    assert_eq!(function.return_type.as_deref(), Some("caractere"));

    let return_statement = analyze_source_line(source, line_of(source, "retorne"))
        .expect("return statement should be found");
    assert_eq!(return_statement.construct, SourceConstruct::Return);
    assert_eq!(return_statement.return_type.as_deref(), Some("caractere"));
}

#[test]
fn analyzes_visualg_scalar_vector_matrix_and_statement_kinds() {
    let source = r#"algoritmo "analise"
var
  total: inteiro
  valores: vetor[1..3] de inteiro
  matriz: vetor[0..1, 2..4] de real
procedimento atualizar(var valor: inteiro)
inicio
  valor <- valor + 1
fimprocedimento
inicio
  total <- 1
  leia(total)
  escreval(total)
  se total > 0 entao
    escreval("positivo")
  fimse
  enquanto total > 0 faca
    total <- total - 1
  fimenquanto
fimalgoritmo
"#;

    let scalar = analyze_source_line(source, line_of(source, "total: inteiro"))
        .expect("scalar declaration should be found");
    assert_eq!(scalar.construct, SourceConstruct::VariableDeclaration);
    assert_eq!(scalar.variables[0].name, "total");
    assert_eq!(scalar.variables[0].type_name, "inteiro");
    assert!(scalar.variables[0].dimensions.is_empty());

    let vector = analyze_source_line(source, line_of(source, "valores:"))
        .expect("vector declaration should be found");
    assert_eq!(vector.variables[0].dimensions.len(), 1);
    assert_eq!(vector.variables[0].dimensions[0].lower_bound, Some(1));
    assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(3));

    let matrix = analyze_source_line(source, line_of(source, "matriz:"))
        .expect("matrix declaration should be found");
    assert_eq!(matrix.variables[0].dimensions.len(), 2);

    let assignment = analyze_source_line(source, line_of(source, "total <-"))
        .expect("assignment should be found");
    assert_eq!(assignment.construct, SourceConstruct::Assignment);

    for (prefix, expected) in [
        ("leia", SourceConstruct::Input),
        ("escreval(total)", SourceConstruct::Output),
        ("se total", SourceConstruct::Decision),
        ("enquanto total", SourceConstruct::Loop),
    ] {
        let context = analyze_source_line(source, line_of(source, prefix))
            .expect("statement should be found");
        assert_eq!(context.construct, expected, "line starting with {prefix:?}");
    }
}

#[test]
fn reports_no_context_for_an_unclassified_line_and_source_errors_for_incomplete_code() {
    let source = "algoritmo \"analise\"\nvar\n  // comentário\ninicio\nfimalgoritmo\n";
    assert!(analyze_source_line(source, 3).is_none());

    let incomplete = "algoritmo \"analise\"\ninicio\n  total <-\nfimalgoritmo\n";
    let SourceLineAnalysisOutcome::Complete {
        analysis,
        diagnostics,
    } = profile_adapter(incomplete, 3, &|| false)
    else {
        panic!("masking the malformed statement should leave a complete prefix");
    };
    assert!(analysis.is_none());
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.line == 3));
}

#[test]
fn recovers_independent_syntax_errors_on_separate_lines() {
    let source =
        "algoritmo \"analise\"\ninicio\n  total : inteiro\n  valor : inteiro\nfimalgoritmo\n";
    let SourceLineAnalysisOutcome::Complete {
        analysis,
        diagnostics,
    } = profile_adapter(source, 4, &|| false)
    else {
        panic!("both independent statements should be recoverable");
    };
    assert!(analysis.is_none());
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == SourceDiagnosticKind::Syntax)
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.line == 3));
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.line == 4));
}

#[test]
fn shared_learning_recovery_preserves_error_and_analyzes_later_visualg_lines() {
    let source = "algoritmo \"recupera\"\ninicio\n@\nescreval(\"depois\")\nfimalgoritmo";
    let expected = potoo_visualg_interpreter::parse(source)
        .expect_err("the first body line has an invalid character")
        .remove(0);
    let SourceLineAnalysisOutcome::Complete {
        analysis: Some(analysis),
        diagnostics,
    } = profile_adapter(source, 4, &|| false)
    else {
        panic!("the valid later output line should be recovered");
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
fn shared_learning_recovery_discards_cancelled_visualg_work() {
    assert!(matches!(
        profile_adapter("algoritmo \"cancelado\"\ninicio\nfimalgoritmo", 1, &|| true),
        SourceLineAnalysisOutcome::Cancelled
    ));
}
