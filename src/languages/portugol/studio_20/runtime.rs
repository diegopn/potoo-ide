use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use super::files::{
    OpenPortugolFile, PORTUGOL_FILE_LINE_ENDING, portugol_file_selection_matches,
    replace_file_text_regex, split_file_lines,
};
use super::http::MAX_HTTP_BODY_BYTES;
use crate::features::execution::{
    ControlAction, DiagnosticKind, ExecutionDiagnostic, ExecutionEvent, HostRequest, InputPrompt,
    MAX_FILE_BYTES, STOP_SENTINEL, WorkerIo,
};
use potoo_portugol_interpreter as portugol;

impl portugol::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: None,
            data_type: None,
            immediate_character: false,
        })
    }

    fn clear_output(&mut self) {
        self.output_pending.clear();
        let _ = self.events.send(ExecutionEvent::ClearOutput);
    }

    fn should_stop(&mut self) -> bool {
        if self.poll_stop() {
            self.stopped = true;
            true
        } else {
            false
        }
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        let started = std::time::Instant::now();
        loop {
            if self.poll_stop() {
                self.stopped = true;
                return Err(STOP_SENTINEL.to_owned());
            }
            let remaining = duration.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Ok(());
            }
            match self
                .control
                .recv_timeout(remaining.min(Duration::from_millis(40)))
            {
                Ok(ControlAction::Stop) => {
                    self.stopped = true;
                    return Err(STOP_SENTINEL.to_owned());
                }
                Ok(action) => self.pending_action = Some(action),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.stopped = true;
                    return Err(STOP_SENTINEL.to_owned());
                }
            }
        }
    }

    fn calendar_now(&mut self) -> Result<portugol::CalendarDateTime, String> {
        let now = gtk::glib::DateTime::now_local().map_err(|error| error.to_string())?;
        let weekday = now.day_of_week().rem_euclid(7) + 1;
        Ok(portugol::CalendarDateTime {
            year: now.year(),
            month: now.month() as u32,
            day: now.day_of_month() as u32,
            weekday: weekday as u32,
            hour: now.hour() as u32,
            minute: now.minute() as u32,
            second: now.second() as u32,
            millisecond: (now.microsecond() / 1_000) as u32,
        })
    }

    fn portugol_file_open(
        &mut self,
        path: &str,
        mode: portugol::FileAccessMode,
    ) -> Result<i64, String> {
        if self.open_portugol_files.len() >= 10 {
            return Err("o limite de 10 arquivos abertos foi atingido".to_owned());
        }
        let path = self.safe_data_path(path)?;
        if self
            .open_portugol_files
            .values()
            .any(|file| file.path() == path)
        {
            return Err("o arquivo já está aberto".to_owned());
        }
        let handle = (0..10)
            .find(|handle| !self.open_portugol_files.contains_key(handle))
            .ok_or_else(|| "não há um endereço livre para o arquivo".to_owned())?;
        let open_file = match mode {
            portugol::FileAccessMode::Read => {
                let file = File::open(&path)
                    .map_err(|error| format!("não foi possível abrir o arquivo: {error}"))?;
                let metadata = file
                    .metadata()
                    .map_err(|error| format!("não foi possível consultar o arquivo: {error}"))?;
                if !metadata.is_file() {
                    return Err("o caminho não é um arquivo regular".to_owned());
                }
                if metadata.len() > MAX_FILE_BYTES {
                    return Err("arquivo excede o limite de 1 MiB".to_owned());
                }
                let mut contents = Vec::new();
                file.take(MAX_FILE_BYTES + 1)
                    .read_to_end(&mut contents)
                    .map_err(|error| format!("não foi possível ler o arquivo: {error}"))?;
                if contents.len() as u64 > MAX_FILE_BYTES {
                    return Err("arquivo excede o limite de 1 MiB".to_owned());
                }
                let contents = String::from_utf8_lossy(&contents);
                OpenPortugolFile::Read {
                    path,
                    lines: split_file_lines(&contents),
                    next_line: 0,
                    eof: false,
                }
            }
            portugol::FileAccessMode::Write => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        format!("não foi possível criar a pasta do arquivo: {error}")
                    })?;
                }
                let file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&path)
                    .map_err(|error| format!("não foi possível abrir para escrita: {error}"))?;
                OpenPortugolFile::Write {
                    path,
                    file,
                    bytes_written: 0,
                }
            }
            portugol::FileAccessMode::Append => {
                let bytes_written = fs::metadata(&path)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        format!("não foi possível criar a pasta do arquivo: {error}")
                    })?;
                }
                let file = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)
                    .map_err(|error| format!("não foi possível abrir para acrescentar: {error}"))?;
                OpenPortugolFile::Append {
                    path,
                    file,
                    bytes_written,
                }
            }
        };
        self.open_portugol_files.insert(handle, open_file);
        Ok(handle)
    }

    fn portugol_file_close(&mut self, handle: i64) -> Result<(), String> {
        match self.open_portugol_files.remove(&handle) {
            Some(OpenPortugolFile::Read { .. }) => Ok(()),
            Some(OpenPortugolFile::Write { mut file, .. })
            | Some(OpenPortugolFile::Append { mut file, .. }) => file
                .flush()
                .map_err(|error| format!("não foi possível fechar o arquivo: {error}")),
            None => Err("endereço de arquivo inválido".to_owned()),
        }
    }

    fn portugol_file_is_eof(&mut self, handle: i64) -> Result<bool, String> {
        match self.open_portugol_files.get(&handle) {
            Some(OpenPortugolFile::Read { eof, .. }) => Ok(*eof),
            Some(_) => Err("o arquivo não está aberto para leitura".to_owned()),
            None => Err("endereço de arquivo inválido".to_owned()),
        }
    }

    fn portugol_file_read_line(&mut self, handle: i64) -> Result<String, String> {
        match self.open_portugol_files.get_mut(&handle) {
            Some(OpenPortugolFile::Read {
                lines,
                next_line,
                eof,
                ..
            }) => {
                if let Some(line) = lines.get(*next_line).cloned() {
                    *next_line += 1;
                    Ok(line)
                } else {
                    *eof = true;
                    Ok(String::new())
                }
            }
            Some(_) => Err("o arquivo não está aberto para leitura".to_owned()),
            None => Err("endereço de arquivo inválido".to_owned()),
        }
    }

    fn portugol_file_write_line(&mut self, handle: i64, line: &str) -> Result<(), String> {
        let (file, bytes_written) = match self.open_portugol_files.get_mut(&handle) {
            Some(OpenPortugolFile::Write {
                file,
                bytes_written,
                ..
            })
            | Some(OpenPortugolFile::Append {
                file,
                bytes_written,
                ..
            }) => (file, bytes_written),
            Some(OpenPortugolFile::Read { .. }) => {
                return Err("o arquivo não está aberto para escrita".to_owned());
            }
            None => return Err("endereço de arquivo inválido".to_owned()),
        };
        let line = line.replace(['\r', '\n'], "");
        let line_bytes = u64::try_from(line.len())
            .unwrap_or(u64::MAX)
            .saturating_add(PORTUGOL_FILE_LINE_ENDING.len() as u64);
        if bytes_written.saturating_add(line_bytes) > MAX_FILE_BYTES {
            return Err("arquivo excede o limite de 1 MiB".to_owned());
        }
        file.write_all(line.as_bytes())
            .and_then(|()| file.write_all(PORTUGOL_FILE_LINE_ENDING.as_bytes()))
            .map_err(|error| format!("não foi possível gravar a linha: {error}"))?;
        *bytes_written += line_bytes;
        Ok(())
    }

    fn portugol_file_replace_text(
        &mut self,
        path: &str,
        search: &str,
        replacement: &str,
        only_first: bool,
    ) -> Result<(), String> {
        let path = self.safe_data_path(path)?;
        if self
            .open_portugol_files
            .values()
            .any(|file| file.path() == path)
        {
            return Err("feche o arquivo antes de substituir o texto".to_owned());
        }
        let bytes =
            fs::read(&path).map_err(|error| format!("não foi possível ler o arquivo: {error}"))?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("arquivo excede o limite de 1 MiB".to_owned());
        }
        let source: String = bytes.into_iter().map(char::from).collect();
        let result = replace_file_text_regex(&source, search, replacement, only_first)?;
        let output = result
            .chars()
            .map(|character| {
                u8::try_from(character as u32)
                    .map_err(|_| "o texto não pode ser representado em ISO-8859-1".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        fs::write(path, output)
            .map_err(|error| format!("não foi possível gravar o arquivo: {error}"))
    }

    fn portugol_http_request(
        &mut self,
        request: portugol::HttpRequest,
    ) -> Result<portugol::HttpResponse, String> {
        self.http_request(request)
    }

    fn portugol_file_write_bytes(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > MAX_HTTP_BODY_BYTES {
            return Err("arquivo baixado excede o limite de 10 MiB".to_owned());
        }
        let path = self.safe_data_path(path)?;
        if self
            .open_portugol_files
            .values()
            .any(|file| file.path() == path)
        {
            return Err("feche o arquivo antes de gravar os dados baixados".to_owned());
        }
        fs::write(path, bytes)
            .map_err(|error| format!("não foi possível gravar o arquivo baixado: {error}"))
    }

    fn portugol_file_write_image(
        &mut self,
        path: &str,
        bytes: &[u8],
        format: &str,
    ) -> Result<(), String> {
        if bytes.len() > MAX_HTTP_BODY_BYTES {
            return Err("imagem baixada excede o limite de 10 MiB".to_owned());
        }
        let path = self.safe_data_path(path)?;
        if self
            .open_portugol_files
            .values()
            .any(|file| file.path() == path)
        {
            return Err("feche o arquivo antes de gravar a imagem baixada".to_owned());
        }
        let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_read(Cursor::new(bytes.to_vec()))
            .map_err(|error| format!("não foi possível decodificar a imagem: {error}"))?;
        pixbuf
            .savev(&path, format, &[])
            .map_err(|error| format!("não foi possível salvar a imagem: {error}"))
    }

    fn portugol_key_pressed(&mut self, key: i64) -> Result<bool, String> {
        match self.request_host(HostRequest::KeyboardKeyPressed(key))? {
            portugol::HostValue::Logical(value) => Ok(value),
            _ => Err("resposta inválida ao consultar o teclado".to_owned()),
        }
    }

    fn portugol_any_key_pressed(&mut self) -> Result<bool, String> {
        match self.request_host(HostRequest::KeyboardAnyKeyPressed)? {
            portugol::HostValue::Logical(value) => Ok(value),
            _ => Err("resposta inválida ao consultar o teclado".to_owned()),
        }
    }

    fn portugol_read_key(&mut self) -> Result<i64, String> {
        match self.request_host(HostRequest::KeyboardReadKey)? {
            portugol::HostValue::Integer(value) => Ok(value),
            _ => Err("resposta inválida ao ler o teclado".to_owned()),
        }
    }

    fn portugol_mouse_button_pressed(&mut self, button: i64) -> Result<bool, String> {
        match self.request_host(HostRequest::MouseButtonPressed(button))? {
            portugol::HostValue::Logical(value) => Ok(value),
            _ => Err("resposta inválida ao consultar o mouse".to_owned()),
        }
    }

    fn portugol_any_mouse_button_pressed(&mut self) -> Result<bool, String> {
        match self.request_host(HostRequest::MouseAnyButtonPressed)? {
            portugol::HostValue::Logical(value) => Ok(value),
            _ => Err("resposta inválida ao consultar o mouse".to_owned()),
        }
    }

    fn portugol_read_mouse_button(&mut self) -> Result<i64, String> {
        match self.request_host(HostRequest::MouseReadButton)? {
            portugol::HostValue::Integer(value) => Ok(value),
            _ => Err("resposta inválida ao ler o mouse".to_owned()),
        }
    }

    fn portugol_mouse_position(&mut self) -> Result<(i64, i64), String> {
        match self.request_host(HostRequest::MousePosition)? {
            portugol::HostValue::Array(values) if values.len() == 2 => {
                match (&values[0], &values[1]) {
                    (portugol::HostValue::Integer(x), portugol::HostValue::Integer(y)) => {
                        Ok((*x, *y))
                    }
                    _ => Err("resposta inválida ao ler a posição do mouse".to_owned()),
                }
            }
            _ => Err("resposta inválida ao ler a posição do mouse".to_owned()),
        }
    }

    fn portugol_mouse_set_cursor_visible(&mut self, visible: bool) -> Result<(), String> {
        match self.request_host(HostRequest::MouseSetCursorVisible(visible))? {
            portugol::HostValue::Null => Ok(()),
            _ => Err("resposta inválida ao alterar o cursor do mouse".to_owned()),
        }
    }

    fn portugol_graphics(
        &mut self,
        function: &str,
        arguments: &[portugol::HostValue],
    ) -> Result<portugol::HostValue, String> {
        self.request_host(HostRequest::Graphics {
            function: function.to_owned(),
            arguments: arguments.to_vec(),
        })
    }

    fn portugol_audio(
        &mut self,
        function: &str,
        arguments: &[portugol::HostValue],
    ) -> Result<portugol::HostValue, String> {
        let command = crate::languages::portugol::studio_20::audio::command(function, arguments)?;
        self.request_audio_host(command)
            .map(crate::languages::portugol::studio_20::audio::response)
    }

    fn portugol_file_exists(&mut self, path: &str) -> Result<bool, String> {
        match self.safe_data_path(path) {
            Ok(path) => Ok(path.is_file()),
            Err(error) if error.starts_with("pasta do arquivo indisponível") => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn portugol_file_delete(&mut self, path: &str) -> Result<(), String> {
        let path = self.safe_data_path(path)?;
        if self
            .open_portugol_files
            .values()
            .any(|file| file.path() == path)
        {
            return Err("feche o arquivo antes de apagá-lo".to_owned());
        }
        if !path.is_file() {
            return Err("o caminho não é um arquivo regular".to_owned());
        }
        fs::remove_file(path).map_err(|error| format!("não foi possível apagar o arquivo: {error}"))
    }

    fn portugol_file_select(
        &mut self,
        extensions: &[String],
        allow_all_files: bool,
    ) -> Result<Option<String>, String> {
        if self.project_root.is_none() {
            return Err("abra um projeto para selecionar arquivos".to_owned());
        }
        for format in extensions {
            let Some((description, suffixes)) = format.split_once('|') else {
                return Err(format!("formato de arquivo inválido: '{format}'"));
            };
            if description.trim().is_empty() || suffixes.trim().is_empty() {
                return Err(format!("formato de arquivo inválido: '{format}'"));
            }
        }
        self.flush_output();
        self.events
            .send(ExecutionEvent::FileSelectionRequested {
                extensions: extensions.to_vec(),
                allow_all_files,
            })
            .map_err(|_| "a janela da IDE foi fechada".to_owned())?;
        let path = loop {
            match self.input.recv_timeout(Duration::from_millis(40)) {
                Ok(path) => break path,
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("a seleção do arquivo foi cancelada".to_owned());
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.poll_stop() {
                        self.stopped = true;
                        return Err(STOP_SENTINEL.to_owned());
                    }
                }
            }
        };
        if path.trim().is_empty() {
            return Ok(None);
        }
        if !portugol_file_selection_matches(&path, extensions, allow_all_files) {
            return Err("o arquivo selecionado não corresponde aos formatos aceitos".to_owned());
        }
        self.safe_data_path(&path)?;
        Ok(Some(path))
    }

    fn portugol_list_directories(&mut self, path: &str) -> Result<Vec<String>, String> {
        let path = self.safe_data_path(path)?;
        if !path.is_dir() {
            return Err("o caminho informado não é uma pasta".to_owned());
        }
        let mut directories = fs::read_dir(path)
            .map_err(|error| format!("não foi possível listar a pasta: {error}"))?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|kind| kind.is_dir())
                    .map(|_| entry.file_name().to_string_lossy().into_owned())
            })
            .collect::<Vec<_>>();
        directories.sort();
        Ok(directories)
    }

    fn portugol_list_files(
        &mut self,
        path: &str,
        extensions: Option<&[String]>,
    ) -> Result<Vec<String>, String> {
        let path = self.safe_data_path(path)?;
        if !path.is_dir() {
            return Err("o caminho informado não é uma pasta".to_owned());
        }
        let mut files = fs::read_dir(path)
            .map_err(|error| format!("não foi possível listar a pasta: {error}"))?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|kind| kind.is_file())
                    .map(|_| entry.file_name().to_string_lossy().into_owned())
            })
            .filter(|name| {
                extensions.is_none_or(|extensions| {
                    extensions
                        .iter()
                        .any(|extension| name.to_lowercase().ends_with(&extension.to_lowercase()))
                })
            })
            .collect::<Vec<_>>();
        files.sort();
        Ok(files)
    }

    fn portugol_create_directory(&mut self, path: &str) -> Result<(), String> {
        let path = self.safe_data_path(path)?;
        fs::create_dir_all(path).map_err(|error| format!("não foi possível criar a pasta: {error}"))
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_statement(
        &mut self,
        point: &portugol::ExecutionPoint,
    ) -> Result<portugol::ControlAction, String> {
        let action = self.await_action(Self::execution_snapshot(
            point.location.line,
            point.location.column,
            point
                .variables
                .iter()
                .map(|variable| (variable.name.clone(), variable.value.clone())),
            None,
        ))?;
        if action == ControlAction::Continue {
            self.debugging = false;
        }
        Ok(match action {
            ControlAction::Continue => portugol::ControlAction::Continue,
            ControlAction::Step => portugol::ControlAction::Step,
            ControlAction::Stop => portugol::ControlAction::Stop,
        })
    }
}

fn convert_portugol_diagnostics(
    diagnostics: Vec<portugol::Diagnostic>,
) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                portugol::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                portugol::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                portugol::DiagnosticKind::Type => DiagnosticKind::Type,
                portugol::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
}

pub(crate) fn execute(source: &str, io: &mut WorkerIo) -> Result<(), Vec<ExecutionDiagnostic>> {
    portugol::execute(source, io).map_err(convert_portugol_diagnostics)
}
