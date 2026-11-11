use super::profile_adapter;
use crate::languages::model::{
    SourceConstruct, SourceDiagnosticKind, SourceLineAnalysis, SourceLineAnalysisOutcome,
};
use crate::languages::project_templates::{ProjectTemplate, ProjectTemplateGroup};

fn analyze_source_line(
    source: &str,
    line: usize,
) -> (
    Option<SourceLineAnalysis>,
    Vec<crate::languages::model::SourceDiagnostic>,
) {
    match profile_adapter(source, line, &|| false) {
        SourceLineAnalysisOutcome::Complete {
            analysis,
            diagnostics,
        } => (analysis, diagnostics),
        outcome => panic!("Potigol analysis should complete for this fixture: {outcome:?}"),
    }
}

#[test]
fn classic_and_jerimum_are_separate_import_aware_fixtures() {
    let classic = "var pontos := 2\npontos := pontos + 1\nescreva pontos";
    let (assignment, diagnostics) = analyze_source_line(classic, 2);
    assert!(diagnostics.is_empty());
    assert_eq!(
        assignment
            .expect("Classic assignment should be recognized")
            .construct,
        SourceConstruct::Assignment
    );

    let jerimum =
        "use \"jerimum\"\ndef desenhe(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim";
    let (import, diagnostics) = analyze_source_line(jerimum, 1);
    assert!(diagnostics.is_empty());
    assert_eq!(
        import
            .expect("Jerimum import should be recognized")
            .construct,
        SourceConstruct::Import
    );
    let (call, diagnostics) = analyze_source_line(jerimum, 3);
    assert!(diagnostics.is_empty());
    let call = call.expect("typed Jerimum call should be recognized");
    assert_eq!(call.construct, SourceConstruct::MethodCall);
    assert_eq!(
        call.call.as_ref().and_then(|call| call.library.as_deref()),
        Some("jerimum")
    );

    let unimported = "g.setColor(Cor.VERDE.color)";
    let (call, diagnostics) = analyze_source_line(unimported, 1);
    assert!(diagnostics.is_empty());
    let call = call.expect("an ordinary method call should remain recognizable");
    assert_eq!(call.construct, SourceConstruct::MethodCall);
    assert_eq!(
        call.call.as_ref().and_then(|call| call.library.as_deref()),
        None
    );

    for (source, line, expects_missing_import, jerimum_active) in [
        (
            "# use \"jerimum\"\ng.setColor(Cor.VERDE.color)",
            2,
            false,
            false,
        ),
        (
            " use \"jerimum\"\ndef desenhe(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim",
            3,
            false,
            true,
        ),
        (
            "se verdadeiro entao\n  use \"jerimum\"\n  g.setColor(Cor.VERDE.color)\nfim",
            3,
            false,
            false,
        ),
    ] {
        let (call, diagnostics) = analyze_source_line(source, line);
        if expects_missing_import {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::StaticAnalysis);
            assert!(diagnostics[0].message.contains("use \"jerimum\""));
        } else {
            assert!(diagnostics.is_empty());
        }
        let call = call.expect("the method call should remain recognizable");
        assert_eq!(call.construct, SourceConstruct::MethodCall);
        assert_eq!(
            call.call.as_ref().and_then(|call| call.library.as_deref()),
            jerimum_active.then_some("jerimum"),
            "only an active top-level import activates Jerimum"
        );
    }
}

#[test]
fn classic_and_jerimum_recover_a_valid_line_after_the_first_error() {
    for (source, error_line, valid_line) in [
        ("@\nescreva \"depois\"", 1, 2),
        ("use \"jerimum\"\n@\nescreva \"depois\"", 2, 3),
    ] {
        let expected = potoo_potigol_interpreter::parse(source)
            .expect_err("the source contains an invalid token")
            .remove(0);
        let (analysis, diagnostics) = analyze_source_line(source, valid_line);

        assert_eq!(
            analysis
                .expect("the later output remains analyzable")
                .construct,
            SourceConstruct::Output
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::Lexical);
        assert_eq!(diagnostics[0].message, expected.message);
        assert_eq!(
            (diagnostics[0].line, diagnostics[0].column),
            (expected.span.start.line, expected.span.start.column)
        );
        assert_eq!(diagnostics[0].line, error_line);
    }
}

