use std::path::{Path, PathBuf};

const PORTUGOL_SAMPLE: &str = "programa {\n  funcao inicio() {\n    inteiro idade\n\n    escreva(\"Qual é sua idade? \")\n    leia(idade)\n    se (idade >= 18) {\n      escreva(\"Pode votar.\")\n    } senao {\n      escreva(\"Ainda não pode votar.\")\n    }\n  }\n}";

const VISUALG_SAMPLE: &str = "algoritmo \"votacao\"\nvar\n  idade: inteiro\ninicio\n  escreva(\"Qual é sua idade? \")\n  leia(idade)\n  se idade >= 18 entao\n    escreval(\"Pode votar.\")\n  senao\n    escreval(\"Ainda não pode votar.\")\n  fimse\nfimalgoritmo";
const POTIGOL_SAMPLE: &str = "escreva \"Olá, Potigol!\"";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    Portugol,
    Visualg,
    Potigol,
}

impl Language {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Portugol => "Portugol",
            Self::Visualg => "VisuAlg",
            Self::Potigol => "Potigol",
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Portugol => "portugol",
            Self::Visualg => "visualg",
            Self::Potigol => "potigol",
        }
    }

    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "visualg" => Self::Visualg,
            "potigol" => Self::Potigol,
            _ => Self::Portugol,
        }
    }

    pub(crate) fn file_name(self) -> &'static str {
        match self {
            Self::Portugol => "principal.por",
            Self::Visualg => "principal.alg",
            Self::Potigol => "principal.poti",
        }
    }

    pub(crate) fn sample(self) -> &'static str {
        match self {
            Self::Portugol => PORTUGOL_SAMPLE,
            Self::Visualg => VISUALG_SAMPLE,
            Self::Potigol => POTIGOL_SAMPLE,
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Portugol => "por",
            Self::Visualg => "alg",
            Self::Potigol => "poti",
        }
    }

    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "por" => Some(Self::Portugol),
            "alg" => Some(Self::Visualg),
            "poti" => Some(Self::Potigol),
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
