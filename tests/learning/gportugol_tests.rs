use super::profile_adapter;
use crate::languages::model::{
    SourceConstruct, SourceDiagnostic, SourceDiagnosticKind, SourceLineAnalysis,
    SourceLineAnalysisOutcome,
};
use potoo_gportugol_interpreter::parse;
use std::cell::Cell;

fn analyze_source_line(
    source: &str,
    line: usize,
) -> Result<Option<SourceLineAnalysis>, Vec<SourceDiagnostic>> {
    match profile_adapter(source, line, &|| false) {
        SourceLineAnalysisOutcome::Complete {
            analysis: _,
            diagnostics,
        } if diagnostics.iter().any(|diagnostic| diagnostic.line == line) => Err(diagnostics),
        SourceLineAnalysisOutcome::Complete {
            analysis,
            diagnostics: _,
        } => Ok(analysis),
        SourceLineAnalysisOutcome::Incomplete { diagnostics } => Err(diagnostics),
        SourceLineAnalysisOutcome::Cancelled => Ok(None),
    }
}

fn line_of(source: &str, prefix: &str) -> usize {
    source
        .lines()
        .position(|line| line.trim_start().starts_with(prefix))
        .expect("the requested source line should exist")
        + 1
}

#[test]
fn analyzes_statements_inside_multiple_statement_control_flow_bodies() {
    let source =
        "algoritmo teste;\ninício\nse verdadeiro então\nimprima(1); imprima(2);\nfim-se\nfim";

    let analysis = analyze_source_line(source, 4)
        .expect("valid G-Portugol source")
        .expect("the selected statement should be analyzed");

    assert_eq!(analysis.construct, SourceConstruct::Output);
}

#[test]
fn recovering_adapter_preserves_accented_byte_spans_and_analyzes_lines_after_errors() {
    let source = "algoritmo teste;\nvariáveis valor: inteiro; fim-variáveis\ninício\n  valor := 1;\n  imprima(\"ação\") @;\n  imprima(\"depois\");\nfim";
    let error_line = line_of(source, "imprima(\"ação\")");
    let expected = parse(source)
        .expect_err("the token after the accented output expression is invalid")
        .remove(0);
    assert_eq!(expected.span.start.line, error_line);
    assert_eq!(expected.span.start.column, 19);
    assert_eq!(expected.span.start.byte_offset, source.find('@').unwrap());

    let SourceLineAnalysisOutcome::Complete {
        analysis,
        diagnostics,
    } = profile_adapter(source, error_line, &|| false)
    else {
        panic!("the single statement error should recover");
    };
    assert!(
        analysis.is_none(),
        "the selected erroneous line has no facts"
    );
    let actual = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.line == error_line)
        .expect("the original line diagnostic should be retained");
    assert_eq!(actual.kind, SourceDiagnosticKind::Lexical);
    assert_eq!(actual.message, expected.message);
    assert_eq!(actual.line, expected.span.start.line);
    assert_eq!(actual.column, expected.span.start.column);
    assert_eq!(
        actual.byte_range,
        Some(expected.span.start.byte_offset..expected.span.end.byte_offset)
    );

    let after = analyze_source_line(source, line_of(source, "imprima(\"depois\")"))
        .expect("the later statement should not inherit the error")
        .expect("the valid statement after the error should be analyzed");
    assert_eq!(after.construct, SourceConstruct::Output);
}

#[test]
fn recovering_adapter_retains_accented_block_and_function_context() {
    let source = "algoritmo teste;\nvariáveis valor: inteiro; fim-variáveis\ninício\n  valor := 1;\n  @;\nfim\nfunção dobrar(valor: inteiro): inteiro\ninício\n  retorne valor * 2;\nfim";
    let function = analyze_source_line(source, line_of(source, "função dobrar"))
        .expect("the later routine should be analyzed after recovery")
        .expect("the routine signature should be present in the recovered AST");

    assert_eq!(function.construct, SourceConstruct::Function);
    assert_eq!(function.name.as_deref(), Some("dobrar"));
    assert_eq!(function.return_type.as_deref(), Some("inteiro"));
    assert_eq!(function.parameters[0].name, "valor");
}

