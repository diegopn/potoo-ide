use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;

use super::{AudioCommand, AudioResponse};

struct Sound {
    path: PathBuf,
    media: gtk::MediaFile,
    volume: f64,
}

pub(crate) struct AudioHost {
    sounds: HashMap<i64, Sound>,
    handles_by_path: HashMap<PathBuf, i64>,
    next_handle: i64,
    master_volume: i64,
}

impl Default for AudioHost {
    fn default() -> Self {
        Self {
            sounds: HashMap::new(),
            handles_by_path: HashMap::new(),
            next_handle: 1,
            master_volume: 100,
        }
    }
}

impl AudioHost {
    pub(crate) fn handle(
        &mut self,
        root: Option<&Path>,
        command: AudioCommand,
    ) -> Result<AudioResponse, String> {
        match command {
            AudioCommand::Load {
                path: relative,
                cache_by_path,
            } => {
                let path = resolve_audio_path(root, &relative)?;
                if cache_by_path {
                    if let Some(handle) = self.handles_by_path.get(&path) {
                        return Ok(AudioResponse::Handle(*handle));
                    }
                }
                let media = gtk::MediaFile::for_filename(&path);
                media.set_volume(1.0);
                wait_until_prepared(&media)?;
                let handle = self.next_handle;
                self.next_handle = self
                    .next_handle
                    .checked_add(1)
                    .ok_or_else(|| "não há mais identificadores de som disponíveis".to_owned())?;
                if cache_by_path {
                    self.handles_by_path.insert(path.clone(), handle);
                }
                self.sounds.insert(
                    handle,
                    Sound {
                        path,
                        media,
                        volume: 1.0,
                    },
                );
                Ok(AudioResponse::Handle(handle))
            }
            AudioCommand::Release { handle } => {
                if let Some(sound) = self.sounds.remove(&handle) {
                    sound.media.pause();
                    sound.media.seek(0);
                    if self.handles_by_path.get(&sound.path) == Some(&handle) {
                        self.handles_by_path.remove(&sound.path);
                    }
                }
                Ok(AudioResponse::Unit)
            }
            AudioCommand::Play { handle, repeat } => {
                let sound = self
                    .sounds
                    .get(&handle)
                    .ok_or_else(|| format!("endereço de som inválido ({handle})"))?;
                if sound.media.is_playing() {
                    sound.media.pause();
                    sound.media.seek(0);
                }
                sound.media.set_loop(repeat);
                sound.media.play();
                Ok(AudioResponse::Handle(handle))
            }
            AudioCommand::Pause { handle } => {
                if let Some(sound) = self.sounds.get(&handle) {
                    sound.media.pause();
                }
                Ok(AudioResponse::Unit)
            }
            AudioCommand::Resume { handle } => {
                if let Some(sound) = self.sounds.get(&handle) {
                    sound.media.play();
                }
                Ok(AudioResponse::Unit)
            }
            AudioCommand::IsPlaying { handle } => Ok(AudioResponse::Logical(
                self.sounds
                    .get(&handle)
                    .is_some_and(|sound| sound.media.is_playing()),
            )),
            AudioCommand::IsPaused { handle } => Ok(AudioResponse::Logical(
                self.sounds.get(&handle).is_some_and(|sound| {
                    let position = sound.media.timestamp();
                    let duration = sound.media.duration();
                    !sound.media.is_playing() && position > 0 && position < duration
                }),
            )),
            AudioCommand::Stop { handle } => {
                if let Some(sound) = self.sounds.get(&handle) {
                    sound.media.pause();
                    sound.media.seek(0);
                }
                Ok(AudioResponse::Unit)
            }
            AudioCommand::Duration { handle } => Ok(AudioResponse::Integer(
                self.sounds
                    .get(&handle)
                    .map_or(0, |sound| sound.media.duration() / 1000),
            )),
            AudioCommand::Position { handle } => Ok(AudioResponse::Integer(
                self.sounds
                    .get(&handle)
                    .map_or(0, |sound| sound.media.timestamp() / 1000)
                    .max(0),
            )),
            AudioCommand::Seek {
                handle,
                milliseconds,
            } => {
                let microseconds = milliseconds
                    .checked_mul(1000)
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| "a posição da música deve ser não negativa".to_owned())?;
                if let Some(sound) = self.sounds.get(&handle) {
                    sound.media.seek(microseconds);
                }
                Ok(AudioResponse::Unit)
            }
            AudioCommand::SetPlaybackVolume { handle, percent } => {
                if let Some(sound) = self.sounds.get_mut(&handle) {
                    sound.volume = percent.clamp(0, 100) as f64 / 100.0;
                    sound
                        .media
                        .set_volume(sound.volume * self.master_volume as f64 / 100.0);
                }
                Ok(AudioResponse::Unit)
            }
            AudioCommand::SetMasterVolume { percent } => {
                self.master_volume = percent.clamp(0, 100);
                for sound in self.sounds.values() {
                    sound
                        .media
                        .set_volume(sound.volume * self.master_volume as f64 / 100.0);
                }
                Ok(AudioResponse::Unit)
            }
            AudioCommand::GetMasterVolume => Ok(AudioResponse::Integer(self.master_volume)),
            AudioCommand::GetPlaybackVolume { handle } => Ok(AudioResponse::Integer(
                self.sounds
                    .get(&handle)
                    .map_or(-1, |sound| (sound.volume * 100.0) as i64),
            )),
        }
    }

    pub(crate) fn stop_all(&mut self) {
        for sound in self.sounds.values() {
            sound.media.pause();
            sound.media.seek(0);
        }
        self.sounds.clear();
        self.handles_by_path.clear();
        self.next_handle = 1;
        self.master_volume = 100;
    }
}

