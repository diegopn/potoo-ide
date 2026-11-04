use super::SourceCall;
use crate::languages::model::Language;

pub(crate) struct LibraryCallKnowledge {
    pub(crate) explanation: &'static str,
    pub(crate) concept: &'static str,
    pub(crate) experiment: &'static str,
}

pub(crate) fn lookup_library_call(
    language: Language,
    call: &SourceCall,
) -> Option<LibraryCallKnowledge> {
    if language != Language::Potigol
        || !call
            .library
            .as_deref()
            .is_some_and(|library| library.eq_ignore_ascii_case("jerimum"))
    {
        return None;
    }

    let receiver = call
        .receiver
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let knowledge = match (receiver.as_str(), call.member.as_str()) {
        ("jogo", "iniciar") => LibraryCallKnowledge {
            explanation: "iniciar configura e abre a janela do jogo com título, dimensões, funções de atualização e desenho, e taxa de quadros.",
            concept: "O laço do jogo chama a atualização e o desenho repetidamente enquanto a janela estiver aberta.",
            experiment: "Altere a largura e a altura da chamada e observe como o espaço do jogo muda.",
        },
        ("desenho", "incluir") => LibraryCallKnowledge {
            explanation: "incluir registra uma função de desenho para compor a cena do jogo.",
            concept: "Separar cada elemento em uma função de desenho ajuda a organizar a cena.",
            experiment: "Comente temporariamente esta inclusão e observe qual elemento deixa de aparecer.",
        },
        (_, "setColor") => LibraryCallKnowledge {
            explanation: "setColor define a cor usada pelas próximas operações gráficas deste contexto.",
            concept: "A cor é um estado do contexto gráfico e permanece ativa até outra chamada de setColor.",
            experiment: "Troque o argumento de cor por outra constante Cor e observe os próximos desenhos.",
        },
        (_, "fillRect") => LibraryCallKnowledge {
            explanation: "fillRect desenha um retângulo preenchido usando posição x/y e largura/altura.",
            concept: "As coordenadas indicam a posição do retângulo; largura e altura definem seu tamanho.",
            experiment: "Aumente os dois últimos argumentos e observe o tamanho do retângulo.",
        },
        (_, "drawString") => LibraryCallKnowledge {
            explanation: "drawString desenha o texto informado nas coordenadas x/y da área gráfica.",
            concept: "A posição do texto é definida por coordenadas dentro da tela do jogo.",
            experiment: "Mude o texto ou suas coordenadas e observe o resultado na tela.",
        },
        _ => return None,
    };
    Some(knowledge)
}
