use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use gtk::gio::prelude::*;
use gtk::glib::{Regex, RegexCompileFlags, RegexMatchFlags};
use potoo_portugol_interpreter as portugol;
use potoo_visualg_interpreter as visualg;

use crate::components::content_panel::Language;

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const OUTPUT_LIMIT_MESSAGE: &str = "\n[saída truncada: limite de 1 MiB]\n";
const STOP_SENTINEL: &str = "__potoo_execution_stopped__";

fn replace_file_text_regex(
    source: &str,
    pattern: &str,
    replacement: &str,
    only_first: bool,
) -> Result<String, String> {
    let regex = Regex::new(
        pattern,
        RegexCompileFlags::empty(),
        RegexMatchFlags::empty(),
    )
    .map_err(|error| format!("expressão regular inválida: {error}"))?
    .ok_or_else(|| "não foi possível compilar a expressão regular".to_owned())?;
    let replacement = java_replacement_for_glib(replacement, &regex)?;
    let source = gtk::glib::GString::from_string_checked(source.to_owned())
        .map_err(|_| "não é possível processar arquivos com byte nulo".to_owned())?;
    let match_info = regex
        .match_(source.as_gstr(), RegexMatchFlags::empty())
        .map_err(|error| format!("falha ao pesquisar o arquivo: {error}"))?;

    let mut output = String::with_capacity(source.len().min(MAX_FILE_BYTES as usize));
    let mut output_bytes = 0usize;
    let mut previous_end = 0usize;
    let match_info = match_info;
    while match_info.matches() {
        let (start, end) = match_info
            .fetch_pos(0)
            .ok_or_else(|| "a expressão regular retornou uma posição inválida".to_owned())?;
        let start = usize::try_from(start)
            .map_err(|_| "a expressão regular retornou uma posição inválida".to_owned())?;
        let end = usize::try_from(end)
            .map_err(|_| "a expressão regular retornou uma posição inválida".to_owned())?;
        append_latin1_bounded(&mut output, &mut output_bytes, &source[previous_end..start])?;
        let expanded = match_info
            .expand_references(&replacement)
            .map_err(|error| format!("referência de substituição inválida: {error}"))?
            .ok_or_else(|| "referência de substituição inválida".to_owned())?;
        append_latin1_bounded(&mut output, &mut output_bytes, expanded.as_str())?;
        previous_end = end;
        if only_first || !match_info.next().map_err(|error| error.to_string())? {
            break;
        }
    }
    append_latin1_bounded(&mut output, &mut output_bytes, &source[previous_end..])?;
    Ok(output)
}

fn java_replacement_for_glib(replacement: &str, regex: &Regex) -> Result<String, String> {
    let mut output = String::with_capacity(replacement.len());
    let mut characters = replacement.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                if let Some(escaped) = characters.next() {
                    output.push(escaped);
                } else {
                    return Err("barra invertida sem caractere para escapar".to_owned());
                }
            }
            '$' => match characters.peek().copied() {
                Some('{') => {
                    characters.next();
                    let mut name = String::new();
                    let mut closed = false;
                    for next in characters.by_ref() {
                        if next == '}' {
                            closed = true;
                            break;
                        }
                        name.push(next);
                    }
                    if !closed || name.is_empty() || regex.string_number(&name) < 0 {
                        return Err(format!("grupo nomeado '${{{name}}}' não existe"));
                    }
                    output.push_str("\\g<");
                    output.push_str(&name);
                    output.push('>');
                }
                Some(first @ '0'..='9') => {
                    let mut group = first.to_digit(10).unwrap_or(0) as usize;
                    characters.next();
                    while let Some(next @ '0'..='9') = characters.peek().copied() {
                        let candidate = group
                            .saturating_mul(10)
                            .saturating_add(next.to_digit(10).unwrap_or(0) as usize);
                        if candidate > regex.capture_count() as usize {
                            break;
                        }
                        group = candidate;
                        characters.next();
                    }
                    if group > regex.capture_count() as usize {
                        return Err(format!("grupo '${group}' não existe"));
                    }
                    output.push('\\');
                    output.push_str(&group.to_string());
                }
                _ => return Err("referência de grupo inválida na substituição".to_owned()),
            },
            character => output.push(character),
        }
    }
    Ok(output)
}

fn append_latin1_bounded(
    output: &mut String,
    output_bytes: &mut usize,
    text: &str,
) -> Result<(), String> {
    for character in text.chars() {
        if character as u32 > u8::MAX as u32 {
            return Err("o texto não pode ser representado em ISO-8859-1".to_owned());
        }
        *output_bytes = output_bytes
            .checked_add(1)
            .ok_or_else(|| "arquivo excede o limite de 1 MiB".to_owned())?;
        if *output_bytes > MAX_FILE_BYTES as usize {
            return Err("arquivo excede o limite de 1 MiB".to_owned());
        }
        output.push(character);
    }
    Ok(())
}

const MAX_HTTP_BODY_BYTES: usize = 10 * 1024 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;

