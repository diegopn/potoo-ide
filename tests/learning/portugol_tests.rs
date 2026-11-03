use super::super::SourceConstruct;
use super::analyze_source_line_cancellable;

fn analyze_source_line(
    source: &str,
    line: usize,
) -> Result<Option<super::super::SourceLineAnalysis>, Vec<potoo_portugol_interpreter::Diagnostic>> {
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
fn analyzes_portugol_functions_parameters_and_return_types() {
    let source = r#"programa {
  funcao vazio ajustar(inteiro &valor, cadeia nome) {
    valor = valor + 1
  }
  funcao cadeia identificar(inteiro valor) {
    retorne "inteiro"
  }
  funcao inicio() {}
}"#;

    let void_function = analyze_source_line(source, line_of(source, "funcao vazio"))
        .expect("valid Portugol source")
        .expect("function declaration should be found");
    assert_eq!(void_function.construct, SourceConstruct::Function);
    assert_eq!(void_function.name.as_deref(), Some("ajustar"));
    assert_eq!(void_function.return_type.as_deref(), Some("vazio"));
    assert_eq!(void_function.parameters.len(), 2);
    assert_eq!(void_function.parameters[0].name, "valor");
    assert_eq!(void_function.parameters[0].type_name, "inteiro");
    assert!(void_function.parameters[0].by_reference);
    assert_eq!(void_function.parameters[1].type_name, "cadeia");

    let text_function = analyze_source_line(source, line_of(source, "funcao cadeia"))
        .expect("valid Portugol source")
        .expect("function declaration should be found");
    assert_eq!(text_function.construct, SourceConstruct::Function);
    assert_eq!(text_function.name.as_deref(), Some("identificar"));
    assert_eq!(text_function.return_type.as_deref(), Some("cadeia"));

    let return_statement = analyze_source_line(source, line_of(source, "retorne"))
        .expect("valid Portugol source")
        .expect("return statement should be found");
    assert_eq!(return_statement.construct, SourceConstruct::Return);
    assert_eq!(return_statement.return_type.as_deref(), Some("cadeia"));
}

#[test]
fn analyzes_portugol_scalar_vector_matrix_and_statement_kinds() {
    let source = r#"programa {
  funcao vazio inicio() {
    inteiro total = 1
    inteiro valores[3] = {1, 2, 3}
    inteiro matriz[2][3]
    total = total + valores[0]
    leia(total)
    escreva(total)
    se (total > 0) {}
    enquanto (total > 0) {}
  }
}"#;

    let scalar = analyze_source_line(source, line_of(source, "inteiro total"))
        .expect("valid Portugol source")
        .expect("variable declaration should be found");
    assert_eq!(scalar.construct, SourceConstruct::VariableDeclaration);
    assert_eq!(scalar.variables[0].name, "total");
    assert_eq!(scalar.variables[0].type_name, "inteiro");
    assert!(scalar.variables[0].dimensions.is_empty());
    assert_eq!(scalar.variables[0].initializer.as_deref(), Some("1"));

    let vector = analyze_source_line(source, line_of(source, "inteiro valores"))
        .expect("valid Portugol source")
        .expect("vector declaration should be found");
    assert_eq!(vector.variables[0].dimensions.len(), 1);
    assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(2));

    let matrix = analyze_source_line(source, line_of(source, "inteiro matriz"))
        .expect("valid Portugol source")
        .expect("matrix declaration should be found");
    assert_eq!(matrix.variables[0].dimensions.len(), 2);

    for (prefix, expected) in [
        ("total =", SourceConstruct::Assignment),
        ("leia", SourceConstruct::Input),
        ("escreva", SourceConstruct::Output),
        ("se ", SourceConstruct::Decision),
        ("enquanto", SourceConstruct::Loop),
    ] {
        let context = analyze_source_line(source, line_of(source, prefix))
            .expect("valid Portugol source")
            .expect("statement should be found");
        assert_eq!(context.construct, expected, "line starting with {prefix:?}");
    }
}

#[test]
fn reports_no_context_for_an_unclassified_line_and_source_errors_for_incomplete_code() {
    let source = "programa {\n  funcao inicio() {\n    // comentário\n  }\n}";
    assert!(
        analyze_source_line(source, 3)
            .expect("comment-only line is valid")
            .is_none()
    );

    let incomplete = "programa {\n  funcao inicio() {\n    inteiro valor =\n  }\n}";
    assert!(analyze_source_line(incomplete, 3).is_err());
}

#[test]
fn recovers_independent_syntax_errors_on_separate_lines() {
    let source = "programa {\n funcao inicio() {\n  inteiro =\n  real =\n }\n}";
    let diagnostics =
        analyze_source_line(source, 4).expect_err("the selected line contains a syntax error");
    assert_eq!(
        diagnostics
            .iter()
            .filter(
                |diagnostic| diagnostic.kind == potoo_portugol_interpreter::DiagnosticKind::Syntax
            )
            .count(),
        2,
        "recovery diagnostics: {diagnostics:?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.line == 3));
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.line == 4));
}
