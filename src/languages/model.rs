use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    Portugol,
    Visualg,
    Potigol,
    GPortugol,
    Calango,
    Mapler,
}

impl Language {
    pub(crate) const ALL: [Self; 6] = [
        Self::Portugol,
        Self::Visualg,
        Self::Potigol,
        Self::GPortugol,
        Self::Calango,
        Self::Mapler,
    ];

    pub(crate) fn profile(self) -> &'static crate::languages::profile::LanguageProfile {
        crate::languages::profile::for_language(self)
    }

    pub(crate) fn label(self) -> &'static str {
        self.profile().display_name
    }

    pub(crate) fn key(self) -> &'static str {
        self.profile().key
    }

    pub(crate) fn from_key(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|language| language.key() == key)
            .unwrap_or(Self::Portugol)
    }

    pub(crate) fn file_name(self) -> &'static str {
        self.profile().file_name
    }

    pub(crate) fn sample(self) -> &'static str {
        self.profile().sample
    }

    pub(crate) fn extension(self) -> &'static str {
        self.profile().extension
    }

    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|language| language.extension() == extension)
    }

    pub(crate) fn keywords(self) -> &'static [&'static str] {
        self.profile().keywords
    }
}

pub(crate) use crate::languages::project_templates::ProjectTemplate;

#[cfg(test)]
mod tests {
    use super::Language;
    use std::path::Path;

    #[test]
    fn from_path_recognizes_gpt_without_changing_existing_languages() {
        for (path, expected) in [
            ("main.gpt", Language::GPortugol),
            ("MAIN.GPT", Language::GPortugol),
            ("main.por", Language::Portugol),
            ("main.alg", Language::Visualg),
            ("main.poti", Language::Potigol),
            ("main.mapler", Language::Mapler),
            ("MAIN.MAPLER", Language::Mapler),
        ] {
            assert_eq!(
                Language::from_path(Path::new(path)),
                Some(expected),
                "{path}"
            );
        }
        assert_eq!(
            Language::from_path(Path::new("main.clg")),
            Some(Language::Calango)
        );
        assert_eq!(Language::from_path(Path::new("main.txt")), None);
    }

    #[test]
    fn gportugol_starter_uses_gpt_extension() {
        let language = Language::GPortugol;
        assert_eq!(language.file_name(), "principal.gpt");
        assert_eq!(language.extension(), "gpt");
        assert!(language.sample().contains("algoritmo contagem;"));
        assert_eq!(
            Language::from_path(Path::new(language.file_name())),
            Some(language)
        );
        potoo_gportugol_interpreter::parse(language.sample())
            .expect("the starter program should be valid G-Portugol");
    }

