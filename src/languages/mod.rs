pub(crate) mod calango;
pub(crate) mod model;
pub(crate) mod portugol;
pub(crate) mod potigol;

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
            potigol::syntax_analysis::views::source_line_code,
        ];

        for view in views {
            let code = view(source, 1);
            assert!(code.contains("http://example.test"), "{code:?}");
            assert!(!code.contains("se fimse"), "{code:?}");
        }
    }

    #[test]
    fn every_language_view_masks_only_the_diagnostic_statement() {
        let source = "before; bad @; after\nsecond line";
        let views = [
            calango::syntax_analysis::views::mask_diagnostic_statement,
            portugol::g_portugol::syntax_analysis::views::mask_diagnostic_statement,
            portugol::mapler::syntax_analysis::views::mask_diagnostic_statement,
            portugol::studio_20::syntax_analysis::views::mask_diagnostic_statement,
            portugol::visualg::syntax_analysis::views::mask_diagnostic_statement,
            potigol::syntax_analysis::views::mask_diagnostic_statement,
        ];

        for view in views {
            let recovered = view(source, 1, 13);
            assert!(recovered.starts_with("before;"), "{recovered:?}");
            assert!(!recovered.contains("bad @"), "{recovered:?}");
            assert!(recovered.contains("after"), "{recovered:?}");
            assert!(recovered.ends_with("second line"), "{recovered:?}");
        }
    }

    #[test]
    fn jerimum_import_is_lexical_and_top_level() {
        use potigol::syntax_analysis::views::is_jerimum_import;

        assert!(is_jerimum_import("use \"jerimum\"\n"));
        assert!(!is_jerimum_import("# use \"jerimum\"\n"));
        assert!(!is_jerimum_import("escreva \"use \\\"jerimum\\\"\"\n"));
        assert!(!is_jerimum_import(
            "se verdadeiro entao\nuse \"jerimum\"\nfim\n"
        ));
    }
}