fn wait_until_prepared(media: &gtk::MediaFile) -> Result<(), String> {
    if let Some(error) = media.error() {
        return Err(format!("não foi possível carregar o áudio: {error}"));
    }
    if media.is_prepared() {
        return Ok(());
    }

    let main_loop = gtk::glib::MainLoop::new(None, false);
    let loop_on_change = main_loop.clone();
    let handler = media.connect_prepared_notify(move |media| {
        if media.is_prepared() || media.error().is_some() {
            loop_on_change.quit();
        }
    });
    let loop_on_error = main_loop.clone();
    let error_handler = media.connect_error_notify(move |media| {
        if media.error().is_some() {
            loop_on_error.quit();
        }
    });
    let timed_out = Rc::new(Cell::new(false));
    let timeout_state = timed_out.clone();
    let loop_on_timeout = main_loop.clone();
    // shortcut: 30s preparation timeout, extend only if supported codecs need longer.
    let timeout = gtk::glib::timeout_add_local_once(Duration::from_secs(30), move || {
        timeout_state.set(true);
        loop_on_timeout.quit();
    });
    if !media.is_prepared() && media.error().is_none() {
        main_loop.run();
    }
    media.disconnect(handler);
    media.disconnect(error_handler);
    if !timed_out.get() {
        timeout.remove();
    }

    if let Some(error) = media.error() {
        Err(format!("não foi possível carregar o áudio: {error}"))
    } else if media.is_prepared() {
        Ok(())
    } else {
        Err("tempo limite ao preparar o áudio".to_owned())
    }
}

fn resolve_audio_path(root: Option<&Path>, relative: &str) -> Result<PathBuf, String> {
    let root = root.ok_or_else(|| "abra um projeto para carregar arquivos de áudio".to_owned())?;
    let root = std::fs::canonicalize(root)
        .map_err(|error| format!("pasta do projeto indisponível: {error}"))?;
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("o caminho de áudio deve permanecer dentro do projeto".to_owned());
    }
    let path = std::fs::canonicalize(root.join(relative_path))
        .map_err(|error| format!("arquivo de áudio indisponível: {error}"))?;
    if !path.starts_with(root) || !path.is_file() {
        return Err("o caminho de áudio deve apontar para um arquivo do projeto".to_owned());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn temporary_project() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "potoo-audio-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn audio_assets_are_confined_to_the_project() {
        let root = temporary_project();
        std::fs::write(root.join("tone.mp3"), b"sound").unwrap();
        assert_eq!(
            resolve_audio_path(Some(&root), "tone.mp3").unwrap(),
            std::fs::canonicalize(root.join("tone.mp3")).unwrap()
        );
        assert!(resolve_audio_path(Some(&root), "../outside.mp3").is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let outside = root.with_extension("outside");
            std::fs::write(&outside, b"protected").unwrap();
            symlink(&outside, root.join("escape.mp3")).unwrap();
            assert!(resolve_audio_path(Some(&root), "escape.mp3").is_err());
            std::fs::remove_file(outside).unwrap();
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sound_volume_functions_keep_the_documented_range() {
        let mut host = AudioHost::default();
        assert_eq!(
            host.handle(None, AudioCommand::GetMasterVolume).unwrap(),
            AudioResponse::Integer(100)
        );
        host.handle(None, AudioCommand::SetMasterVolume { percent: 140 })
            .unwrap();
        assert_eq!(
            host.handle(None, AudioCommand::GetMasterVolume).unwrap(),
            AudioResponse::Integer(100)
        );
        assert_eq!(
            host.handle(None, AudioCommand::GetPlaybackVolume { handle: 7 })
                .unwrap(),
            AudioResponse::Integer(-1)
        );
    }

    #[test]
    fn finishing_execution_resets_audio_library_state() {
        let mut host = AudioHost::default();
        host.master_volume = 20;
        host.next_handle = 8;
        host.stop_all();
        assert_eq!(host.master_volume, 100);
        assert_eq!(host.next_handle, 1);
        assert!(host.sounds.is_empty());
        assert!(host.handles_by_path.is_empty());
    }
}
