use crate::types::{FunctionSignature, ValueType};

const MODULE: &str = "JerimumModule:";
const TYPE: &str = "JerimumType:";

pub(crate) fn imported_name(name: &str) -> Option<String> {
    let canonical = match name {
        "jogo" | "Jogo" => "Jogo",
        "mouse" | "Mouse" => "Mouse",
        "teclado" | "Teclado" => "Teclado",
        "relógio" | "relogio" | "Relógio" | "Relogio" => "Relogio",
        "cor" | "Cor" => "Cor",
        "Arquivo" => "Arquivo",
        "Desenho" => "Desenho",
        "Imagem" => "Imagem",
        "Animacao" | "Animação" => "Animacao",
        "Fonte" => "Fonte",
        "Som" => "Som",
        "Música" | "Musica" => "Musica",
        "Tela" => "Tela",
        "Graphics2D" => "Graphics2D",
        "Color" => "AwtColor",
        "BufferedImage" => "BufferedImage",
        "JFrame" => "Frame",
        "Canvas" => "Canvas",
        _ => return None,
    };
    Some(
        if matches!(
            canonical,
            "Cor"
                | "Imagem"
                | "Animacao"
                | "Fonte"
                | "Som"
                | "Musica"
                | "Tela"
                | "Graphics2D"
                | "AwtColor"
                | "BufferedImage"
                | "Frame"
                | "Canvas"
        ) {
            format!("{TYPE}{canonical}")
        } else {
            format!("{MODULE}{canonical}")
        },
    )
}

pub(crate) fn constructor_signature(name: &str, arity: usize) -> Option<FunctionSignature> {
    let (parameters, result) = match name {
        "Cor" | "cor" if arity == 3 => (vec![ValueType::Integer; 3], named("Cor")),
        "Imagem" if arity == 1 => (vec![ValueType::Text], named("Imagem")),
        "Animacao" | "Animação" if arity == 2 => (
            vec![
                ValueType::Integer,
                ValueType::List(Box::new(named("Imagem"))),
            ],
            named("Animacao"),
        ),
        "Fonte" if arity == 1 => (vec![ValueType::Integer], named("Fonte")),
        "Som" if arity == 1 => (vec![ValueType::Text], named("Som")),
        "Musica" | "Música" if arity == 1 => (vec![ValueType::Text], named("Musica")),
        "Tela" if arity == 3 => (
            vec![ValueType::Text, ValueType::Integer, ValueType::Integer],
            named("Tela"),
        ),
        _ => return None,
    };
    Some(FunctionSignature {
        parameters,
        return_type: result,
    })
}

pub(crate) fn property_type(owner: &str, name: &str) -> Option<ValueType> {
    let value_type = match owner.strip_prefix(MODULE)? {
        "Jogo" => match name {
            "titulo" => ValueType::Text,
            "largura" | "altura" | "fps" => ValueType::Integer,
            _ => return None,
        },
        "Teclado" if is_key_name(name) => ValueType::Logical,
        "Mouse" => match name {
            "BOTAO_ESQUERDO" | "BOTAO_DIREITO" => ValueType::Logical,
            "x" | "y" => ValueType::Integer,
            _ => return None,
        },
        "Relogio" => match name {
            "agora" => ValueType::Real,
            "milisegundos" => ValueType::Integer,
            _ => return None,
        },
        "_" => return None,
        _ => return None,
    };
    Some(value_type)
}