#[test]
fn recovering_adapter_handles_bare_carriage_return_lines() {
    let source = "algoritmo teste;\rinício\r  @;\r  imprima(\"ação\");\rfim";
    let original = parse(source)
        .expect_err("the standalone invalid token should be diagnosed")
        .remove(0);
    assert_eq!(original.span.start.line, 3);
    assert_eq!(original.span.start.column, 3);

    let SourceLineAnalysisOutcome::Complete {
        analysis,
        diagnostics,
    } = profile_adapter(source, 4, &|| false)
    else {
        panic!("the later valid line should be analyzed after the error");
    };
    assert_eq!(
        analysis
            .expect("the later output should be analyzed")
            .construct,
        SourceConstruct::Output
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].line, original.span.start.line);
    assert_eq!(diagnostics[0].column, original.span.start.column);
    assert_eq!(
        diagnostics[0].byte_range,
        Some(original.span.start.byte_offset..original.span.end.byte_offset)
    );
}

#[test]
fn recovering_adapter_ignores_a_leading_bom_when_mapping_first_line_columns() {
    let source = "\u{feff}algoritmo teste; início imprima(1); @; imprima(2);\nfim";
    let original = parse(source)
        .expect_err("the invalid token after the first output should be diagnosed")
        .remove(0);
    assert_eq!(original.span.start.line, 1);

    let SourceLineAnalysisOutcome::Complete {
        analysis,
        diagnostics,
    } = profile_adapter(source, 2, &|| false)
    else {
        panic!("the first-line statement error should recover without shifting columns");
    };
    assert_eq!(
        analysis
            .expect("the following line should retain its parsed program block")
            .construct,
        SourceConstruct::Program
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].line, original.span.start.line);
    assert_eq!(diagnostics[0].column, original.span.start.column);
    assert_eq!(
        diagnostics[0].byte_range,
        Some(original.span.start.byte_offset..original.span.end.byte_offset)
    );
}

#[test]
fn recovering_adapter_handles_a_multiline_character_literal_before_an_error() {
    let source = "algoritmo teste;\ninício\nimprima('\n');\n@;\nimprima(1);\nfim";
    let original = parse(source)
        .expect_err("the standalone invalid token should be diagnosed")
        .remove(0);
    assert_eq!(original.span.start.line, 5);

    let SourceLineAnalysisOutcome::Complete { diagnostics, .. } =
        profile_adapter(source, 5, &|| false)
    else {
        panic!("a raw newline in a character literal should not hide later diagnostics");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].line, original.span.start.line);
    assert_eq!(diagnostics[0].column, original.span.start.column);
    assert_eq!(
        diagnostics[0].byte_range,
        Some(original.span.start.byte_offset..original.span.end.byte_offset)
    );
}

#[test]
fn recovery_fails_closed_when_a_four_byte_character_cannot_be_masked_safely() {
    let source = "algoritmo teste; início\nimprima(\"😀\");\nfim";
    let expected = parse(source)
        .expect_err("G-Portugol rejects an emoji in its literal range")
        .remove(0);

    let SourceLineAnalysisOutcome::Incomplete { diagnostics } =
        profile_adapter(source, expected.span.start.line, &|| false)
    else {
        panic!("recovery must stop rather than shift a four-byte source span");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.span.start.line);
    assert_eq!(diagnostics[0].column, expected.span.start.column);
    assert_eq!(
        diagnostics[0].byte_range,
        Some(expected.span.start.byte_offset..expected.span.end.byte_offset)
    );
}

#[test]
fn recovering_adapter_preserves_multiple_original_diagnostics_and_cancels_between_retries() {
    let source = "algoritmo erros; início\n @;\n imprima(1);\n @;\nfim";
    let expected = parse(source).expect_err("two invalid statements should be reported");
    let SourceLineAnalysisOutcome::Complete { diagnostics, .. } =
        profile_adapter(source, 4, &|| false)
    else {
        panic!("independent invalid statements should be recoverable");
    };
    assert_eq!(diagnostics.len(), 2);
    for (actual, expected) in diagnostics.iter().zip(expected) {
        assert_eq!(actual.kind, SourceDiagnosticKind::Lexical);
        assert_eq!(actual.message, expected.message);
        assert_eq!(actual.line, expected.span.start.line);
        assert_eq!(actual.column, expected.span.start.column);
        assert_eq!(
            actual.byte_range,
            Some(expected.span.start.byte_offset..expected.span.end.byte_offset)
        );
    }

    let checks = Cell::new(0);
    let cancelled = profile_adapter(source, 4, &|| {
        let next = checks.get() + 1;
        checks.set(next);
        next >= 3
    });
    assert_eq!(cancelled, SourceLineAnalysisOutcome::Cancelled);
}
