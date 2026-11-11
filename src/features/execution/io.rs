use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::time::{Duration, Instant};

use crate::features::media::audio::{AudioCommand, AudioResponse};
use gtk::gio::prelude::*;
use potoo_portugol_interpreter as portugol;

use super::{
    ControlAction, DebugPauseReason, DebuggerSnapshot, ExecutionEvent, HostRequest, InputPrompt,
    MAX_FILE_BYTES, STOP_SENTINEL,
};

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const OUTPUT_LIMIT_MESSAGE: &str = "\n[saída truncada: limite de 1 MiB]\n";
const OUTPUT_BATCH_BYTES: usize = 4096;

pub(crate) struct WorkerIo {
    pub(crate) events: SyncSender<ExecutionEvent>,
    pub(crate) input: Receiver<String>,
    pub(crate) control: Receiver<ControlAction>,
    pub(crate) pending_action: Option<ControlAction>,
    pub(crate) debugging: bool,
    pub(crate) project_root: Option<PathBuf>,
    pub(crate) cancellable: gtk::gio::Cancellable,
    pub(crate) output_bytes: usize,
    pub(crate) output_pending: String,
    pub(crate) output_truncated: bool,
    pub(crate) stopped: bool,
    pub(crate) last_visualg_line: Option<usize>,
    pub(crate) open_portugol_files:
        HashMap<i64, crate::languages::portugol::studio_20::files::OpenPortugolFile>,
}

impl WorkerIo {
    pub(crate) fn http_request(
        &mut self,
        request: portugol::HttpRequest,
    ) -> Result<portugol::HttpResponse, String> {
        let result = crate::languages::portugol::studio_20::http::send_http_request_cancellable(
            request,
            Some(&self.cancellable),
        );
        if self.cancellable.is_cancelled() {
            self.stopped = true;
            Err(STOP_SENTINEL.to_owned())
        } else {
            result
        }
    }

    pub(crate) fn flush_output(&mut self) {
        if !self.output_pending.is_empty() {
            let output = std::mem::take(&mut self.output_pending);
            let _ = self.events.send(ExecutionEvent::Output(output));
        }
    }

    pub(crate) fn request_host(
        &mut self,
        request: HostRequest,
    ) -> Result<portugol::HostValue, String> {
        self.request_host_response(|response| ExecutionEvent::HostRequest { request, response })
    }

    pub(crate) fn request_audio_host(
        &mut self,
        command: AudioCommand,
    ) -> Result<AudioResponse, String> {
        self.request_host_response(|response| ExecutionEvent::AudioRequest { command, response })
    }

