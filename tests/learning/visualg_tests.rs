use super::analyze_source_line_cancellable;
use crate::languages::model::{SourceConstruct, SourceLineAnalysis};

fn analyze_source_line(
    source: &str,
    line: usize,
) -> Result<Option<SourceLineAnalysis>, Vec<potoo_visualg_interpreter::Diagnostic>> {
    analyze_source_line_cancellable(source, line, || false)
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
        .expect("valid VisuAlg source")
        .expect("procedure declaration should be found");
    assert_eq!(procedure.construct, SourceConstruct::Procedure);
    assert_eq!(procedure.name.as_deref(), Some("mostrar"));
    assert_eq!(procedure.return_type, None);
    assert_eq!(procedure.parameters[0].name, "valor");
    assert_eq!(procedure.parameters[0].type_name, "inteiro");

    let function = analyze_source_line(source, line_of(source, "funcao"))
        .expect("valid VisuAlg source")
        .expect("function declaration should be found");
    assert_eq!(function.construct, SourceConstruct::Function);
    assert_eq!(function.name.as_deref(), Some("nome"));
    assert_eq!(function.return_type.as_deref(), Some("caractere"));

    let return_statement = analyze_source_line(source, line_of(source, "retorne"))
        .expect("valid VisuAlg source")
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
        .expect("valid VisuAlg source")
        .expect("scalar declaration should be found");
    assert_eq!(scalar.construct, SourceConstruct::VariableDeclaration);
    assert_eq!(scalar.variables[0].name, "total");
    assert_eq!(scalar.variables[0].type_name, "inteiro");
    assert!(scalar.variables[0].dimensions.is_empty());

    let vector = analyze_source_line(source, line_of(source, "valores:"))
        .expect("valid VisuAlg source")
        .expect("vector declaration should be found");
    assert_eq!(vector.variables[0].dimensions.len(), 1);
    assert_eq!(vector.variables[0].dimensions[0].lower_bound, Some(1));
    assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(3));

    let matrix = analyze_source_line(source, line_of(source, "matriz:"))
        .expect("valid VisuAlg source")
        .expect("matrix declaration should be found");
    assert_eq!(matrix.variables[0].dimensions.len(), 2);

    let assignment = analyze_source_line(source, line_of(source, "total <-"))
        .expect("valid VisuAlg source")
        .expect("assignment should be found");
    assert_eq!(assignment.construct, SourceConstruct::Assignment);

    for (prefix, expected) in [
        ("leia", SourceConstruct::Input),
        ("escreval(total)", SourceConstruct::Output),
        ("se total", SourceConstruct::Decision),
        ("enquanto total", SourceConstruct::Loop),
    ] {
        let context = analyze_source_line(source, line_of(source, prefix))
            .expect("valid VisuAlg source")
            .expect("statement should be found");
        assert_eq!(context.construct, expected, "line starting with {prefix:?}");
    }
}

#[test]
fn reports_no_context_for_an_unclassified_line_and_source_errors_for_incomplete_code() {
    let source = "algoritmo \"analise\"\nvar\n  // comentário\ninicio\nfimalgoritmo\n";
    assert!(
        analyze_source_line(source, 3)
            .expect("comment-only line is valid")
            .is_none()
    );

    let incomplete = "algoritmo \"analise\"\ninicio\n  total <-\nfimalgoritmo\n";
    assert!(analyze_source_line(incomplete, 3).is_err());
}

#[test]
fn recovers_independent_syntax_errors_on_separate_lines() {
    let source =
        "algoritmo \"analise\"\ninicio\n  total : inteiro\n  valor : inteiro\nfimalgoritmo\n";
    let diagnostics =
        analyze_source_line(source, 4).expect_err("the selected line contains a syntax error");
    assert_eq!(
        diagnostics
            .iter()
            .filter(
                |diagnostic| diagnostic.kind == potoo_visualg_interpreter::DiagnosticKind::Syntax
            )
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.line == 3));
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.line == 4));
}