#[test]
fn jerimum_game_project_remains_a_potigol_variant_with_a_valid_imported_sample() {
    let template = ProjectTemplate::PotigolJerimum;
    assert_eq!(
        template.language(),
        crate::languages::model::Language::Potigol
    );
    assert_eq!(template.group(), ProjectTemplateGroup::Potigol);
    assert_eq!(template.order(), 1);
    assert_eq!(template.file_name(), "principal.poti");
    assert_eq!(template.new_file_sample(), template.sample());

    let source = template.sample();
    assert!(source.starts_with("use \"jerimum\"\n"));
    assert!(source.contains("jogo.iniciar("));
    let line = source
        .lines()
        .position(|line| line.contains("g.fillRect("))
        .expect("the game sample should draw its player")
        + 1;
    let (analysis, diagnostics) = analyze_source_line(source, line);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let call = analysis
        .expect("the sample's graphics operation should be recognized")
        .call
        .expect("the graphics operation should retain its call facts");
    assert_eq!(call.member, "fillRect");
    assert_eq!(call.library.as_deref(), Some("jerimum"));
}

#[test]
fn missing_output_argument_keeps_the_same_location_in_learning_and_editor_diagnostics() {
    for (source, error_line, expected_line) in [
        ("escreva\nescreva 2", 1, 1),
        ("use \"jerimum\"\nescreva\nescreva 2", 2, 2),
        ("imprima\nimprima 2", 1, 1),
        ("use \"jerimum\"\nimprima\nimprima 2", 2, 2),
    ] {
        let editor = (crate::languages::model::Language::Potigol
            .profile()
            .diagnostics)(source)
        .into_iter()
        .find(|diagnostic| diagnostic.kind == SourceDiagnosticKind::Syntax)
        .expect("the missing output expression should have a syntax diagnostic");
        let (_, learning_diagnostics) = analyze_source_line(source, error_line);
        let learning = learning_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == SourceDiagnosticKind::Syntax)
            .expect("the learning view should keep the same syntax diagnostic");

        assert_eq!(
            (learning.line, learning.column),
            (editor.line, editor.column),
            "{source}"
        );
        assert_eq!((editor.line, editor.column), (expected_line, 1), "{source}");
        assert_eq!(learning.message, editor.message, "{source}");
    }
}

#[test]
fn hash_comments_require_a_line_ending_and_unclosed_comment_keeps_its_error() {
    let valid = "# comentário\nescreva 1";
    let (analysis, diagnostics) = analyze_source_line(valid, 1);
    assert!(analysis.is_none());
    assert!(diagnostics.is_empty());

    let invalid = "# comentário";
    let expected = potoo_potigol_interpreter::parse(invalid)
        .expect_err("Potigol requires a line ending after a hash comment")
        .remove(0);
    let outcome = profile_adapter(invalid, 1, &|| false);
    let SourceLineAnalysisOutcome::Incomplete { diagnostics } = outcome else {
        panic!("an unterminated comment has no safe range to mask");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(
        (diagnostics[0].line, diagnostics[0].column),
        (expected.span.start.line, expected.span.start.column)
    );

    let malformed = "# comentário\r@\n";
    let expected = potoo_potigol_interpreter::parse(malformed)
        .expect_err("a standalone CR invalidates the comment")
        .remove(0);
    let SourceLineAnalysisOutcome::Incomplete { diagnostics } =
        profile_adapter(malformed, 1, &|| false)
    else {
        panic!("the malformed comment must retain its parser diagnostic");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].kind, SourceDiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(
        (diagnostics[0].line, diagnostics[0].column),
        (expected.span.start.line, expected.span.start.column)
    );
}

#[test]
fn nested_conditionals_keep_their_source_lines_and_block_comments_remain_code() {
    let source = "se verdadeiro entao\n  se falso entao\n    escreva \"dentro\"\n  fim\nfim";
    let (analysis, diagnostics) = analyze_source_line(source, 3);
    assert!(diagnostics.is_empty());
    assert_eq!(
        analysis.expect("nested output should be found").construct,
        SourceConstruct::Output
    );

    let block_comment = "/* não é comentário em Potigol */\nescreva 1";
    assert_eq!(
        crate::languages::potigol::syntax_analysis::views::source_line_code(block_comment, 1)
            .trim(),
        "/* não é comentário em Potigol */"
    );
}

#[test]
fn recovery_discards_cancelled_potigol_work() {
    assert!(matches!(
        profile_adapter("escreva 1", 1, &|| true),
        SourceLineAnalysisOutcome::Cancelled
    ));
}