fn send_http_request(mut request: portugol::HttpRequest) -> Result<portugol::HttpResponse, String> {
    if !(1..=30_000).contains(&request.timeout_millis) {
        return Err("o tempo limite HTTP deve estar entre 1 e 30000 milissegundos".to_owned());
    }
    if request.body.len() > MAX_HTTP_BODY_BYTES {
        return Err("o corpo da requisição HTTP excede 10 MiB".to_owned());
    }
    let method = request.method.to_ascii_uppercase();
    if !matches!(method.as_str(), "GET" | "HEAD" | "DELETE" | "POST" | "PUT") {
        return Err("método HTTP não suportado".to_owned());
    }
    request.method = method;

    for redirect in 0..=5 {
        let uri = gtk::glib::Uri::parse(&request.url, gtk::glib::UriFlags::ENCODED)
            .map_err(|error| format!("endereço HTTP inválido: {error}"))?;
        let scheme = uri.scheme().to_ascii_lowercase();
        let secure = match scheme.as_str() {
            "http" => false,
            "https" => true,
            _ => return Err("o endereço deve usar HTTP ou HTTPS".to_owned()),
        };
        let host = uri
            .host()
            .map(|host| host.to_string())
            .filter(|host| !host.is_empty())
            .ok_or_else(|| "o endereço HTTP não informa um servidor".to_owned())?;
        if uri.userinfo().is_some() {
            return Err("credenciais embutidas no endereço não são aceitas".to_owned());
        }
        let port = if uri.port() >= 0 {
            u16::try_from(uri.port()).map_err(|_| "porta HTTP inválida".to_owned())?
        } else if secure {
            443
        } else {
            80
        };
        let timeout_seconds = request.timeout_millis.div_ceil(1000).clamp(1, 30) as u32;
        let client = gtk::gio::SocketClient::new();
        client.set_timeout(timeout_seconds);
        client.set_tls(secure);
        let connection = client
            .connect_to_uri(&request.url, port, None::<&gtk::gio::Cancellable>)
            .map_err(|error| format!("não foi possível conectar ao endereço HTTP: {error}"))?;
        connection.socket().set_timeout(timeout_seconds);

        let mut target = uri.path().to_string();
        if target.is_empty() {
            target.push('/');
        }
        if let Some(query) = uri.query() {
            target.push('?');
            target.push_str(query.as_str());
        }
        if target.contains(['\r', '\n', ' ']) {
            return Err("o caminho do endereço contém caracteres inválidos".to_owned());
        }
        let display_host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host.clone()
        };
        let default_port = if secure { 443 } else { 80 };
        let host_header = if port == default_port {
            display_host
        } else {
            format!("{display_host}:{port}")
        };

        let mut wire_request = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: PotooIDE/0.1\r\nAccept-Encoding: identity\r\nConnection: close\r\n",
            request.method, target, host_header
        );
        let mut has_content_type = false;
        for (name, value) in &request.headers {
            if !is_http_header_name(name)
                || value.contains(['\r', '\n'])
                || name.eq_ignore_ascii_case("host")
                || name.eq_ignore_ascii_case("content-length")
                || name.eq_ignore_ascii_case("transfer-encoding")
                || name.eq_ignore_ascii_case("connection")
            {
                return Err("cabeçalho HTTP inválido ou reservado".to_owned());
            }
            if name.eq_ignore_ascii_case("content-type") {
                has_content_type = true;
            }
            wire_request.push_str(name);
            wire_request.push_str(": ");
            wire_request.push_str(value);
            wire_request.push_str("\r\n");
        }
        if !request.body.is_empty() {
            if !has_content_type {
                wire_request.push_str("Content-Type: application/octet-stream\r\n");
            }
            wire_request.push_str(&format!("Content-Length: {}\r\n", request.body.len()));
        }
        wire_request.push_str("\r\n");
        connection
            .output_stream()
            .write_all(wire_request.as_bytes(), None::<&gtk::gio::Cancellable>)
            .map_err(|error| format!("não foi possível enviar a requisição HTTP: {error}"))?;
        if !request.body.is_empty() {
            connection
                .output_stream()
                .write_all(&request.body, None::<&gtk::gio::Cancellable>)
                .map_err(|error| format!("não foi possível enviar o corpo HTTP: {error}"))?;
        }

        let response = read_http_response(&connection, &request.method)?;
        if matches!(response.status, 301 | 302 | 303 | 307 | 308)
            && matches!(request.method.as_str(), "GET" | "HEAD")
        {
            let Some((_, location)) = response
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("location"))
            else {
                return Ok(response);
            };
            if redirect == 5 {
                return Err("o endereço excedeu o limite de cinco redirecionamentos".to_owned());
            }
            let redirected = uri
                .parse_relative(location, gtk::glib::UriFlags::ENCODED)
                .map_err(|error| format!("redirecionamento HTTP inválido: {error}"))?;
            let old_origin = http_origin(&uri);
            let new_origin = http_origin(&redirected);
            if old_origin != new_origin {
                request.headers.retain(|(name, _)| {
                    !name.eq_ignore_ascii_case("authorization")
                        && !name.eq_ignore_ascii_case("cookie")
                });
            }
            request.url = redirected.to_string();
            continue;
        }
        return Ok(response);
    }
    Err("o endereço excedeu o limite de redirecionamentos".to_owned())
}

fn is_http_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

fn http_origin(uri: &gtk::glib::Uri) -> (String, String, i32) {
    (
        uri.scheme().to_ascii_lowercase(),
        uri.host()
            .map(|host| host.to_ascii_lowercase())
            .unwrap_or_default(),
        uri.port(),
    )
}

fn read_http_response(
    connection: &gtk::gio::SocketConnection,
    method: &str,
) -> Result<portugol::HttpResponse, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = connection
            .input_stream()
            .read(&mut buffer, None::<&gtk::gio::Cancellable>)
            .map_err(|error| format!("não foi possível ler a resposta HTTP: {error}"))?;
        if count == 0 {
            break;
        }
        let next_size = bytes.len().saturating_add(count);
        if next_size > MAX_HTTP_BODY_BYTES + MAX_HTTP_HEADER_BYTES {
            return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    parse_http_response(&bytes, method)
}

