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
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Portugol => "Portugol Studio 2.0",
            Self::Visualg => "VisuAlg",
            Self::Potigol => "Potigol",
            Self::GPortugol => "G-Portugol",
            Self::Calango => "Calango",
            Self::Mapler => "Mapler",
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Portugol => "portugol",
            Self::Visualg => "visualg",
            Self::Potigol => "potigol",
            Self::GPortugol => "gportugol",
            Self::Calango => "calango",
            Self::Mapler => "mapler",
        }
    }

    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "visualg" => Self::Visualg,
            "potigol" => Self::Potigol,
            "gportugol" => Self::GPortugol,
            "calango" => Self::Calango,
            "mapler" => Self::Mapler,
            _ => Self::Portugol,
        }
    }

    pub(crate) fn file_name(self) -> &'static str {
        match self {
            Self::Portugol => "principal.por",
            Self::Visualg => "principal.alg",
            Self::Potigol => "principal.poti",
            Self::GPortugol => "principal.gpt",
            Self::Calango => "principal.clg",
            Self::Mapler => "principal.mapler",
        }
    }

    pub(crate) fn sample(self) -> &'static str {
        match self {
            Self::Portugol => crate::languages::portugol::studio_20::examples::SAMPLE,
            Self::Visualg => crate::languages::portugol::visualg::examples::SAMPLE,
            Self::Potigol => crate::languages::potigol::classic::examples::SAMPLE,
            Self::GPortugol => crate::languages::portugol::g_portugol::examples::SAMPLE,
            Self::Calango => crate::languages::calango::examples::SAMPLE,
            Self::Mapler => crate::languages::portugol::mapler::examples::SAMPLE,
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Portugol => "por",
            Self::Visualg => "alg",
            Self::Potigol => "poti",
            Self::GPortugol => "gpt",
            Self::Calango => "clg",
            // The original Mapler IDE saves `.txt`; `.mapler` is Potoo's extension.
            Self::Mapler => "mapler",
        }
    }

    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "por" => Some(Self::Portugol),
            "alg" => Some(Self::Visualg),
            "poti" => Some(Self::Potigol),
            "gpt" => Some(Self::GPortugol),
            "clg" => Some(Self::Calango),
            "mapler" => Some(Self::Mapler),
            _ => None,
        }
    }

    pub(crate) fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Portugol => &[
                "programa", "funcao", "inicio", "inteiro", "escreva", "leia", "se", "senao",
            ],
            Self::Visualg => &[
                "algoritmo",
                "var",
                "inteiro",
                "inicio",
                "escreva",
                "escreval",
                "leia",
                "se",
                "entao",
                "senao",
                "fimse",
                "fimalgoritmo",
            ],
            Self::Potigol => &[
                "def",
                "var",
                "tipo",
                "escreva",
                "imprima",
                "retorne",
                "enquanto",
                "para",
                "faça",
                "faca",
                "de",
                "até",
                "ate",
                "passo",
                "em",
                "se",
                "então",
                "entao",
                "senãose",
                "senaose",
                "senão",
                "senao",
                "escolha",
                "caso",
                "gere",
                "fim",
                "verdadeiro",
                "falso",
                "e",
                "ou",
                "não",
                "nao",
                "div",
                "mod",
            ],
            Self::GPortugol => &[
                "algoritmo",
                "variáveis",
                "fim-variáveis",
                "início",
                "fim",
                "função",
                "retorne",
                "se",
                "então",
                "senão",
                "fim-se",
                "enquanto",
                "faça",
                "fim-enquanto",
                "repita",
                "até",
                "para",
                "de",
                "passo",
                "imprima",
                "leia",
                "inteiro",
                "reais",
                "caractere",
                "literais",
                "lógico",
                "verdadeiro",
                "falso",
            ],
            Self::Calango => &[
                "algoritmo",
                "principal",
                "fimPrincipal",
                "inteiro",
                "real",
                "logico",
                "caracter",
                "texto",
                "funcao",
                "procedimento",
                "retorna",
                "se",
                "entao",
                "senao",
                "senaoSe",
                "fimSe",
                "escolha",
                "caso",
                "outroCaso",
                "fimEscolha",
                "para",
                "de",
                "ate",
                "passo",
                "faca",
                "fimPara",
                "enquanto",
                "fimEnquanto",
                "interrompa",
                "escreva",
                "escreval",
                "leia",
                "leiaCaracter",
                "verdadeiro",
                "falso",
                "e",
                "ou",
                "nao",
            ],
            Self::Mapler => &[
                "variaveis",
                "inicio",
                "fim",
                "inteiro",
                "real",
                "logico",
                "cadeia",
                "caractere",
                "vetor",
                "de",
                "modulo",
                "escrever",
                "ler",
                "se",
                "entao",
                "senao",
                "enquanto",
                "para",
                "ate",
                "passo",
                "faca",
                "repita",
                "verdadeiro",
                "falso",
                "e",
                "ou",
                "nao",
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectTemplate {
    Portugol20,
    GPortugol,
    Visualg,
    PotigolClassic,
    PotigolJerimum,
    Calango,
    Mapler,
}

impl ProjectTemplate {
    pub(crate) fn default_for_language(language: Language) -> Self {
        match language {
            Language::Portugol => Self::GPortugol,
            Language::Visualg => Self::Visualg,
            Language::Potigol => Self::PotigolClassic,
            Language::GPortugol => Self::GPortugol,
            Language::Calango => Self::Calango,
            Language::Mapler => Self::Mapler,
        }
    }

    pub(crate) fn language(self) -> Language {
        match self {
            Self::Portugol20 => Language::Portugol,
            Self::GPortugol => Language::GPortugol,
            Self::Visualg => Language::Visualg,
            Self::PotigolClassic | Self::PotigolJerimum => Language::Potigol,
            Self::Calango => Language::Calango,
            Self::Mapler => Language::Mapler,
        }
    }

    pub(crate) fn file_name(self) -> &'static str {
        self.language().file_name()
    }

    pub(crate) fn sample(self) -> &'static str {
        match self {
            Self::PotigolJerimum => crate::languages::potigol::jerimum::examples::SAMPLE,
            _ => self.language().sample(),
        }
    }

    pub(crate) fn new_file_sample(self) -> &'static str {
        match self {
            Self::PotigolJerimum => crate::languages::potigol::jerimum::examples::SAMPLE,
            _ => "",
        }
    }
}

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
