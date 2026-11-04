use std::path::{Path, PathBuf};

use crate::diagnostics::EditorDiagnostic;

const PORTUGOL_SAMPLE: &str = r#"programa {
    funcao logico eh_par(inteiro numero) {
        retorne numero % 2 == 0
    }

    funcao inicio() {
        inteiro valor
        escreva("Digite um número inteiro: ")
        leia(valor)

        se (eh_par(valor)) {
            escreva("O número é par\n")
        } senao {
            escreva("O número é ímpar\n")
        }
    }
}"#;

const VISUALG_SAMPLE: &str = r#"algoritmo "Resultado da turma"
var
    nota1, nota2, nota3, media: real
funcao aprovado(media: real): logico
inicio
    retorne media >= 7
fimfuncao

inicio
    escreval("Digite três notas:")
    leia(nota1, nota2, nota3)
    media <- (nota1 + nota2 + nota3) / 3

    se aprovado(media) entao
        escreval("Aprovado com média ", media:5:2)
    senao
        escreval("Estude mais: média ", media:5:2)
    fimse
fimalgoritmo"#;
const POTIGOL_SAMPLE: &str = r#"media(a: Real, b: Real, c: Real) = (a + b + c) / 3

escreva "Digite três notas:"
a, b, c = leia_numero
resultado = media(a, b, c)

se resultado >= 7 então
  escreva "Aprovada: {resultado}"
senão
  escreva "Em recuperação: {resultado}"
fim"#;
const POTIGOL_JERIMUM_GAME_SAMPLE: &str = r#"use "jerimum"

# Mova o quadrado verde com WASD ou as setas.
var jogador_x := 144
var jogador_y := 84

atualize(): Nada
  jogador_x := se (teclado.TECLA_D ou teclado.TECLA_PARA_DIREITA) e jogador_x < 288 então jogador_x + 4 senão jogador_x fim
  jogador_x := se (teclado.TECLA_A ou teclado.TECLA_PARA_ESQUERDA) e jogador_x > 0 então jogador_x - 4 senão jogador_x fim
  jogador_y := se (teclado.TECLA_S ou teclado.TECLA_PARA_BAIXO) e jogador_y < 168 então jogador_y + 4 senão jogador_y fim
  jogador_y := se (teclado.TECLA_W ou teclado.TECLA_PARA_CIMA) e jogador_y > 0 então jogador_y - 4 senão jogador_y fim
fim

desenhe_personagem(g: Graphics2D): Nada
  g.setColor(Cor.VERDE.color)
  g.fillRect(jogador_x, jogador_y, 32, 32)
  g.setColor(Cor.PRETO.color)
  g.drawString("WASD / setas: mover", 8, 20)
fim

desenhe(): Nada
  Desenho.incluir(1, desenhe_personagem)
fim

jogo.iniciar("Mova o quadrado", 320, 200, atualize, desenhe, 60)"#;
const GPORTUGOL_SAMPLE: &str = r#"algoritmo contagem;
variáveis
    limite, numero : inteiro;
fim-variáveis

início
    imprima("Digite o limite:");
    limite := leia();

    para numero de 1 até limite faça
        se numero % 2 = 0 então
            imprima(numero, " é par.");
        senão
            imprima(numero, " é ímpar.");
        fim-se
    fim-para
fim"#;
const CALANGO_SAMPLE: &str = r#"algoritmo mediaTurma;
principal
    inteiro i;
    real notas[3];
    real soma;
    soma = 0;
    para i de 0 ate 2 passo 1 faca
        escreva("Nota: ");
        leia(notas[i]);
        soma = soma + notas[i];
    fimPara
    se (soma / 3 >= 7) entao
        escreval("Aprovado");
    senao
        escreval("Continue estudando");
    fimSe
fimPrincipal"#;
const MAPLER_SAMPLE: &str = r#"variaveis
    valor_a: real;
    valor_b: real;
    media: real;
inicio
    escrever "Primeiro valor:";
    ler valor_a;
    escrever "Segundo valor:";
    ler valor_b;
    media <- (valor_a + valor_b) / 2;
    escrever "Média: ", media;
    se media >= 7 entao
        escrever "Resultado suficiente";
    senao
        escrever "Revise o conteúdo";
    fim se;
fim."#;

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
            Self::Portugol => PORTUGOL_SAMPLE,
            Self::Visualg => VISUALG_SAMPLE,
            Self::Potigol => POTIGOL_SAMPLE,
            Self::GPortugol => GPORTUGOL_SAMPLE,
            Self::Calango => CALANGO_SAMPLE,
            Self::Mapler => MAPLER_SAMPLE,
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

#[derive(Clone, Debug)]
pub(crate) struct SourceDocument {
    pub(crate) id: u64,
    pub(crate) path: Option<PathBuf>,
    pub(crate) name: String,
    pub(crate) language: Language,
    pub(crate) contents: String,
    pub(crate) saved_contents: String,
    pub(crate) revision: u64,
    pub(crate) diagnostics: Option<Vec<EditorDiagnostic>>,
    pub(crate) runtime_diagnostics: Vec<EditorDiagnostic>,
}

impl SourceDocument {
    pub(crate) fn is_dirty(&self) -> bool {
        self.contents != self.saved_contents
    }
}
