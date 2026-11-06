pub(crate) mod calango;
pub(crate) mod completion;
pub(crate) mod model;
pub(crate) mod portugol;
pub(crate) mod potigol;
pub(crate) mod profile;
pub(crate) mod project_templates;
// The shared engine is introduced before profiles migrate to it in later stages.
#[allow(dead_code)]
pub(crate) mod source;

#[cfg(test)]
mod syntax_analysis_tests {
    use super::{calango, portugol, potigol};

    #[test]
    fn every_language_view_masks_comments_without_truncating_strings() {
        let source = r#"escreva("http://example.test") // se fimse"#;
        let views = [
            calango::syntax_analysis::views::source_line_code,
            portugol::g_portugol::syntax_analysis::views::source_line_code,
            portugol::mapler::syntax_analysis::views::source_line_code,
            portugol::studio_20::syntax_analysis::views::source_line_code,
            portugol::visualg::syntax_analysis::views::source_line_code,
        ];

        for view in views {
            let code = view(source, 1);
            assert!(code.contains("http://example.test"), "{code:?}");
            assert!(!code.contains("se fimse"), "{code:?}");
        }

        let potigol = potigol::syntax_analysis::views::source_line_code(source, 1);
        assert!(potigol.contains("se fimse"), "// is not a Potigol comment");
    }

    #[test]
    fn every_language_view_masks_only_the_diagnostic_statement() {
        let source = "before\nbad @\nafter\nsecond line";
        let views = [potigol::syntax_analysis::views::mask_diagnostic_statement];

        for view in views {
            let recovered = view(source, 2, 5).expect("the statement boundary should be safe");
            assert!(recovered.starts_with("before\n"), "{recovered:?}");
            assert!(!recovered.contains("bad @"), "{recovered:?}");
            assert!(recovered.contains("after"), "{recovered:?}");
            assert!(recovered.ends_with("second line"), "{recovered:?}");
        }

        let calango_source =
            "algoritmo teste;\nprincipal\nescreva(1);\n@;\nescreva(2);\nfimPrincipal";
        let diagnostic = potoo_calango_interpreter::parse(calango_source)
            .expect_err("the selected statement is invalid")
            .remove(0);
        let recovered = calango::syntax_analysis::views::mask_diagnostic_statement(
            calango_source,
            diagnostic.line,
            diagnostic.column,
        )
        .expect("the semicolon statement boundary should be safe");
        assert!(recovered.contains("escreva(1);"), "{recovered:?}");
        assert!(!recovered.contains('@'), "{recovered:?}");
        assert!(recovered.contains("escreva(2);"), "{recovered:?}");
        assert!(potoo_calango_interpreter::parse(&recovered).is_ok());

        let visualg_source = "algoritmo \"teste\"\ninicio\nescreva(1)\n@\nescreva(2)\nfimalgoritmo";
        let recovered = portugol::visualg::syntax_analysis::views::mask_diagnostic_statement(
            visualg_source,
            4,
            1,
        )
        .expect("the VisuAlg statement boundary should be safe");
        assert!(recovered.contains("escreva(1)"), "{recovered:?}");
        assert!(!recovered.contains('@'), "{recovered:?}");
        assert!(recovered.contains("escreva(2)"), "{recovered:?}");
        assert!(potoo_visualg_interpreter::parse(&recovered).is_ok());

        let gportugol_source = "before; bad @; after\nsecond line";
        let recovered = portugol::g_portugol::syntax_analysis::views::mask_diagnostic_statement(
            gportugol_source,
            1,
            13,
        )
        .expect("the G-Portugol statement boundary should be safe");
        assert!(recovered.starts_with("before;"), "{recovered:?}");
        assert!(!recovered.contains("bad @"), "{recovered:?}");
        assert!(recovered.contains("after"), "{recovered:?}");
        assert!(recovered.ends_with("second line"), "{recovered:?}");

        let recovered = portugol::mapler::syntax_analysis::views::mask_diagnostic_statement(
            gportugol_source,
            1,
            13,
        )
        .expect("the Mapler statement boundary should be safe");
        assert!(recovered.starts_with("before;"), "{recovered:?}");
        assert!(!recovered.contains("bad @"), "{recovered:?}");
        assert!(recovered.contains("after"), "{recovered:?}");
        assert!(recovered.ends_with("second line"), "{recovered:?}");

        let portugol_source = "programa {\n funcao inicio() {\n  escreva(\"before\")\n  @\n  escreva(\"after\")\n }\n}";
        let recovered = portugol::studio_20::syntax_analysis::views::mask_diagnostic_statement(
            portugol_source,
            4,
            3,
        )
        .expect("the Portugol statement boundary should be safe");
        assert!(recovered.contains("escreva(\"before\")"), "{recovered:?}");
        assert!(!recovered.contains("@"), "{recovered:?}");
        assert!(recovered.contains("escreva(\"after\")"), "{recovered:?}");
        assert!(potoo_portugol_interpreter::parse(&recovered).is_ok());
    }

    #[test]
    fn language_views_mask_literals_and_each_language_comment_syntax() {
        let source = "valores[2] /* valores[8] */ // valores[9] \"valores[10]\"\n";
        let views = [
            calango::syntax_analysis::views::without_literals_and_comments,
            portugol::g_portugol::syntax_analysis::views::without_literals_and_comments,
            portugol::studio_20::syntax_analysis::views::without_literals_and_comments,
        ];

        for view in views {
            let code = view(source);
            assert!(code.contains("valores[2]"), "{code:?}");
            assert!(!code.contains("valores[8]"), "{code:?}");
            assert!(!code.contains("valores[9]"), "{code:?}");
            assert!(!code.contains("valores[10]"), "{code:?}");
        }

        let mapler = portugol::mapler::syntax_analysis::views::without_literals_and_comments(
            "valores[2] // valores[9] \"valores[10]\"\n",
        );
        assert!(mapler.contains("valores[2]"), "{mapler:?}");
        assert!(!mapler.contains("valores[9]"), "{mapler:?}");
        assert!(!mapler.contains("valores[10]"), "{mapler:?}");

        let potigol = potigol::syntax_analysis::views::without_literals_and_comments(
            "valores[2] # valores[8]\n/* valores[9] */ \"valores[10]\"\n",
        );
        assert!(potigol.contains("valores[2]"), "{potigol:?}");
        assert!(!potigol.contains("valores[8]"), "{potigol:?}");
        assert!(potigol.contains("valores[9]"), "{potigol:?}");
        assert!(!potigol.contains("valores[10]"), "{potigol:?}");
    }
}
