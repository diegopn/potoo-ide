use std::path::{Path, PathBuf};

const PORTUGOL_SAMPLE: &str = "programa {\n  funcao inicio() {\n    inteiro idade\n\n    escreva(\"Qual é sua idade? \")\n    leia(idade)\n    se (idade >= 18) {\n      escreva(\"Pode votar.\")\n    } senao {\n      escreva(\"Ainda não pode votar.\")\n    }\n  }\n}";

const VISUALG_SAMPLE: &str = "algoritmo \"votacao\"\nvar\n  idade: inteiro\ninicio\n  escreva(\"Qual é sua idade? \")\n  leia(idade)\n  se idade >= 18 entao\n    escreval(\"Pode votar.\")\n  senao\n    escreval(\"Ainda não pode votar.\")\n  fimse\nfimalgoritmo";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    Portugol,
    Visualg,
}

impl Language {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Portugol => "Portugol",
            Self::Visualg => "VisuAlg",
        }
    }

    pub(crate) fn selected_index(self) -> u32 {
        match self {
            Self::Portugol => 0,
            Self::Visualg => 1,
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Portugol => "portugol",
            Self::Visualg => "visualg",
        }
    }

    pub(crate) fn from_key(key: &str) -> Self {
        if key == "visualg" {
            Self::Visualg
        } else {
            Self::Portugol
        }
    }

    pub(crate) fn file_name(self) -> &'static str {
        match self {
            Self::Portugol => "principal.por",
            Self::Visualg => "principal.alg",
        }
    }

    pub(crate) fn sample(self) -> &'static str {
        match self {
            Self::Portugol => PORTUGOL_SAMPLE,
            Self::Visualg => VISUALG_SAMPLE,
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Portugol => "por",
            Self::Visualg => "alg",
        }
    }

    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "por" => Some(Self::Portugol),
            "alg" => Some(Self::Visualg),
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
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceDocument {
    pub(crate) id: u64,
    pub(crate) path: Option<PathBuf>,
    pub(crate) name: String,
    pub(crate) language: Language,
    pub(crate) contents: String,
    pub(crate) saved_contents: String,
}

impl SourceDocument {
    pub(crate) fn is_dirty(&self) -> bool {
        self.contents != self.saved_contents
    }
}
