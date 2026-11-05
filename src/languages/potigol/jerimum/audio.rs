use potoo_potigol_interpreter::JerimumValue;

use crate::features::media::audio::{AudioCommand, AudioResponse};

pub(crate) fn command(function: &str, arguments: &[JerimumValue]) -> Result<AudioCommand, String> {
    let command = match function {
        "carregar_som" | "carregar_musica" => AudioCommand::Load {
            path: string_argument(arguments, 0)?,
            cache_by_path: function == "carregar_som",
        },
        "liberar_som" => AudioCommand::Release {
            handle: integer_argument(arguments, 0)?,
        },
        "reproduzir_som" => AudioCommand::Play {
            handle: integer_argument(arguments, 0)?,
            repeat: logical_argument(arguments, 1)?,
        },
        "pausar_som" => AudioCommand::Pause {
            handle: integer_argument(arguments, 0)?,
        },
        "continuar_som" => AudioCommand::Resume {
            handle: integer_argument(arguments, 0)?,
        },
        "som_tocando" => AudioCommand::IsPlaying {
            handle: integer_argument(arguments, 0)?,
        },
        "musica_pausada" => AudioCommand::IsPaused {
            handle: integer_argument(arguments, 0)?,
        },
        "interromper_som" => AudioCommand::Stop {
            handle: integer_argument(arguments, 0)?,
        },
        "obter_tamanho_musica" => AudioCommand::Duration {
            handle: integer_argument(arguments, 0)?,
        },
        "obter_posicao_atual_musica" => AudioCommand::Position {
            handle: integer_argument(arguments, 0)?,
        },
        "definir_posicao_atual_musica" => AudioCommand::Seek {
            handle: integer_argument(arguments, 0)?,
            milliseconds: integer_argument(arguments, 1)?,
        },
        "definir_volume_reproducao" => AudioCommand::SetPlaybackVolume {
            handle: integer_argument(arguments, 0)?,
            percent: integer_argument(arguments, 1)?,
        },
        "definir_volume" => AudioCommand::SetMasterVolume {
            percent: integer_argument(arguments, 0)?,
        },
        "obter_volume" => AudioCommand::GetMasterVolume,
        "obter_volume_reproducao" => AudioCommand::GetPlaybackVolume {
            handle: integer_argument(arguments, 0)?,
        },
        _ => return Err(format!("função de áudio desconhecida: {function}")),
    };
    Ok(command)
}

pub(crate) fn response(response: AudioResponse) -> JerimumValue {
    match response {
        AudioResponse::Unit => JerimumValue::Null,
        AudioResponse::Handle(handle) | AudioResponse::Integer(handle) => {
            JerimumValue::Integer(handle)
        }
        AudioResponse::Logical(value) => JerimumValue::Logical(value),
    }
}

fn integer_argument(arguments: &[JerimumValue], index: usize) -> Result<i64, String> {
    match arguments.get(index) {
        Some(JerimumValue::Integer(value)) => Ok(*value),
        _ => Err(format!("argumento {} deve ser inteiro", index + 1)),
    }
}

fn logical_argument(arguments: &[JerimumValue], index: usize) -> Result<bool, String> {
    match arguments.get(index) {
        Some(JerimumValue::Logical(value)) => Ok(*value),
        _ => Err(format!("argumento {} deve ser lógico", index + 1)),
    }
}

fn string_argument(arguments: &[JerimumValue], index: usize) -> Result<String, String> {
    match arguments.get(index) {
        Some(JerimumValue::String(value)) => Ok(value.clone()),
        _ => Err(format!(
            "argumento {} deve ser cadeia de caracteres",
            index + 1
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_aliases_keep_their_cache_policies() {
        let arguments = [JerimumValue::String("tone.mp3".to_owned())];
        assert_eq!(
            command("carregar_som", &arguments),
            Ok(AudioCommand::Load {
                path: "tone.mp3".to_owned(),
                cache_by_path: true,
            })
        );
        assert_eq!(
            command("carregar_musica", &arguments),
            Ok(AudioCommand::Load {
                path: "tone.mp3".to_owned(),
                cache_by_path: false,
            })
        );
    }

    #[test]
    fn playback_functions_map_to_typed_commands() {
        let cases = [
            (
                "reproduzir_som",
                vec![JerimumValue::Integer(4), JerimumValue::Logical(true)],
                AudioCommand::Play {
                    handle: 4,
                    repeat: true,
                },
            ),
            (
                "definir_posicao_atual_musica",
                vec![JerimumValue::Integer(4), JerimumValue::Integer(250)],
                AudioCommand::Seek {
                    handle: 4,
                    milliseconds: 250,
                },
            ),
            (
                "definir_volume_reproducao",
                vec![JerimumValue::Integer(4), JerimumValue::Integer(65)],
                AudioCommand::SetPlaybackVolume {
                    handle: 4,
                    percent: 65,
                },
            ),
        ];

        for (function, arguments, expected) in cases {
            assert_eq!(command(function, &arguments), Ok(expected));
        }
    }

    #[test]
    fn invalid_calls_keep_their_errors_and_replies_keep_their_value_types() {
        assert_eq!(
            command("pausar_som", &[]),
            Err("argumento 1 deve ser inteiro".to_owned())
        );
        assert_eq!(
            command("audio_desconhecido", &[]),
            Err("função de áudio desconhecida: audio_desconhecido".to_owned())
        );
        assert!(matches!(response(AudioResponse::Unit), JerimumValue::Null));
        assert!(matches!(
            response(AudioResponse::Handle(4)),
            JerimumValue::Integer(4)
        ));
        assert!(matches!(
            response(AudioResponse::Logical(true)),
            JerimumValue::Logical(true)
        ));
    }
}