    #[test]
    fn mapler_starter_uses_potoo_extension_and_official_syntax() {
        let language = Language::Mapler;
        assert_eq!(language.file_name(), "principal.mapler");
        assert_eq!(language.extension(), "mapler");
        assert_eq!(Language::from_path(Path::new("principal.txt")), None);
        let program = potoo_mapler_interpreter::parse(language.sample())
            .expect("the Mapler starter should parse");
        potoo_mapler_interpreter::check(&program)
            .expect("the Mapler starter should pass static checking");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceConstruct {
    Program,
    Function,
    Procedure,
    VariableDeclaration,
    Assignment,
    Call,
    MethodCall,
    Import,
    Input,
    Output,
    Decision,
    Loop,
    Return,
    Unrecognized,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceCall {
    pub(crate) receiver: Option<String>,
    pub(crate) member: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) library: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceDimension {
    pub(crate) lower_bound: Option<i64>,
    pub(crate) upper_bound: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceSymbol {
    pub(crate) name: String,
    pub(crate) type_name: String,
    pub(crate) is_constant: bool,
    pub(crate) by_reference: bool,
    pub(crate) dimensions: Vec<SourceDimension>,
    pub(crate) initializer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceLineAnalysis {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) construct: SourceConstruct,
    pub(crate) name: Option<String>,
    pub(crate) return_type: Option<String>,
    pub(crate) parameters: Vec<SourceSymbol>,
    pub(crate) variables: Vec<SourceSymbol>,
    pub(crate) call: Option<SourceCall>,
}
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceDiagnosticKind {
    Lexical,
    Syntax,
    Type,
    Runtime,
    StaticAnalysis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceDiagnostic {
    pub(crate) kind: SourceDiagnosticKind,
    pub(crate) message: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) byte_range: Option<Range<usize>>,
}

impl SourceDiagnostic {
    pub(crate) fn at_line(
        kind: SourceDiagnosticKind,
        message: String,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            kind,
            message,
            line,
            column,
            byte_range: None,
        }
    }
}

#[cfg(test)]
mod language_profile_tests {
    use super::{Language, ProjectTemplate};

    #[test]
    fn static_profiles_cover_current_languages_and_preserve_metadata() {
        let expected = [
            (
                Language::Portugol,
                "portugol",
                "Portugol Studio 2.0",
                "principal.por",
                "por",
                crate::languages::portugol::studio_20::examples::SAMPLE,
                crate::languages::portugol::studio_20::completion::LANGUAGE_KEYWORDS,
                8,
                "programa",
                ProjectTemplate::GPortugol,
            ),
            (
                Language::Visualg,
                "visualg",
                "VisuAlg",
                "principal.alg",
                "alg",
                crate::languages::portugol::visualg::examples::SAMPLE,
                crate::languages::portugol::visualg::completion::LANGUAGE_KEYWORDS,
                12,
                "algoritmo",
                ProjectTemplate::Visualg,
            ),
            (
                Language::Potigol,
                "potigol",
                "Potigol",
                "principal.poti",
                "poti",
                crate::languages::potigol::classic::examples::SAMPLE,
                crate::languages::potigol::completion::LANGUAGE_KEYWORDS,
                34,
                "escreva",
                ProjectTemplate::PotigolClassic,
            ),
            (
                Language::GPortugol,
                "gportugol",
                "G-Portugol",
                "principal.gpt",
                "gpt",
                crate::languages::portugol::g_portugol::examples::SAMPLE,
                crate::languages::portugol::g_portugol::completion::LANGUAGE_KEYWORDS,
                28,
                "algoritmo",
                ProjectTemplate::GPortugol,
            ),
            (
                Language::Calango,
                "calango",
                "Calango",
                "principal.clg",
                "clg",
                crate::languages::calango::examples::SAMPLE,
                crate::languages::calango::completion::LANGUAGE_KEYWORDS,
                38,
                "fimPrincipal",
                ProjectTemplate::Calango,
            ),
            (
                Language::Mapler,
                "mapler",
                "Mapler",
                "principal.mapler",
                "mapler",
                crate::languages::portugol::mapler::examples::SAMPLE,
                crate::languages::portugol::mapler::completion::LANGUAGE_KEYWORDS,
                27,
                "variaveis",
                ProjectTemplate::Mapler,
            ),
        ];

        assert_eq!(Language::ALL, expected.map(|entry| entry.0));
        for (
            language,
            key,
            name,
            file_name,
            extension,
            sample,
            language_keywords,
            keyword_count,
            keyword,
            template,
        ) in expected
        {
            let profile = language.profile();
            assert_eq!(profile.language, language);
            assert_eq!(profile.key, key);
            assert_eq!(profile.display_name, name);
            assert_eq!(profile.file_name, file_name);
            assert_eq!(profile.extension, extension);
            assert_eq!(profile.sample, sample);
            assert_eq!(profile.keywords, language_keywords, "{key}");
            assert_eq!(profile.keywords.len(), keyword_count, "{key}");
            assert!(profile.keywords.contains(&keyword), "{key}: {keyword}");
            assert_eq!(profile.default_template, template, "{key}");
            assert_eq!(
                ProjectTemplate::default_for_language(language),
                template,
                "{key}"
            );
            assert_eq!(Language::from_key(key), language);
        }
        assert_eq!(Language::from_key("unknown"), Language::Portugol);
    }

    #[test]
    fn static_profiles_have_unique_keys_and_extensions() {
        for (index, language) in Language::ALL.iter().enumerate() {
            let profile = language.profile();
            for other in Language::ALL.iter().skip(index + 1) {
                assert_ne!(profile.key, other.profile().key);
                assert_ne!(profile.extension, other.profile().extension);
            }
        }
    }
}

#[cfg(test)]
mod project_template_catalog_tests {
    use crate::languages::project_templates::{ProjectTemplate, ProjectTemplateGroup};

    #[test]
    fn template_catalog_preserves_language_group_order_and_examples() {
        let expected = [
            (
                ProjectTemplate::GPortugol,
                super::Language::GPortugol,
                ProjectTemplateGroup::Portugol,
                0,
                crate::languages::portugol::g_portugol::examples::SAMPLE,
                "",
            ),
            (
                ProjectTemplate::Visualg,
                super::Language::Visualg,
                ProjectTemplateGroup::Portugol,
                1,
                crate::languages::portugol::visualg::examples::SAMPLE,
                "",
            ),
            (
                ProjectTemplate::Portugol20,
                super::Language::Portugol,
                ProjectTemplateGroup::Portugol,
                2,
                crate::languages::portugol::studio_20::examples::SAMPLE,
                "",
            ),
            (
                ProjectTemplate::Mapler,
                super::Language::Mapler,
                ProjectTemplateGroup::Portugol,
                3,
                crate::languages::portugol::mapler::examples::SAMPLE,
                "",
            ),
            (
                ProjectTemplate::PotigolClassic,
                super::Language::Potigol,
                ProjectTemplateGroup::Potigol,
                0,
                crate::languages::potigol::classic::examples::SAMPLE,
                "",
            ),
            (
                ProjectTemplate::PotigolJerimum,
                super::Language::Potigol,
                ProjectTemplateGroup::Potigol,
                1,
                crate::languages::potigol::jerimum::examples::SAMPLE,
                crate::languages::potigol::jerimum::examples::SAMPLE,
            ),
            (
                ProjectTemplate::Calango,
                super::Language::Calango,
                ProjectTemplateGroup::Calango,
                0,
                crate::languages::calango::examples::SAMPLE,
                "",
            ),
        ];

        assert_eq!(ProjectTemplate::ALL, expected.map(|entry| entry.0));
        for (template, language, group, order, sample, new_file_sample) in expected {
            let profile = template.profile();
            assert_eq!(profile.template, template);
            assert_eq!(profile.language, language);
            assert_eq!(profile.group, group);
            assert_eq!(profile.order, order);
            assert_eq!(profile.sample, sample);
            assert_eq!(profile.new_file_sample, new_file_sample);
            assert_eq!(template.file_name(), language.file_name());
            assert_eq!(template.sample(), sample);
            assert_eq!(template.new_file_sample(), new_file_sample);
        }
    }

    #[test]
    fn template_group_maps_to_the_existing_variant_selectors() {
        for (language, group) in [
            (super::Language::Portugol, ProjectTemplateGroup::Portugol),
            (super::Language::GPortugol, ProjectTemplateGroup::Portugol),
            (super::Language::Visualg, ProjectTemplateGroup::Portugol),
            (super::Language::Mapler, ProjectTemplateGroup::Portugol),
            (super::Language::Potigol, ProjectTemplateGroup::Potigol),
            (super::Language::Calango, ProjectTemplateGroup::Calango),
        ] {
            assert_eq!(ProjectTemplateGroup::for_language(language), group);
        }

        for (group, order, expected) in [
            (
                ProjectTemplateGroup::Portugol,
                0,
                Some(ProjectTemplate::GPortugol),
            ),
            (
                ProjectTemplateGroup::Portugol,
                1,
                Some(ProjectTemplate::Visualg),
            ),
            (
                ProjectTemplateGroup::Portugol,
                2,
                Some(ProjectTemplate::Portugol20),
            ),
            (
                ProjectTemplateGroup::Portugol,
                3,
                Some(ProjectTemplate::Mapler),
            ),
            (
                ProjectTemplateGroup::Potigol,
                0,
                Some(ProjectTemplate::PotigolClassic),
            ),
            (
                ProjectTemplateGroup::Potigol,
                1,
                Some(ProjectTemplate::PotigolJerimum),
            ),
            (ProjectTemplateGroup::Calango, 0, None),
            (ProjectTemplateGroup::Portugol, 4, None),
        ] {
            assert_eq!(ProjectTemplate::for_group_order(group, order), expected);
        }
    }
}
