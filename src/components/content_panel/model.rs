use std::path::{Path, PathBuf};

const PORTUGOL_SAMPLE: &str = "programa {\n  funcao inicio() {\n    inteiro idade\n\n    escreva(\"Qual é sua idade? \")\n    leia(idade)\n    se (idade >= 18) {\n      escreva(\"Pode votar.\")\n    } senao {\n      escreva(\"Ainda não pode votar.\")\n    }\n  }\n}";

const VISUALG_SAMPLE: &str = "algoritmo \"votacao\"\nvar\n  idade: inteiro\ninicio\n  escreva(\"Qual é sua idade? \")\n  leia(idade)\n  se idade >= 18 entao\n    escreval(\"Pode votar.\")\n  senao\n    escreval(\"Ainda não pode votar.\")\n  fimse\nfimalgoritmo";
const POTIGOL_SAMPLE: &str = "escreva \"Olá, Potigol!\"";
const POTIGOL_JERIMUM_GAME_SAMPLE: &str = r#"use "jerimum"

# Mova o quadrado verde com as teclas A e D.
var jogador_x := 144

atualize(): Nada
  jogador_x := se teclado.TECLA_D e jogador_x < 280 então jogador_x + 4 senão jogador_x fim
  jogador_x := se teclado.TECLA_A e jogador_x > 0 então jogador_x - 4 senão jogador_x fim
fim

desenhe_personagem(g: Graphics2D): Nada
  g.setColor(Cor.VERDE.color)
  g.fillRect(jogador_x, 120, 32, 32)
  g.setColor(Cor.PRETO.color)
  g.drawString("A/D: mover", 8, 20)
fim

desenhe(): Nada
  Desenho.incluir(1, desenhe_personagem)
fim

jogo.iniciar("Mova o quadrado", 320, 200, atualize, desenhe, 60)"#;
const GPORTUGOL_SAMPLE: &str = "algoritmo exemplo;\ninício\n  imprima(\"Olá, G-Portugol!\");\nfim";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    Portugol,
    Visualg,
    Potigol,
    GPortugol,
}

impl Language {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Portugol => "Portugol",
            Self::Visualg => "VisuAlg",
            Self::Potigol => "Potigol",
            Self::GPortugol => "G-Portugol",
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Portugol => "portugol",
            Self::Visualg => "visualg",
            Self::Potigol => "potigol",
            Self::GPortugol => "gportugol",
        }
    }

    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "visualg" => Self::Visualg,
            "potigol" => Self::Potigol,
            "gportugol" => Self::GPortugol,
            _ => Self::Portugol,
        }
    }

    pub(crate) fn file_name(self) -> &'static str {
        match self {
            Self::Portugol => "principal.por",
            Self::Visualg => "principal.alg",
            Self::Potigol => "principal.poti",
            Self::GPortugol => "principal.gpt",
        }
    }

    pub(crate) fn sample(self) -> &'static str {
        match self {
            Self::Portugol => PORTUGOL_SAMPLE,
            Self::Visualg => VISUALG_SAMPLE,
            Self::Potigol => POTIGOL_SAMPLE,
            Self::GPortugol => GPORTUGOL_SAMPLE,
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Portugol => "por",
            Self::Visualg => "alg",
            Self::Potigol => "poti",
            Self::GPortugol => "gpt",
        }
    }

    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "por" => Some(Self::Portugol),
            "alg" => Some(Self::Visualg),
            "poti" => Some(Self::Potigol),
            "gpt" => Some(Self::GPortugol),
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
}

impl ProjectTemplate {
    pub(crate) fn default_for_language(language: Language) -> Self {
        match language {
            Language::Portugol => Self::Portugol20,
            Language::Visualg => Self::Visualg,
            Language::Potigol => Self::PotigolClassic,
            Language::GPortugol => Self::GPortugol,
        }
    }

    pub(crate) fn language(self) -> Language {
        match self {
            Self::Portugol20 => Language::Portugol,
            Self::GPortugol => Language::GPortugol,
            Self::Visualg => Language::Visualg,
            Self::PotigolClassic | Self::PotigolJerimum => Language::Potigol,
        }
    }

    pub(crate) fn file_name(self) -> &'static str {
        self.language().file_name()
    }

    pub(crate) fn sample(self) -> &'static str {
        match self {
            Self::PotigolJerimum => POTIGOL_JERIMUM_GAME_SAMPLE,
            _ => self.language().sample(),
        }
    }

    pub(crate) fn new_file_sample(self) -> &'static str {
        match self {
            Self::PotigolJerimum => POTIGOL_JERIMUM_GAME_SAMPLE,
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
        ] {
            assert_eq!(
                Language::from_path(Path::new(path)),
                Some(expected),
                "{path}"
            );
        }
        assert_eq!(Language::from_path(Path::new("main.txt")), None);
    }

    #[test]
    fn gportugol_starter_uses_gpt_extension() {
        let language = Language::GPortugol;
        assert_eq!(language.file_name(), "principal.gpt");
        assert_eq!(language.extension(), "gpt");
        assert!(language.sample().contains("algoritmo exemplo;"));
        assert_eq!(
            Language::from_path(Path::new(language.file_name())),
            Some(language)
        );
        potoo_gportugol_interpreter::parse(language.sample())
            .expect("the starter program should be valid G-Portugol");
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