pub(crate) fn object_property_type(owner: &str, name: &str) -> Option<ValueType> {
    match owner.strip_prefix(TYPE)? {
        "Cor" => match name {
            "vermelho" | "verde" | "azul" => Some(ValueType::Integer),
            "color" => Some(named("AwtColor")),
            _ => None,
        },
        "Imagem" => match name {
            "largura" | "altura" => Some(ValueType::Integer),
            "caminho" => Some(ValueType::Text),
            "buffer" => Some(named("BufferedImage")),
            _ => None,
        },
        "Animacao" => match name {
            "velocidade" => Some(ValueType::Integer),
            "imagens" => Some(ValueType::List(Box::new(named("Imagem")))),
            "imagem" => Some(named("Imagem")),
            _ => None,
        },
        "Fonte" if name == "tamanho" => Some(ValueType::Integer),
        "Tela" => match name {
            "frame" => Some(named("Frame")),
            "canvas" => Some(named("Canvas")),
            _ => None,
        },
        "Som" => match name {
            "arquivo" => Some(ValueType::Text),
            "tocando" => Some(ValueType::Logical),
            "pare" => Some(ValueType::Nothing),
            _ => None,
        },
        "Musica" => match name {
            "arquivo" => Some(ValueType::Text),
            "tocando" | "pausado" => Some(ValueType::Logical),
            "pare" | "pause" | "continue" | "feche" => Some(ValueType::Nothing),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn companion_property_type(owner: &str, name: &str) -> Option<ValueType> {
    (owner == "JerimumType:Cor" && color_constant(name).is_some()).then(|| named("Cor"))
}

pub(crate) fn companion_member_type(owner: &str, name: &str) -> Option<ValueType> {
    companion_property_type(owner, name).or_else(|| {
        let signature = companion_method_signature(owner, name, 3)?;
        Some(ValueType::Function(
            signature.parameters,
            Box::new(signature.return_type),
        ))
    })
}

pub(crate) fn companion_method_signature(
    owner: &str,
    name: &str,
    arity: usize,
) -> Option<FunctionSignature> {
    (owner == "JerimumType:Imagem" && name == "fatie" && arity == 3).then(|| FunctionSignature {
        parameters: vec![ValueType::Text, ValueType::Integer, ValueType::Integer],
        return_type: ValueType::List(Box::new(named("Imagem"))),
    })
}

pub(crate) fn color_constant(name: &str) -> Option<(i32, i32, i32)> {
    Some(match name {
        "AMARELO" => (255, 255, 0),
        "AZUL" => (0, 0, 255),
        "BRANCO" => (255, 255, 255),
        "CIANO" => (0, 255, 255),
        "CINZA" => (128, 128, 128),
        "CINZA_CLARO" => (192, 192, 192),
        "CINZA_ESCURO" => (64, 64, 64),
        "LARANJA" => (255, 200, 0),
        "MAGENTA" => (255, 0, 255),
        "PRETO" => (0, 0, 0),
        "ROSA" => (255, 175, 175),
        "VERDE" => (0, 255, 0),
        "VERMELHO" => (255, 0, 0),
        _ => return None,
    })
}

pub(crate) fn member_type(owner: &str, name: &str) -> Option<ValueType> {
    property_type(owner, name)
        .or_else(|| object_property_type(owner, name))
        .or_else(|| {
            let signature = signature_with_max_arity(owner, name)?;
            Some(ValueType::Function(
                signature.parameters,
                Box::new(signature.return_type),
            ))
        })
}

pub(crate) fn method_signature(owner: &str, name: &str, arity: usize) -> Option<FunctionSignature> {
    if owner.starts_with(TYPE) {
        return object_method_signature(owner, name, arity);
    }
    let (parameters, result, min_arity) = match owner.strip_prefix(MODULE)? {
        "Jogo" => match name {
            "iniciar" => (
                vec![
                    ValueType::Text,
                    ValueType::Integer,
                    ValueType::Integer,
                    ValueType::Function(vec![], Box::new(ValueType::Nothing)),
                    ValueType::Function(vec![], Box::new(ValueType::Nothing)),
                    ValueType::Integer,
                ],
                ValueType::Nothing,
                0,
            ),
            "distância" | "distancia" => (vec![ValueType::Real; 4], ValueType::Real, 4),
            "projeção_X" | "projecao_X" | "projeçao_X" | "projecão_X" => {
                (vec![ValueType::Real; 2], ValueType::Real, 2)
            }
            "projeção_Y" | "projecao_Y" | "projeçao_Y" | "projecão_Y" => {
                (vec![ValueType::Real; 2], ValueType::Real, 2)
            }
            _ => return None,
        },
        "Arquivo" if name == "leia" => (
            vec![ValueType::Text],
            ValueType::List(Box::new(ValueType::Text)),
            1,
        ),
        "Imagem" if name == "fatie" => (
            vec![ValueType::Text, ValueType::Integer, ValueType::Integer],
            ValueType::List(Box::new(named("Imagem"))),
            3,
        ),
        "Desenho" if name == "incluir" => (
            vec![
                ValueType::Integer,
                ValueType::Function(vec![named("Graphics2D")], Box::new(ValueType::Nothing)),
            ],
            ValueType::Nothing,
            2,
        ),
        "Desenho" if name == "desenhe" => (vec![named("Graphics2D")], ValueType::Nothing, 1),
        _ => return None,
    };
    let max_arity = parameters.len();
    (min_arity..=max_arity)
        .contains(&arity)
        .then(|| FunctionSignature {
            parameters: parameters[..arity].to_vec(),
            return_type: result,
        })
}

fn object_method_signature(owner: &str, name: &str, arity: usize) -> Option<FunctionSignature> {
    let (parameters, result, min_arity) = match owner.strip_prefix(TYPE)? {
        "Imagem" if name == "fatie" => (
            vec![ValueType::Integer; 2],
            ValueType::List(Box::new(named("Imagem"))),
            2,
        ),
        "Imagem" if name == "desenhe" || name == "desenhe_centralizado" => (
            vec![
                ValueType::Real,
                ValueType::Real,
                ValueType::Integer,
                ValueType::Real,
                ValueType::Real,
                ValueType::Real,
            ],
            ValueType::Nothing,
            3,
        ),
        "Fonte" if name == "desenhe" || name == "desenhe_centralizado" => (
            vec![
                ValueType::Text,
                ValueType::Real,
                ValueType::Real,
                ValueType::Integer,
                named("Cor"),
            ],
            ValueType::Nothing,
            4,
        ),
        "Animacao" if name == "imagem" => (vec![], named("Imagem"), 0),
        "Som" if name == "toque" => (vec![], ValueType::Nothing, 0),
        "Som" if name == "pare" => (vec![], ValueType::Nothing, 0),
        "Musica" if name == "toque" => (vec![ValueType::Logical], ValueType::Nothing, 0),
        "Musica" if matches!(name, "pare" | "pause" | "continue" | "feche") => {
            (vec![], ValueType::Nothing, 0)
        }
        "AwtColor"
            if matches!(
                name,
                "getRed" | "getGreen" | "getBlue" | "getAlpha" | "getRGB"
            ) =>
        {
            (vec![], ValueType::Integer, 0)
        }
        "BufferedImage" if matches!(name, "getWidth" | "getHeight") => {
            (vec![], ValueType::Integer, 0)
        }
        "BufferedImage" if name == "getRGB" => (vec![ValueType::Integer; 2], ValueType::Integer, 2),
        "BufferedImage" if name == "getSubimage" => {
            (vec![ValueType::Integer; 4], named("BufferedImage"), 4)
        }
        "Frame" if name == "getTitle" => (vec![], ValueType::Text, 0),
        "Frame" if matches!(name, "getWidth" | "getHeight") => (vec![], ValueType::Integer, 0),
        "Canvas" if matches!(name, "getWidth" | "getHeight") => (vec![], ValueType::Integer, 0),
        "Graphics2D" if name == "setColor" => (vec![named("AwtColor")], ValueType::Nothing, 1),
        "Graphics2D" if name == "getColor" => (vec![], named("AwtColor"), 0),
        "Graphics2D" if name == "drawLine" => (vec![ValueType::Real; 4], ValueType::Nothing, 4),
        "Graphics2D" if matches!(name, "drawRect" | "fillRect" | "drawOval" | "fillOval") => {
            (vec![ValueType::Real; 4], ValueType::Nothing, 4)
        }
        "Graphics2D" if name == "drawString" => (
            vec![ValueType::Text, ValueType::Real, ValueType::Real],
            ValueType::Nothing,
            3,
        ),
        "Graphics2D" if name == "drawImage" => {
            let (parameters, min_arity) = match arity {
                3 => (
                    vec![
                        named("BufferedImage"),
                        ValueType::Integer,
                        ValueType::Integer,
                    ],
                    3,
                ),
                4 => (
                    vec![
                        named("BufferedImage"),
                        ValueType::Integer,
                        ValueType::Integer,
                        ValueType::Any,
                    ],
                    4,
                ),
                5 => (
                    vec![
                        named("BufferedImage"),
                        ValueType::Integer,
                        ValueType::Integer,
                        ValueType::Integer,
                        ValueType::Integer,
                    ],
                    5,
                ),
                6 => (
                    vec![
                        named("BufferedImage"),
                        ValueType::Integer,
                        ValueType::Integer,
                        ValueType::Integer,
                        ValueType::Integer,
                        ValueType::Any,
                    ],
                    6,
                ),
                _ => return None,
            };
            (parameters, ValueType::Logical, min_arity)
        }
        _ => return None,
    };
    (min_arity..=parameters.len())
        .contains(&arity)
        .then(|| FunctionSignature {
            parameters: parameters[..arity].to_vec(),
            return_type: result,
        })
}

fn signature_with_max_arity(owner: &str, name: &str) -> Option<FunctionSignature> {
    if let Some(property_type) = object_property_type(owner, name) {
        if matches!(name, "tocando" | "pausado" | "imagem") {
            return None;
        }
        return Some(FunctionSignature {
            parameters: Vec::new(),
            return_type: property_type,
        });
    }
    if owner.starts_with(TYPE) {
        let max_arity = match owner.strip_prefix(TYPE)? {
            "Imagem" if name == "fatie" => 2,
            "Imagem" if name == "desenhe" || name == "desenhe_centralizado" => 6,
            "Fonte" if name == "desenhe" || name == "desenhe_centralizado" => 5,
            "Animacao" if name == "imagem" => 0,
            "Som" if matches!(name, "toque" | "pare") => 0,
            "Musica" if name == "toque" => 1,
            "Musica" if matches!(name, "pare" | "pause" | "continue" | "feche") => 0,
            "AwtColor"
                if matches!(
                    name,
                    "getRed" | "getGreen" | "getBlue" | "getAlpha" | "getRGB"
                ) =>
            {
                0
            }
            "BufferedImage" if matches!(name, "getWidth" | "getHeight") => 0,
            "BufferedImage" if name == "getRGB" => 2,
            "BufferedImage" if name == "getSubimage" => 4,
            "Frame" if name == "getTitle" => 0,
            "Frame" if matches!(name, "getWidth" | "getHeight") => 0,
            "Canvas" if matches!(name, "getWidth" | "getHeight") => 0,
            "Graphics2D" if name == "setColor" => 1,
            "Graphics2D" if name == "getColor" => 0,
            "Graphics2D" if name == "drawLine" => 4,
            "Graphics2D" if matches!(name, "drawRect" | "fillRect" | "drawOval" | "fillOval") => 4,
            "Graphics2D" if name == "drawString" => 3,
            "Graphics2D" if name == "drawImage" => 6,
            _ => return None,
        };
        return object_method_signature(owner, name, max_arity);
    }
    let max_arity = match owner.strip_prefix(MODULE)? {
        "Jogo" => match name {
            "iniciar" => 6,
            "distância" | "distancia" => 4,
            "projeção_X" | "projecao_X" | "projeçao_X" | "projecão_X" | "projeção_Y"
            | "projecao_Y" | "projeçao_Y" | "projecão_Y" => 2,
            _ => return None,
        },
        "Arquivo" if name == "leia" => 1,
        "Imagem" if name == "fatie" => 3,
        "Desenho" if name == "incluir" => 2,
        "Desenho" if name == "desenhe" => 1,
        _ => return None,
    };
    method_signature(owner, name, max_arity)
}

pub(crate) fn max_arity(owner: &str, name: &str) -> Option<usize> {
    signature_with_max_arity(owner, name).map(|signature| signature.parameters.len())
}

pub(crate) fn mutable_module_property(owner: &str, name: &str) -> Option<ValueType> {
    match owner.strip_prefix(MODULE)? {
        "Jogo" if matches!(name, "titulo" | "largura" | "altura" | "fps") => {
            property_type(owner, name)
        }
        _ => None,
    }
}

pub(crate) fn constructor_name(name: &str) -> Option<&'static str> {
    match name {
        "Cor" | "cor" => Some("Cor"),
        "Imagem" => Some("Imagem"),
        "Animacao" | "Animação" => Some("Animacao"),
        "Fonte" => Some("Fonte"),
        "Som" => Some("Som"),
        "Musica" | "Música" => Some("Musica"),
        "Tela" => Some("Tela"),
        _ => None,
    }
}

fn is_key_name(name: &str) -> bool {
    matches!(
        name,
        "TECLA_ESPACO"
            | "TECLA_ESPAÇO"
            | "TECLA_ENTER"
            | "TECLA_PARA_CIMA"
            | "TECLA_PARA_BAIXO"
            | "TECLA_PARA_ESQUERDA"
            | "TECLA_PARA_DIREITA"
    ) || name
        .strip_prefix("TECLA_")
        .is_some_and(|key| key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric())
}

pub(crate) fn named(name: &str) -> ValueType {
    ValueType::Named(format!("{TYPE}{name}"))
}