    pub(crate) fn request_host_response<T>(
        &mut self,
        create_event: impl FnOnce(Sender<Result<T, String>>) -> ExecutionEvent,
    ) -> Result<T, String> {
        self.flush_output();
        let (sender, response) = mpsc::channel();
        self.events
            .send(create_event(sender))
            .map_err(|_| "a janela da IDE foi fechada".to_owned())?;
        loop {
            match response.recv_timeout(Duration::from_millis(40)) {
                Ok(result) => return result,
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("a solicitação para a janela foi cancelada".to_owned());
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.poll_stop() {
                        self.stopped = true;
                        return Err(STOP_SENTINEL.to_owned());
                    }
                }
            }
        }
    }

    pub(crate) fn send_input_prompt(&mut self, prompt: InputPrompt) -> Result<String, String> {
        self.send_input_prompt_with_timeout(prompt, None)
    }

    pub(crate) fn send_input_prompt_with_timeout(
        &mut self,
        prompt: InputPrompt,
        timeout: Option<Duration>,
    ) -> Result<String, String> {
        self.flush_output();
        self.events
            .send(ExecutionEvent::InputRequested(prompt))
            .map_err(|_| "a janela da IDE foi fechada".to_owned())?;
        let started = Instant::now();
        loop {
            let poll_interval = Duration::from_millis(40);
            let wait = timeout.map_or(poll_interval, |limit| {
                limit.saturating_sub(started.elapsed()).min(poll_interval)
            });
            match self.input.recv_timeout(wait) {
                Ok(input) => return Ok(input),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("a entrada do programa foi cancelada".to_owned());
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.poll_stop() {
                        self.stopped = true;
                        return Err(STOP_SENTINEL.to_owned());
                    }
                    if timeout.is_some_and(|limit| started.elapsed() >= limit) {
                        return Err("tempo limite de entrada excedido".to_owned());
                    }
                }
            }
        }
    }

    pub(crate) fn poll_stop(&mut self) -> bool {
        if self.cancellable.is_cancelled() {
            return true;
        }
        loop {
            match self.control.try_recv() {
                Ok(ControlAction::Stop) => return true,
                Ok(action) => self.pending_action = Some(action),
                Err(_) => return false,
            }
        }
    }

    pub(crate) fn await_action(
        &mut self,
        snapshot: DebuggerSnapshot,
    ) -> Result<ControlAction, String> {
        self.flush_output();
        self.events
            .send(ExecutionEvent::Debugger(snapshot))
            .map_err(|_| "a janela da IDE foi fechada".to_owned())?;
        if let Some(action) = self.pending_action.take() {
            if action == ControlAction::Stop {
                self.stopped = true;
            }
            return Ok(action);
        }
        loop {
            match self.control.recv_timeout(Duration::from_millis(40)) {
                Ok(action) => {
                    if action == ControlAction::Stop {
                        self.stopped = true;
                    }
                    return Ok(action);
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.stopped = true;
                    return Ok(ControlAction::Stop);
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    pub(crate) fn write_limited(&mut self, text: &str) {
        if self.output_truncated {
            return;
        }
        let remaining = MAX_OUTPUT_BYTES.saturating_sub(self.output_bytes);
        let mut end = text.len().min(remaining);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end > 0 {
            self.output_pending.push_str(&text[..end]);
            self.output_bytes += end;
            if self.output_pending.len() >= OUTPUT_BATCH_BYTES {
                self.flush_output();
            }
        }
        if end < text.len() {
            self.output_pending.push_str(OUTPUT_LIMIT_MESSAGE);
            self.output_truncated = true;
            self.flush_output();
        }
    }

    pub(crate) fn execution_snapshot(
        line: usize,
        column: usize,
        variables: impl IntoIterator<Item = (String, String)>,
        pause_reason: Option<DebugPauseReason>,
    ) -> DebuggerSnapshot {
        DebuggerSnapshot {
            line,
            column,
            variables: variables.into_iter().collect(),
            pause_reason,
        }
    }

    pub(crate) fn read_data_file(&mut self, path: &str) -> Result<Option<Vec<String>>, String> {
        let path = self.safe_data_path(path)?;
        if !path.exists() {
            return Ok(None);
        }
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("não foi possível consultar o arquivo: {error}"))?;
        if !metadata.is_file() {
            return Err("o arquivo de dados precisa ser um arquivo regular".to_owned());
        }
        if metadata.len() > MAX_FILE_BYTES {
            return Err("arquivo de dados excede o limite de 1 MiB".to_owned());
        }
        let file = fs::File::open(path)
            .map_err(|error| format!("não foi possível abrir o arquivo: {error}"))?;
        let mut contents = String::new();
        file.take(MAX_FILE_BYTES + 1)
            .read_to_string(&mut contents)
            .map_err(|error| format!("não foi possível ler o arquivo: {error}"))?;
        if contents.len() as u64 > MAX_FILE_BYTES {
            return Err("arquivo de dados excede o limite de 1 MiB".to_owned());
        }
        Ok(Some(contents.lines().map(str::to_owned).collect()))
    }

    pub(crate) fn safe_data_path(&self, relative: &str) -> Result<PathBuf, String> {
        let root = self
            .project_root
            .as_deref()
            .ok_or_else(|| "abra um projeto para usar arquivos de dados".to_owned())?;
        let root = fs::canonicalize(root)
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
            return Err(
                "o caminho do arquivo de dados deve permanecer dentro do projeto".to_owned(),
            );
        }
        let candidate = root.join(relative_path);
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {
                let canonical = fs::canonicalize(&candidate)
                    .map_err(|error| format!("caminho do arquivo inválido: {error}"))?;
                if !canonical.starts_with(&root) {
                    return Err("o caminho do arquivo de dados sai da pasta do projeto".to_owned());
                }
                Ok(canonical)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut cursor = candidate.as_path();
                let mut missing_components = Vec::new();
                loop {
                    match fs::canonicalize(cursor) {
                        Ok(canonical_parent) => {
                            if !canonical_parent.starts_with(&root) {
                                return Err(
                                    "o caminho do arquivo de dados sai da pasta do projeto"
                                        .to_owned(),
                                );
                            }
                            let mut safe_path = canonical_parent;
                            for component in missing_components.iter().rev() {
                                safe_path.push(component);
                            }
                            return Ok(safe_path);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            let component = cursor
                                .file_name()
                                .ok_or_else(|| "caminho do arquivo inválido".to_owned())?;
                            missing_components.push(component.to_os_string());
                            cursor = cursor
                                .parent()
                                .ok_or_else(|| "caminho do arquivo inválido".to_owned())?;
                        }
                        Err(error) => {
                            return Err(format!("pasta do arquivo indisponível: {error}"));
                        }
                    }
                }
            }
            Err(error) => Err(format!("caminho do arquivo inválido: {error}")),
        }
    }
}