fn parse_http_response(bytes: &[u8], method: &str) -> Result<portugol::HttpResponse, String> {
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "resposta HTTP sem cabeçalho completo".to_owned())?;
    if header_end > MAX_HTTP_HEADER_BYTES {
        return Err("o cabeçalho HTTP excede 64 KiB".to_owned());
    }
    let header_text = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| "o cabeçalho HTTP não é texto válido".to_owned())?;
    let mut lines = header_text.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| "resposta HTTP sem código de status".to_owned())?;
    let status = status_line
        .split_ascii_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "resposta HTTP com código inválido".to_owned())?;
    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }
    let raw_body = &bytes[header_end + 4..];
    let transfer_encoding = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding"))
        .flat_map(|(_, value)| value.split(','))
        .map(str::trim)
        .collect::<Vec<_>>();
    let is_chunked =
        transfer_encoding.len() == 1 && transfer_encoding[0].eq_ignore_ascii_case("chunked");
    if !transfer_encoding.is_empty() && !is_chunked {
        return Err("codificação da resposta HTTP não suportada".to_owned());
    }
    let content_lengths = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| {
            value
                .parse::<usize>()
                .map_err(|_| "Content-Length inválido na resposta HTTP".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if is_chunked && !content_lengths.is_empty() {
        return Err("a resposta HTTP combina Transfer-Encoding e Content-Length".to_owned());
    }
    if content_lengths
        .iter()
        .any(|length| *length != content_lengths[0])
    {
        return Err("a resposta HTTP contém valores Content-Length conflitantes".to_owned());
    }
    let body = if method == "HEAD" {
        Vec::new()
    } else if is_chunked {
        decode_chunked_http_body(raw_body)?
    } else if let Some(length) = content_lengths.first().copied() {
        if length > MAX_HTTP_BODY_BYTES {
            return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
        }
        if length > raw_body.len() {
            return Err("a resposta HTTP terminou antes do Content-Length".to_owned());
        }
        raw_body[..length].to_vec()
    } else {
        if raw_body.len() > MAX_HTTP_BODY_BYTES {
            return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
        }
        raw_body.to_vec()
    };
    Ok(portugol::HttpResponse {
        status,
        headers,
        body,
    })
}

fn decode_chunked_http_body(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let mut position = 0usize;
    loop {
        let line_end = bytes[position..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| position + offset)
            .ok_or_else(|| "resposta HTTP em blocos inválida".to_owned())?;
        let size = std::str::from_utf8(&bytes[position..line_end])
            .ok()
            .and_then(|line| line.split(';').next())
            .and_then(|size| usize::from_str_radix(size.trim(), 16).ok())
            .ok_or_else(|| "resposta HTTP em blocos inválida".to_owned())?;
        position = line_end + 2;
        if size == 0 {
            loop {
                let trailer_end = bytes[position..]
                    .windows(2)
                    .position(|window| window == b"\r\n")
                    .map(|offset| position + offset)
                    .ok_or_else(|| "rodapé da resposta HTTP em blocos inválido".to_owned())?;
                if trailer_end == position {
                    return Ok(body);
                }
                if !bytes[position..trailer_end].contains(&b':') {
                    return Err("rodapé da resposta HTTP em blocos inválido".to_owned());
                }
                position = trailer_end + 2;
            }
        }
        let end = position
            .checked_add(size)
            .ok_or_else(|| "resposta HTTP excede o limite".to_owned())?;
        let chunk_end = end
            .checked_add(2)
            .ok_or_else(|| "resposta HTTP excede o limite".to_owned())?;
        if chunk_end > bytes.len() || bytes[end..chunk_end] != *b"\r\n" {
            return Err("resposta HTTP em blocos incompleta".to_owned());
        }
        if body.len().saturating_add(size) > MAX_HTTP_BODY_BYTES {
            return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
        }
        body.extend_from_slice(&bytes[position..end]);
        position = chunk_end;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagnosticKind {
    Lexical,
    Syntax,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionDiagnostic {
    pub(crate) kind: DiagnosticKind,
    pub(crate) message: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InputPrompt {
    pub(crate) variable_name: Option<String>,
    pub(crate) data_type: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DebugPauseReason {
    Step,
    Pause,
    Debug,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DebuggerSnapshot {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) variables: Vec<(String, String)>,
    pub(crate) pause_reason: Option<DebugPauseReason>,
}

#[derive(Debug)]
pub(crate) enum HostRequest {
    KeyboardKeyPressed(i64),
    KeyboardAnyKeyPressed,
    KeyboardReadKey,
    MouseButtonPressed(i64),
    MouseAnyButtonPressed,
    MouseReadButton,
    MousePosition,
    MouseSetCursorVisible(bool),
    Graphics {
        function: String,
        arguments: Vec<portugol::HostValue>,
    },
    Audio {
        function: String,
        arguments: Vec<portugol::HostValue>,
    },
}

#[derive(Debug)]
pub(crate) enum ExecutionEvent {
    Output(String),
    ClearOutput,
    InputRequested(InputPrompt),
    FileSelectionRequested {
        extensions: Vec<String>,
        allow_all_files: bool,
    },
    HostRequest {
        request: HostRequest,
        response: Sender<Result<portugol::HostValue, String>>,
    },
    Debugger(DebuggerSnapshot),
    Finished {
        result: Result<(), Vec<ExecutionDiagnostic>>,
        stopped: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControlAction {
    Continue,
    Step,
    Stop,
}

pub(crate) struct RunningProgram {
    pub(crate) events: Receiver<ExecutionEvent>,
    pub(crate) input: Sender<String>,
    pub(crate) control: Sender<ControlAction>,
}

struct WorkerIo {
    events: Sender<ExecutionEvent>,
    input: Receiver<String>,
    control: Receiver<ControlAction>,
    pending_action: Option<ControlAction>,
    debugging: bool,
    project_root: Option<PathBuf>,
    output_bytes: usize,
    output_truncated: bool,
    stopped: bool,
    open_portugol_files: HashMap<i64, OpenPortugolFile>,
}

enum OpenPortugolFile {
    Read {
        path: PathBuf,
        lines: Vec<String>,
        next_line: usize,
    },
    Write {
        path: PathBuf,
        file: File,
        bytes_written: u64,
    },
    Append {
        path: PathBuf,
        file: File,
        bytes_written: u64,
    },
}

impl OpenPortugolFile {
    fn path(&self) -> &Path {
        match self {
            Self::Read { path, .. } | Self::Write { path, .. } | Self::Append { path, .. } => path,
        }
    }
}

impl WorkerIo {
    fn request_host(&mut self, request: HostRequest) -> Result<portugol::HostValue, String> {
        let (sender, response) = mpsc::channel();
        self.events
            .send(ExecutionEvent::HostRequest {
                request,
                response: sender,
            })
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

    fn send_input_prompt(&mut self, prompt: InputPrompt) -> Result<String, String> {
        self.events
            .send(ExecutionEvent::InputRequested(prompt))
            .map_err(|_| "a janela da IDE foi fechada".to_owned())?;
        loop {
            match self.input.recv_timeout(Duration::from_millis(40)) {
                Ok(input) => return Ok(input),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("a entrada do programa foi cancelada".to_owned());
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

    fn poll_stop(&mut self) -> bool {
        loop {
            match self.control.try_recv() {
                Ok(ControlAction::Stop) => return true,
                Ok(action) => self.pending_action = Some(action),
                Err(_) => return false,
            }
        }
    }

    fn await_action(&mut self, snapshot: DebuggerSnapshot) -> Result<ControlAction, String> {
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

    fn write_limited(&mut self, text: &str) {
        if self.output_truncated {
            return;
        }
        let remaining = MAX_OUTPUT_BYTES.saturating_sub(self.output_bytes);
        let mut end = text.len().min(remaining);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end > 0 {
            let _ = self
                .events
                .send(ExecutionEvent::Output(text[..end].to_owned()));
            self.output_bytes += end;
        }
        if end < text.len() {
            let _ = self
                .events
                .send(ExecutionEvent::Output(OUTPUT_LIMIT_MESSAGE.to_owned()));
            self.output_truncated = true;
        }
    }

    fn execution_snapshot(
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

    fn safe_data_path(&self, relative: &str) -> Result<PathBuf, String> {
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

impl portugol::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: None,
            data_type: None,
        })
    }

    fn clear_output(&mut self) {
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
                let mut contents = String::new();
                file.take(MAX_FILE_BYTES + 1)
                    .read_to_string(&mut contents)
                    .map_err(|error| format!("não foi possível ler o arquivo: {error}"))?;
                if contents.len() as u64 > MAX_FILE_BYTES {
                    return Err("arquivo excede o limite de 1 MiB".to_owned());
                }
                OpenPortugolFile::Read {
                    path,
                    lines: contents.lines().map(str::to_owned).collect(),
                    next_line: 0,
                }
            }
            portugol::FileAccessMode::Write => {
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
            Some(OpenPortugolFile::Read {
                lines, next_line, ..
            }) => Ok(*next_line >= lines.len()),
            Some(_) => Err("o arquivo não está aberto para leitura".to_owned()),
            None => Err("endereço de arquivo inválido".to_owned()),
        }
    }

    fn portugol_file_read_line(&mut self, handle: i64) -> Result<String, String> {
        match self.open_portugol_files.get_mut(&handle) {
            Some(OpenPortugolFile::Read {
                lines, next_line, ..
            }) => {
                let line = lines
                    .get(*next_line)
                    .cloned()
                    .ok_or_else(|| "não há mais linhas para ler".to_owned())?;
                *next_line += 1;
                Ok(line)
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
        let line_bytes = u64::try_from(line.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        if bytes_written.saturating_add(line_bytes) > MAX_FILE_BYTES {
            return Err("arquivo excede o limite de 1 MiB".to_owned());
        }
        writeln!(file, "{line}")
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
        send_http_request(request)
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
        self.request_host(HostRequest::Audio {
            function: function.to_owned(),
            arguments: arguments.to_vec(),
        })
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
        if !allow_all_files
            && !extensions.is_empty()
            && !extensions.iter().any(|format| {
                format
                    .split_once('|')
                    .map_or(format.as_str(), |(_, extensions)| extensions)
                    .split(',')
                    .any(|extension| {
                        let extension = extension.trim().trim_start_matches('.');
                        extension == "*"
                            || path
                                .to_lowercase()
                                .ends_with(&format!(".{}", extension.to_lowercase()))
                    })
            })
        {
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
        Ok(match action {
            ControlAction::Continue => portugol::ControlAction::Continue,
            ControlAction::Step => portugol::ControlAction::Step,
            ControlAction::Stop => portugol::ControlAction::Stop,
        })
    }
}

impl visualg::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &visualg::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: Some(prompt.variable_name.clone()),
            data_type: Some(
                match prompt.data_type {
                    visualg::DataType::Integer => "inteiro",
                    visualg::DataType::Real => "real",
                    visualg::DataType::Character => "caractere",
                    visualg::DataType::Logical => "logico",
                }
                .to_owned(),
            ),
        })
    }

    fn clear_output(&mut self) {
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

    fn read_data_file(&mut self, path: &str) -> Result<Option<Vec<String>>, String> {
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

    fn create_data_file(&mut self, path: &str) -> Result<(), String> {
        let path = self.safe_data_path(path)?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map(|_| ())
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(format!("não foi possível criar o arquivo: {error}"))
                }
            })
    }

    fn append_data_file_line(&mut self, path: &str, line: &str) -> Result<(), String> {
        let path = self.safe_data_path(path)?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|error| format!("não foi possível gravar o arquivo: {error}"))?;
        writeln!(file, "{line}")
            .map_err(|error| format!("não foi possível gravar o arquivo: {error}"))
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        match self.control.recv_timeout(duration) {
            Ok(ControlAction::Stop) => {
                self.stopped = true;
                Err(STOP_SENTINEL.to_owned())
            }
            Ok(action) => {
                self.pending_action = Some(action);
                Ok(())
            }
            Err(RecvTimeoutError::Timeout) => Ok(()),
            Err(RecvTimeoutError::Disconnected) => {
                self.stopped = true;
                Err(STOP_SENTINEL.to_owned())
            }
        }
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &visualg::ExecutionEvent,
    ) -> Result<visualg::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            visualg::PauseReason::Step => DebugPauseReason::Step,
            visualg::PauseReason::Pause => DebugPauseReason::Pause,
            visualg::PauseReason::Debug => DebugPauseReason::Debug,
        });
        let action = self.await_action(Self::execution_snapshot(
            event.location.line,
            event.location.column,
            event
                .variables
                .iter()
                .map(|variable| (variable.name.clone(), variable.value.clone())),
            reason,
        ))?;
        Ok(match action {
            ControlAction::Continue => visualg::ControlAction::Continue,
            ControlAction::Step => visualg::ControlAction::Step,
            ControlAction::Stop => visualg::ControlAction::Stop,
        })
    }
}

pub(crate) fn start(
    source: String,
    language: Language,
    document_path: Option<PathBuf>,
    project_root: Option<PathBuf>,
    debugging: bool,
) -> RunningProgram {
    let (event_sender, events) = mpsc::channel();
    let (input, input_receiver) = mpsc::channel();
    let (control, control_receiver) = mpsc::channel();
    let root = project_root.or_else(|| {
        document_path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    });

    std::thread::spawn(move || {
        let mut io = WorkerIo {
            events: event_sender.clone(),
            input: input_receiver,
            control: control_receiver,
            pending_action: None,
            debugging,
            project_root: root,
            output_bytes: 0,
            output_truncated: false,
            stopped: false,
            open_portugol_files: HashMap::new(),
        };
        let result = match language {
            Language::Portugol => {
                portugol::execute(&source, &mut io).map_err(convert_portugol_diagnostics)
            }
            Language::Visualg => {
                visualg::execute(&source, &mut io).map_err(convert_visualg_diagnostics)
            }
        };
        let stopped = io.stopped
            || result.as_ref().is_err_and(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message == STOP_SENTINEL)
            });
        let result = if stopped { Ok(()) } else { result };
        let _ = event_sender.send(ExecutionEvent::Finished { result, stopped });
    });

    RunningProgram {
        events,
        input,
        control,
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
                portugol::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
}

fn convert_visualg_diagnostics(diagnostics: Vec<visualg::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                visualg::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                visualg::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                visualg::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::{
        ControlAction, ExecutionEvent, HostRequest, RunningProgram, STOP_SENTINEL, WorkerIo,
        decode_chunked_http_body, parse_http_response, portugol, replace_file_text_regex,
        send_http_request, start,
    };
    use crate::components::content_panel::Language;

    fn local_http_server(
        responses: Vec<String>,
    ) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener should bind");
        let address = listener.local_addr().unwrap();
        let (sender, requests) = mpsc::channel();
        let server = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("client should connect");
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let count = stream
                        .read(&mut buffer)
                        .expect("request should be readable");
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
                    else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
                sender
                    .send(String::from_utf8_lossy(&request).into_owned())
                    .unwrap();
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{address}"), requests, server)
    }

    #[test]
    fn host_http_client_sends_requests_and_decodes_chunked_responses() {
        let (url, requests, server) = local_http_server(vec![
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n4\r\nok!!\r\n0\r\n\r\n".to_owned(),
        ]);
        let response = send_http_request(portugol::HttpRequest {
            method: "POST".to_owned(),
            url: format!("{url}/api"),
            headers: vec![("X-Test".to_owned(), "yes".to_owned())],
            body: b"payload".to_vec(),
            timeout_millis: 2_000,
        })
        .expect("HTTP request should succeed");
        let request = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        server.join().unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"ok!!");
        assert!(request.starts_with("POST /api HTTP/1.1\r\n"));
        assert!(request.contains("Host: 127.0.0.1:"));
        assert!(request.contains("X-Test: yes\r\n"));
        assert!(request.ends_with("\r\n\r\npayload"));
    }

    #[test]
    fn host_http_client_follows_relative_get_redirects() {
        let (url, requests, server) = local_http_server(vec![
            "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
            "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nfinal".to_owned(),
        ]);
        let response = send_http_request(portugol::HttpRequest {
            method: "GET".to_owned(),
            url: format!("{url}/redirect"),
            headers: Vec::new(),
            body: Vec::new(),
            timeout_millis: 2_000,
        })
        .expect("GET redirect should succeed");
        let first_request = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        let second_request = requests.recv_timeout(Duration::from_secs(2)).unwrap();
        server.join().unwrap();

        assert_eq!(response.body, b"final");
        assert!(first_request.starts_with("GET /redirect HTTP/1.1\r\n"));
        assert!(second_request.starts_with("GET /final HTTP/1.1\r\n"));
    }

    #[test]
    fn host_http_parser_obeys_content_length_and_rejects_invalid_framing() {
        let response = parse_http_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabcNEXT",
            "GET",
        )
        .expect("valid response should parse");
        assert_eq!(response.body, b"abc");

        assert!(
            parse_http_response(b"HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\nabc", "GET")
                .is_err()
        );
        assert!(
            parse_http_response(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nabc", "GET").is_err()
        );
        assert!(
            parse_http_response(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\nabc",
                "GET"
            )
            .is_err()
        );
        assert!(parse_http_response(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Length: 4\r\n\r\n4\r\ntest\r\n0\r\n\r\n",
            "GET"
        )
        .is_err());
        assert!(
            parse_http_response(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, chunked\r\n\r\n0\r\n\r\n",
                "GET"
            )
            .is_err()
        );
        assert!(decode_chunked_http_body(b"0\r\n").is_err());
    }

    #[test]
    fn portugol_file_replacement_supports_java_regex_groups_and_first_match_mode() {
        assert_eq!(
            replace_file_text_regex("item 12, item 34", r"item (\d+)", "registro $1", false)
                .unwrap(),
            "registro 12, registro 34"
        );
        assert_eq!(
            replace_file_text_regex("item 12, item 34", r"item (\d+)", "primeiro $1", true)
                .unwrap(),
            "primeiro 12, item 34"
        );
        assert_eq!(
            replace_file_text_regex("id=42", r"(?<id>\d+)", "${id}\\$", false).unwrap(),
            "id=42$"
        );
    }

    #[test]
    fn portugol_file_replacement_rejects_invalid_patterns_and_references() {
        assert!(replace_file_text_regex("texto", "[", "x", false).is_err());
        assert!(replace_file_text_regex("texto", "texto", "$2", false).is_err());
        assert!(replace_file_text_regex("texto", "texto", "${inexistente}", false).is_err());
    }

    #[test]
    fn cancelling_a_portugol_wait_stops_without_waiting_for_its_deadline() {
        let (events, _event_receiver) = mpsc::channel();
        let (_input_sender, input) = mpsc::channel();
        let (control, control_receiver) = mpsc::channel();
        let mut io = WorkerIo {
            events,
            input,
            control: control_receiver,
            pending_action: None,
            debugging: false,
            project_root: None,
            output_bytes: 0,
            output_truncated: false,
            stopped: false,
            open_portugol_files: HashMap::new(),
        };
        control.send(ControlAction::Stop).unwrap();
        let started = std::time::Instant::now();

        let result = portugol::RuntimeIo::wait(&mut io, Duration::from_secs(30));

        assert_eq!(result, Err(STOP_SENTINEL.to_owned()));
        assert!(io.stopped);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    fn collect(
        program: RunningProgram,
        input: &[&str],
        controls: &[ControlAction],
    ) -> (String, Vec<ExecutionEvent>) {
        let mut output = String::new();
        let mut events = Vec::new();
        let mut next_input = input.iter();
        let mut next_control = controls.iter();
        loop {
            let event = program
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("execution event should arrive");
            match &event {
                ExecutionEvent::Output(text) => output.push_str(text),
                ExecutionEvent::ClearOutput => output.clear(),
                ExecutionEvent::InputRequested(_) => program
                    .input
                    .send(next_input.next().copied().unwrap_or_default().to_owned())
                    .unwrap(),
                ExecutionEvent::FileSelectionRequested { .. } => {
                    program.input.send(String::new()).unwrap()
                }
                ExecutionEvent::HostRequest { response, .. } => {
                    let _ =
                        response.send(Err("host de teste sem suporte a essa operação".to_owned()));
                }
                ExecutionEvent::Debugger(_) => program
                    .control
                    .send(*next_control.next().unwrap_or(&ControlAction::Continue))
                    .unwrap(),
                ExecutionEvent::Finished { result, .. } => {
                    result.as_ref().expect("program should finish");
                    events.push(event);
                    break;
                }
            }
            events.push(event);
        }
        (output, events)
    }

    #[test]
    fn streams_output_and_waits_for_portugol_input() {
        let source = r#"programa {
  funcao inicio() {
    inteiro valor
    leia(valor)
    escreva(valor * 2)
  }
}"#;
        let (output, events) = collect(
            start(source.into(), Language::Portugol, None, None, false),
            &["21"],
            &[],
        );
        assert_eq!(output, "42");
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutionEvent::InputRequested(_)))
        );
    }

    #[test]
    fn portugol_host_requests_wait_for_the_ui_response() {
        let source = r#"programa {
  inclua biblioteca Graficos --> g
  funcao inicio() {
    escreva(g.largura_janela())
  }
}"#;
        let program = start(source.to_owned(), Language::Portugol, None, None, false);
        let mut output = String::new();

        loop {
            match program
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("host operation event should arrive")
            {
                ExecutionEvent::HostRequest {
                    request:
                        HostRequest::Graphics {
                            function,
                            arguments,
                        },
                    response,
                } => {
                    assert_eq!(function, "largura_janela");
                    assert!(arguments.is_empty());
                    response
                        .send(Ok(portugol::HostValue::Integer(640)))
                        .unwrap();
                }
                ExecutionEvent::Output(text) => output.push_str(&text),
                ExecutionEvent::Finished { result, stopped } => {
                    result.expect("host response should let the program finish");
                    assert!(!stopped);
                    break;
                }
                event => panic!("unexpected execution event: {event:?}"),
            }
        }

        assert_eq!(output, "640");
    }

    #[test]
    fn stopping_interrupts_a_pending_portugol_host_request() {
        let source = r#"programa {
  inclua biblioteca Teclado --> t
  funcao inicio() {
    t.ler_tecla()
  }
}"#;
        let program = start(source.to_owned(), Language::Portugol, None, None, false);

        let event = program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("keyboard request should arrive");
        let ExecutionEvent::HostRequest {
            request: HostRequest::KeyboardReadKey,
            response: _,
        } = event
        else {
            panic!("expected a keyboard host request");
        };
        program.control.send(ControlAction::Stop).unwrap();

        let event = program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("stopping should finish the program");
        assert!(matches!(
            event,
            ExecutionEvent::Finished {
                result: Ok(()),
                stopped: true
            }
        ));
    }

    #[test]
    fn runs_visualg_voting_sample_for_both_inputs() {
        let source = include_str!("../tests/fixtures/visualg-voting.alg");
        for (input, expected) in [
            ("19", "Qual é sua idade? Pode votar.\n"),
            ("16", "Qual é sua idade? Ainda não pode votar.\n"),
        ] {
            let (output, _) = collect(
                start(source.into(), Language::Visualg, None, None, false),
                &[input],
                &[],
            );
            assert_eq!(output, expected);
        }
    }

    #[test]
    fn worker_handles_visualg_typed_prompt_debug_step_stop_and_disconnect() {
        let source = "algoritmo \"worker\"\nvar\nx: inteiro\ninicio\nleia(x)\nx <- x + 1\nescreva(x)\nfimalgoritmo\n";
        let program = start(source.into(), Language::Visualg, None, None, true);
        let mut saw_typed_prompt = false;
        let mut debugger_events = 0;
        loop {
            match program
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("event should arrive")
            {
                ExecutionEvent::InputRequested(prompt) => {
                    assert_eq!(prompt.variable_name.as_deref(), Some("x"));
                    assert_eq!(prompt.data_type.as_deref(), Some("inteiro"));
                    saw_typed_prompt = true;
                    program.input.send("4".into()).unwrap();
                }
                ExecutionEvent::Debugger(_) => {
                    debugger_events += 1;
                    program
                        .control
                        .send(if debugger_events == 1 {
                            ControlAction::Step
                        } else {
                            ControlAction::Stop
                        })
                        .unwrap();
                }
                ExecutionEvent::Finished { result, stopped } => {
                    result.expect("stopping should not become a runtime error");
                    assert!(stopped);
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_typed_prompt);
        assert!(debugger_events >= 2);
    }

    #[test]
    fn stopping_visualg_while_waiting_for_input_finishes_cleanly() {
        let source = "algoritmo \"cancelar leitura\"\nvar\nx: inteiro\ninicio\nleia(x)\nescreva(x)\nfimalgoritmo\n";
        let program = start(source.into(), Language::Visualg, None, None, false);

        let event = program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("the input prompt should arrive");
        assert!(matches!(event, ExecutionEvent::InputRequested(_)));
        program.control.send(ControlAction::Stop).unwrap();

        let event = program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("stopping the pending input should finish the program");
        assert!(matches!(
            event,
            ExecutionEvent::Finished {
                result: Ok(()),
                stopped: true
            }
        ));
    }

    #[test]
    fn stop_interrupts_cpu_bound_loops_in_both_languages() {
        let programs = [
            (
                Language::Portugol,
                "programa { funcao inicio() { enquanto (verdadeiro) { } } }",
            ),
            (
                Language::Visualg,
                "algoritmo \"infinito\"\ninicio\nenquanto verdadeiro faca\nfimenquanto\nfimalgoritmo\n",
            ),
        ];
        for (language, source) in programs {
            let program = start(source.into(), language, None, None, false);
            program.control.send(ControlAction::Stop).unwrap();
            let result = loop {
                match program
                    .events
                    .recv_timeout(Duration::from_secs(2))
                    .expect("a CPU-bound run should respond to stop")
                {
                    ExecutionEvent::Finished { result, stopped } => {
                        break (result, stopped);
                    }
                    _ => {}
                }
            };
            result
                .0
                .expect("stopping should not report a runtime error");
            assert!(result.1, "{language:?} should report a stopped run");
        }
    }

    #[test]
    fn visualg_data_files_stay_inside_project_root() {
        let root = std::env::temp_dir().join(format!("potoo-visualg-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let outside = root.with_extension("outside");
        fs::write(&outside, "unchanged").unwrap();
        let mut escaped_paths = vec![
            "../escape.txt".to_owned(),
            outside.to_string_lossy().into_owned(),
        ];
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            symlink(&outside, root.join("escape-link.txt")).unwrap();
            escaped_paths.push("escape-link.txt".to_owned());
        }
        for escaped_path in escaped_paths {
            let source =
                format!("algoritmo \"safe\"\narquivo \"{escaped_path}\"\ninicio\nfimalgoritmo\n");
            let program = start(source, Language::Visualg, None, Some(root.clone()), false);
            let failure = loop {
                match program
                    .events
                    .recv_timeout(Duration::from_secs(2))
                    .expect("event should arrive")
                {
                    ExecutionEvent::Finished { result, .. } => {
                        break result.err();
                    }
                    _ => {}
                }
            };
            let diagnostic = failure.unwrap().remove(0);
            assert!(diagnostic.message.contains("projeto"), "{diagnostic:?}");
        }
        assert_eq!(fs::read_to_string(&outside).unwrap(), "unchanged");
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn portugol_file_library_reads_and_writes_inside_the_project() {
        let root =
            std::env::temp_dir().join(format!("potoo-portugol-files-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = r#"programa {
  inclua biblioteca Arquivos --> a
  funcao inicio() {
    a.criar_pasta("subdir/deep")
    inteiro arquivo = a.abrir_arquivo("subdir/deep/dados.txt", a.MODO_ESCRITA)
    a.escrever_linha("olá", arquivo)
    a.fechar_arquivo(arquivo)
    arquivo = a.abrir_arquivo("subdir/deep/dados.txt", a.MODO_LEITURA)
    escreva(a.ler_linha(arquivo), "|", a.fim_arquivo(arquivo))
    a.fechar_arquivo(arquivo)
  }
}"#;

        let (output, _) = collect(
            start(
                source.into(),
                Language::Portugol,
                None,
                Some(root.clone()),
                false,
            ),
            &[],
            &[],
        );

        assert_eq!(output, "olá|verdadeiro");
        assert_eq!(
            fs::read_to_string(root.join("subdir/deep/dados.txt")).unwrap(),
            "olá\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn portugol_file_selection_uses_supported_extensions_and_returns_project_paths() {
        let root =
            std::env::temp_dir().join(format!("potoo-portugol-picker-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = r#"programa {
  inclua biblioteca Arquivos --> a
  funcao inicio() {
    cadeia formatos[1] = {"Texto|txt"}
    escreva(a.selecionar_arquivo(formatos, falso))
  }
}"#;
        let program = start(
            source.into(),
            Language::Portugol,
            None,
            Some(root.clone()),
            false,
        );
        let mut output = String::new();
        loop {
            match program
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("execution event should arrive")
            {
                ExecutionEvent::FileSelectionRequested {
                    extensions,
                    allow_all_files,
                } => {
                    assert_eq!(extensions, ["Texto|txt"]);
                    assert!(!allow_all_files);
                    program.input.send("dados.txt".to_owned()).unwrap();
                }
                ExecutionEvent::Output(text) => output.push_str(&text),
                ExecutionEvent::Finished { result, .. } => {
                    result.expect("the selected path should be returned");
                    break;
                }
                _ => {}
            }
        }
        assert_eq!(output, "dados.txt");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn portugol_file_library_lists_filters_replaces_checks_and_deletes() {
        let root =
            std::env::temp_dir().join(format!("potoo-portugol-file-ops-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = r#"programa {
  inclua biblioteca Arquivos --> a
  funcao inicio() {
    a.criar_pasta("pasta")
    inteiro arquivo = a.abrir_arquivo("a.txt", a.MODO_ESCRITA)
    a.escrever_linha("item 1 e item 2", arquivo)
    a.fechar_arquivo(arquivo)
    inteiro apagado = a.abrir_arquivo("b.alg", a.MODO_ESCRITA)
    a.fechar_arquivo(apagado)
    a.apagar_arquivo("b.alg")
    a.substituir_texto("a.txt", "item ([0-9]+)", "registro $1", falso)
    a.substituir_texto("a.txt", "registro ([0-9]+)", "primeiro $1", verdadeiro)
    cadeia pastas[2]
    cadeia arquivos[3]
    cadeia tipos[1] = {"txt"}
    a.listar_pastas(".", pastas)
    a.listar_arquivos_por_tipo(".", arquivos, tipos)
    escreva(a.arquivo_existe("a.txt"), "|", a.arquivo_existe("b.alg"), "|", pastas[0], "|", arquivos[0], "|")
    arquivo = a.abrir_arquivo("a.txt", a.MODO_LEITURA)
    escreva(a.ler_linha(arquivo))
    a.fechar_arquivo(arquivo)
  }
}"#;
        let (output, _) = collect(
            start(
                source.into(),
                Language::Portugol,
                None,
                Some(root.clone()),
                false,
            ),
            &[],
            &[],
        );

        assert_eq!(
            output,
            "verdadeiro|falso|pasta|a.txt|primeiro 1 e registro 2"
        );
        assert!(!root.join("b.alg").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn portugol_file_library_rejects_paths_that_leave_the_project() {
        let root = std::env::temp_dir().join(format!("potoo-portugol-safe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let outside = root.with_extension("outside");
        fs::write(&outside, "unchanged").unwrap();
        let mut paths = vec![
            "../escape.txt".to_owned(),
            outside.to_string_lossy().into_owned(),
        ];
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            symlink(&outside, root.join("escape-link.txt")).unwrap();
            paths.push("escape-link.txt".to_owned());
        }
        for path in paths {
            let source = format!(
                "programa {{\n inclua biblioteca Arquivos --> a\n funcao inicio() {{\n a.abrir_arquivo(\"{path}\", a.MODO_ESCRITA)\n }}\n}}"
            );
            let program = start(source, Language::Portugol, None, Some(root.clone()), false);
            let failure = loop {
                match program
                    .events
                    .recv_timeout(Duration::from_secs(2))
                    .expect("execution event should arrive")
                {
                    ExecutionEvent::Finished { result, .. } => break result.err(),
                    _ => {}
                }
            };
            let diagnostic = failure.unwrap().remove(0);
            assert!(diagnostic.message.contains("projeto"), "{diagnostic:?}");
        }
        assert_eq!(fs::read_to_string(&outside).unwrap(), "unchanged");
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(root);
    }
}
