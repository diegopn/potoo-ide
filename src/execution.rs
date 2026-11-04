use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::time::{Duration, Instant};

use gtk::gio::prelude::*;
use gtk::glib::{Regex, RegexCompileFlags, RegexMatchFlags};
use potoo_calango_interpreter as calango;
use potoo_gportugol_interpreter as gportugol;
use potoo_mapler_interpreter as mapler;
use potoo_portugol_interpreter as portugol;
use potoo_potigol_interpreter as potigol;
use potoo_visualg_interpreter as visualg;

use crate::components::content_panel::Language;

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
#[cfg(windows)]
const PORTUGOL_FILE_LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
const PORTUGOL_FILE_LINE_ENDING: &str = "\n";
const OUTPUT_LIMIT_MESSAGE: &str = "\n[saída truncada: limite de 1 MiB]\n";
const OUTPUT_BATCH_BYTES: usize = 4096;
const MAX_EVENTS_PER_UI_TICK: usize = 128;
const STOP_SENTINEL: &str = "__potoo_execution_stopped__";

fn replace_file_text_regex(
    source: &str,
    pattern: &str,
    replacement: &str,
    only_first: bool,
) -> Result<String, String> {
    // shortcut: PCRE differs from Java Pattern beyond this alias; replace the engine for full compatibility.
    let pattern = pattern.replace(r"\p{javaLowerCase}", r"\p{Lowercase}");
    let regex = Regex::new(
        &pattern,
        RegexCompileFlags::empty(),
        RegexMatchFlags::empty(),
    )
    .map_err(|error| format!("expressão regular inválida: {error}"))?
    .ok_or_else(|| "não foi possível compilar a expressão regular".to_owned())?;
    let source = gtk::glib::GString::from_string_checked(source.to_owned())
        .map_err(|_| "não é possível processar arquivos com byte nulo".to_owned())?;
    let match_info = regex
        .match_(source.as_gstr(), RegexMatchFlags::empty())
        .map_err(|error| format!("falha ao pesquisar o arquivo: {error}"))?;
    if !match_info.matches() {
        return Ok(source.to_string());
    }
    let replacement = java_replacement_for_glib(replacement, &regex)?;

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
                    if escaped == '\\' {
                        output.push_str("\\\\");
                    } else {
                        output.push(escaped);
                    }
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
        *output_bytes = output_bytes
            .checked_add(1)
            .ok_or_else(|| "arquivo excede o limite de 1 MiB".to_owned())?;
        if *output_bytes > MAX_FILE_BYTES as usize {
            return Err("arquivo excede o limite de 1 MiB".to_owned());
        }
        output.push(if character as u32 > u8::MAX as u32 {
            '?'
        } else {
            character
        });
    }
    Ok(())
}

const MAX_HTTP_BODY_BYTES: usize = 10 * 1024 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;

#[cfg(test)]
fn send_http_request(request: portugol::HttpRequest) -> Result<portugol::HttpResponse, String> {
    send_http_request_cancellable(request, None)
}

fn send_http_request_cancellable(
    mut request: portugol::HttpRequest,
    cancellable: Option<&gtk::gio::Cancellable>,
) -> Result<portugol::HttpResponse, String> {
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

    let operation_cancellable = gtk::gio::Cancellable::new();
    let _external_cancel_handler = cancellable.map(|external| {
        let operation_cancellable = operation_cancellable.clone();
        external.connect_cancelled(move |_| operation_cancellable.cancel())
    });
    let (finished_sender, finished_receiver) = mpsc::channel();
    let (timed_out_sender, timed_out_receiver) = mpsc::channel();
    let timeout_cancellable = operation_cancellable.clone();
    let timeout = Duration::from_millis(request.timeout_millis);
    let timeout_thread = std::thread::spawn(move || {
        if matches!(
            finished_receiver.recv_timeout(timeout),
            Err(RecvTimeoutError::Timeout)
        ) {
            timeout_cancellable.cancel();
            let _ = timed_out_sender.send(());
        }
    });
    let result = send_http_request_inner(request, &operation_cancellable);
    let _ = finished_sender.send(());
    let _ = timeout_thread.join();
    if result.is_err() && timed_out_receiver.try_recv().is_ok() {
        return Err("a requisição HTTP excedeu o tempo limite".to_owned());
    }
    if result.is_err() && operation_cancellable.is_cancelled() {
        return Err(STOP_SENTINEL.to_owned());
    }
    result
}

fn send_http_request_inner(
    mut request: portugol::HttpRequest,
    cancellable: &gtk::gio::Cancellable,
) -> Result<portugol::HttpResponse, String> {
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
            .connect_to_uri(&request.url, port, Some(cancellable))
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
            .write_all(wire_request.as_bytes(), Some(cancellable))
            .map_err(|error| format!("não foi possível enviar a requisição HTTP: {error}"))?;
        if !request.body.is_empty() {
            connection
                .output_stream()
                .write_all(&request.body, Some(cancellable))
                .map_err(|error| format!("não foi possível enviar o corpo HTTP: {error}"))?;
        }

        let response = read_http_response(&connection, &request.method, Some(cancellable))?;
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
    cancellable: Option<&gtk::gio::Cancellable>,
) -> Result<portugol::HttpResponse, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = connection
            .input_stream()
            .read(&mut buffer, cancellable)
            .map_err(|error| format!("não foi possível ler a resposta HTTP: {error}"))?;
        if count == 0 {
            break;
        }
        let next_size = bytes.len().saturating_add(count);
        if next_size > MAX_HTTP_BODY_BYTES + MAX_HTTP_HEADER_BYTES {
            return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
        }
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = framed_http_response_end(&bytes, method)? {
            return parse_http_response(&bytes[..end], method);
        }
    }
    parse_http_response(&bytes, method)
}

fn parse_http_head(
    bytes: &[u8],
    offset: usize,
) -> Result<Option<(usize, u16, Vec<(String, String)>)>, String> {
    let remaining = &bytes[offset..];
    let Some(header_end) = remaining
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
    else {
        if remaining.len() > MAX_HTTP_HEADER_BYTES {
            return Err("o cabeçalho HTTP excede 64 KiB".to_owned());
        }
        return Ok(None);
    };
    if header_end > MAX_HTTP_HEADER_BYTES {
        return Err("o cabeçalho HTTP excede 64 KiB".to_owned());
    }
    let header_text = String::from_utf8_lossy(&remaining[..header_end]);
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
    Ok(Some((offset + header_end + 4, status, headers)))
}

#[derive(Clone, Copy)]
enum HttpBodyFraming {
    None,
    ContentLength(usize),
    Chunked,
    CloseDelimited,
}

fn http_body_framing(
    status: u16,
    method: &str,
    headers: &[(String, String)],
) -> Result<HttpBodyFraming, String> {
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
    if method == "HEAD" || matches!(status, 204 | 304 | 205) {
        return Ok(HttpBodyFraming::None);
    }
    if is_chunked {
        Ok(HttpBodyFraming::Chunked)
    } else if let Some(length) = content_lengths.first().copied() {
        if length > MAX_HTTP_BODY_BYTES {
            return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
        }
        Ok(HttpBodyFraming::ContentLength(length))
    } else {
        Ok(HttpBodyFraming::CloseDelimited)
    }
}

fn framed_http_response_end(bytes: &[u8], method: &str) -> Result<Option<usize>, String> {
    let mut offset = 0;
    loop {
        let Some((body_start, status, headers)) = parse_http_head(bytes, offset)? else {
            return Ok(None);
        };
        if (100..200).contains(&status) {
            offset = body_start;
            continue;
        }
        return match http_body_framing(status, method, &headers)? {
            HttpBodyFraming::None => Ok(Some(body_start)),
            HttpBodyFraming::ContentLength(length) => {
                let end = body_start
                    .checked_add(length)
                    .ok_or_else(|| "resposta HTTP excede o limite".to_owned())?;
                Ok((bytes.len() >= end).then_some(end))
            }
            HttpBodyFraming::Chunked => chunked_http_body_end(bytes, body_start),
            HttpBodyFraming::CloseDelimited => Ok(None),
        };
    }
}

fn parse_http_response(bytes: &[u8], method: &str) -> Result<portugol::HttpResponse, String> {
    let mut offset = 0;
    loop {
        let (body_start, status, headers) = parse_http_head(bytes, offset)?
            .ok_or_else(|| "resposta HTTP sem cabeçalho completo".to_owned())?;
        if (100..200).contains(&status) {
            offset = body_start;
            if offset == bytes.len() {
                return Err("resposta HTTP sem resposta final".to_owned());
            }
            continue;
        }
        let raw_body = &bytes[body_start..];
        let body = match http_body_framing(status, method, &headers)? {
            HttpBodyFraming::None => Vec::new(),
            HttpBodyFraming::Chunked => decode_chunked_http_body(raw_body)?,
            HttpBodyFraming::ContentLength(length) => {
                if length > raw_body.len() {
                    return Err("a resposta HTTP terminou antes do Content-Length".to_owned());
                }
                raw_body[..length].to_vec()
            }
            HttpBodyFraming::CloseDelimited => {
                if raw_body.len() > MAX_HTTP_BODY_BYTES {
                    return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
                }
                raw_body.to_vec()
            }
        };
        return Ok(portugol::HttpResponse {
            status,
            headers,
            body,
        });
    }
}

fn chunked_http_body_end(bytes: &[u8], start: usize) -> Result<Option<usize>, String> {
    let mut position = start;
    let mut body_bytes = 0usize;
    loop {
        let Some(line_end_offset) = bytes[position..]
            .windows(2)
            .position(|window| window == b"\r\n")
        else {
            if bytes.len().saturating_sub(position) > MAX_HTTP_HEADER_BYTES {
                return Err("resposta HTTP em blocos inválida".to_owned());
            }
            return Ok(None);
        };
        let line_end = position + line_end_offset;
        let size = std::str::from_utf8(&bytes[position..line_end])
            .ok()
            .and_then(|line| line.split(';').next())
            .and_then(|size| usize::from_str_radix(size.trim(), 16).ok())
            .ok_or_else(|| "resposta HTTP em blocos inválida".to_owned())?;
        position = line_end + 2;
        if size == 0 {
            let trailer_start = position;
            loop {
                let Some(trailer_end_offset) = bytes[position..]
                    .windows(2)
                    .position(|window| window == b"\r\n")
                else {
                    if bytes.len().saturating_sub(trailer_start) > MAX_HTTP_HEADER_BYTES {
                        return Err("rodapé da resposta HTTP em blocos inválido".to_owned());
                    }
                    return Ok(None);
                };
                let trailer_end = position + trailer_end_offset;
                if trailer_end == position {
                    return Ok(Some(trailer_end + 2));
                }
                if trailer_end - trailer_start > MAX_HTTP_HEADER_BYTES
                    || !bytes[position..trailer_end].contains(&b':')
                {
                    return Err("rodapé da resposta HTTP em blocos inválido".to_owned());
                }
                position = trailer_end + 2;
            }
        }
        body_bytes = body_bytes
            .checked_add(size)
            .ok_or_else(|| "resposta HTTP excede o limite".to_owned())?;
        if body_bytes > MAX_HTTP_BODY_BYTES {
            return Err("a resposta HTTP excede o limite de 10 MiB".to_owned());
        }
        let end = position
            .checked_add(size)
            .ok_or_else(|| "resposta HTTP excede o limite".to_owned())?;
        let chunk_end = end
            .checked_add(2)
            .ok_or_else(|| "resposta HTTP excede o limite".to_owned())?;
        if chunk_end > bytes.len() {
            return Ok(None);
        }
        if bytes[end..chunk_end] != *b"\r\n" {
            return Err("resposta HTTP em blocos incompleta".to_owned());
        }
        position = chunk_end;
    }
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
    Type,
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
    pub(crate) immediate_character: bool,
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
    GraphicsWindowIsOpen,
    JerimumPreviewStart,
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
    LineChanged(usize),
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
    pub(crate) cancellable: gtk::gio::Cancellable,
}

struct WorkerIo {
    events: SyncSender<ExecutionEvent>,
    input: Receiver<String>,
    control: Receiver<ControlAction>,
    pending_action: Option<ControlAction>,
    debugging: bool,
    project_root: Option<PathBuf>,
    cancellable: gtk::gio::Cancellable,
    output_bytes: usize,
    output_pending: String,
    output_truncated: bool,
    stopped: bool,
    last_visualg_line: Option<usize>,
    open_portugol_files: HashMap<i64, OpenPortugolFile>,
}

enum OpenPortugolFile {
    Read {
        path: PathBuf,
        lines: Vec<String>,
        next_line: usize,
        eof: bool,
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

pub(crate) fn normalize_file_selection_suffixes(suffixes: &str) -> Option<Vec<String>> {
    let mut normalized = Vec::new();
    for suffix in suffixes.split(',') {
        let suffix = suffix.trim();
        if matches!(suffix, "*" | "*.*") {
            return None;
        }
        let suffix = suffix.trim_start_matches('*').trim_start_matches('.');
        if !suffix.is_empty() {
            normalized.push(suffix.to_owned());
        }
    }
    Some(normalized)
}

fn portugol_file_selection_matches(path: &str, formats: &[String], allow_all_files: bool) -> bool {
    allow_all_files
        || formats.is_empty()
        || formats.iter().any(|format| {
            let suffixes = format
                .split_once('|')
                .map_or(format.as_str(), |(_, suffixes)| suffixes);
            normalize_file_selection_suffixes(suffixes).is_none_or(|suffixes| {
                suffixes.iter().any(|suffix| {
                    path.to_lowercase()
                        .ends_with(&format!(".{}", suffix.to_lowercase()))
                })
            })
        })
}

fn split_file_lines(contents: &str) -> Vec<String> {
    let bytes = contents.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'\r' | b'\n') {
            lines.push(contents[start..index].to_owned());
            if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
            index += 1;
            start = index;
        } else {
            index += 1;
        }
    }
    if start < bytes.len() {
        lines.push(contents[start..].to_owned());
    }
    lines
}

impl OpenPortugolFile {
    fn path(&self) -> &Path {
        match self {
            Self::Read { path, .. } | Self::Write { path, .. } | Self::Append { path, .. } => path,
        }
    }
}

impl WorkerIo {
    fn http_request(
        &mut self,
        request: portugol::HttpRequest,
    ) -> Result<portugol::HttpResponse, String> {
        let result = send_http_request_cancellable(request, Some(&self.cancellable));
        if self.cancellable.is_cancelled() {
            self.stopped = true;
            Err(STOP_SENTINEL.to_owned())
        } else {
            result
        }
    }

    fn flush_output(&mut self) {
        if !self.output_pending.is_empty() {
            let output = std::mem::take(&mut self.output_pending);
            let _ = self.events.send(ExecutionEvent::Output(output));
        }
    }

    fn request_host(&mut self, request: HostRequest) -> Result<portugol::HostValue, String> {
        self.flush_output();
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
        self.send_input_prompt_with_timeout(prompt, None)
    }

    fn send_input_prompt_with_timeout(
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

    fn poll_stop(&mut self) -> bool {
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

    fn await_action(&mut self, snapshot: DebuggerSnapshot) -> Result<ControlAction, String> {
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
            immediate_character: false,
        })
    }

    fn clear_output(&mut self) {
        self.output_pending.clear();
        let _ = self.events.send(ExecutionEvent::ClearOutput);
    }

    fn clear_screen(&mut self) {}

    fn on_line_changed(&mut self, line: usize) {
        if self.last_visualg_line != Some(line) {
            self.last_visualg_line = Some(line);
            let _ = self.events.send(ExecutionEvent::LineChanged(line));
        }
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
        WorkerIo::read_data_file(self, path)
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
        let current_len = file
            .metadata()
            .map_err(|error| format!("não foi possível verificar o arquivo: {error}"))?
            .len();
        let appended_len = u64::try_from(line.len())
            .ok()
            .and_then(|length| length.checked_add(1))
            .ok_or_else(|| "arquivo de dados excede o limite de 1 MiB".to_owned())?;
        if current_len
            .checked_add(appended_len)
            .is_none_or(|length| length > MAX_FILE_BYTES)
        {
            return Err("arquivo de dados excede o limite de 1 MiB".to_owned());
        }
        writeln!(file, "{line}")
            .map_err(|error| format!("não foi possível gravar o arquivo: {error}"))
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        let started = std::time::Instant::now();
        loop {
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
                Ok(action) => {
                    self.pending_action = Some(action);
                    return Ok(());
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.stopped = true;
                    return Err(STOP_SENTINEL.to_owned());
                }
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
        if action == ControlAction::Continue {
            self.debugging = false;
        }
        Ok(match action {
            ControlAction::Continue => visualg::ControlAction::Continue,
            ControlAction::Step => visualg::ControlAction::Step,
            ControlAction::Stop => visualg::ControlAction::Stop,
        })
    }
}

impl calango::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &calango::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(calango_input_prompt(prompt))
    }

    fn read_line_with_timeout(
        &mut self,
        prompt: &calango::InputPrompt,
        timeout: Duration,
    ) -> Result<String, String> {
        self.send_input_prompt_with_timeout(calango_input_prompt(prompt), Some(timeout))
    }

    fn should_stop(&mut self) -> bool {
        if self.poll_stop() {
            self.stopped = true;
            true
        } else {
            false
        }
    }

    fn clear_output(&mut self) {
        self.output_pending.clear();
        let _ = self.events.send(ExecutionEvent::ClearOutput);
    }

    fn clear_screen(&mut self) {
        self.clear_output();
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &calango::ExecutionEvent,
    ) -> Result<calango::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            calango::PauseReason::Debugger => DebugPauseReason::Debug,
            calango::PauseReason::Step => DebugPauseReason::Step,
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
        if action == ControlAction::Continue {
            self.debugging = false;
        }
        Ok(match action {
            ControlAction::Continue => calango::ControlAction::Continue,
            ControlAction::Step => calango::ControlAction::Step,
            ControlAction::Stop => calango::ControlAction::Stop,
        })
    }
}

impl mapler::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &mapler::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: Some(prompt.variable_name.clone()),
            data_type: Some(
                match prompt.data_type {
                    mapler::DataType::Integer => "inteiro",
                    mapler::DataType::Real => "real",
                    mapler::DataType::Logical => "lógico",
                    mapler::DataType::Chain => "cadeia",
                    mapler::DataType::Character => "caractere",
                }
                .to_owned(),
            ),
            immediate_character: false,
        })
    }

    fn should_stop(&mut self) -> bool {
        if self.poll_stop() {
            self.stopped = true;
            true
        } else {
            false
        }
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &mapler::ExecutionEvent,
    ) -> Result<mapler::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            mapler::PauseReason::Debug => DebugPauseReason::Debug,
            mapler::PauseReason::Step => DebugPauseReason::Step,
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
        if action == ControlAction::Continue {
            self.debugging = false;
        }
        Ok(match action {
            ControlAction::Continue => mapler::ControlAction::Continue,
            ControlAction::Step => mapler::ControlAction::Step,
            ControlAction::Stop => mapler::ControlAction::Stop,
        })
    }
}

fn calango_input_prompt(prompt: &calango::InputPrompt) -> InputPrompt {
    InputPrompt {
        variable_name: prompt.variable_name.clone(),
        data_type: Some(
            match prompt.data_type {
                calango::DataType::Integer => "inteiro",
                calango::DataType::Real => "real",
                calango::DataType::Logical => "lógico",
                calango::DataType::Character => "caractere",
                calango::DataType::Text => "texto",
            }
            .to_owned(),
        ),
        immediate_character: prompt.data_type == calango::DataType::Character,
    }
}

impl potigol::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &potigol::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: None,
            data_type: Some(
                match prompt.data_type {
                    potigol::DataType::Integer => "inteiro",
                    potigol::DataType::Real => "real",
                    potigol::DataType::Text => "texto",
                    potigol::DataType::Character => "caractere",
                    potigol::DataType::Logical => "lógico",
                }
                .to_owned(),
            ),
            immediate_character: false,
        })
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
            if self.should_stop() {
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

    fn jerimum_request(
        &mut self,
        request: potigol::JerimumRequest,
    ) -> Result<potigol::JerimumValue, String> {
        fn to_host(value: potigol::JerimumValue) -> portugol::HostValue {
            match value {
                potigol::JerimumValue::Null => portugol::HostValue::Null,
                potigol::JerimumValue::Integer(value) => portugol::HostValue::Integer(value),
                potigol::JerimumValue::Real(value) => portugol::HostValue::Real(value),
                potigol::JerimumValue::Logical(value) => portugol::HostValue::Logical(value),
                potigol::JerimumValue::String(value) => portugol::HostValue::String(value),
                potigol::JerimumValue::Array(values) => {
                    portugol::HostValue::Array(values.into_iter().map(to_host).collect())
                }
            }
        }
        fn from_host(value: portugol::HostValue) -> potigol::JerimumValue {
            match value {
                portugol::HostValue::Null => potigol::JerimumValue::Null,
                portugol::HostValue::Integer(value) => potigol::JerimumValue::Integer(value),
                portugol::HostValue::Real(value) => potigol::JerimumValue::Real(value),
                portugol::HostValue::Logical(value) => potigol::JerimumValue::Logical(value),
                portugol::HostValue::Character(value) => {
                    potigol::JerimumValue::String(value.to_string())
                }
                portugol::HostValue::String(value) => potigol::JerimumValue::String(value),
                portugol::HostValue::Array(values) => {
                    potigol::JerimumValue::Array(values.into_iter().map(from_host).collect())
                }
            }
        }

        match request {
            potigol::JerimumRequest::Graphics {
                function,
                arguments,
            } => {
                let request = if function == "iniciar_modo_grafico" {
                    HostRequest::JerimumPreviewStart
                } else {
                    HostRequest::Graphics {
                        function,
                        arguments: arguments.into_iter().map(to_host).collect(),
                    }
                };
                self.request_host(request).map(from_host)
            }
            potigol::JerimumRequest::Audio {
                function,
                arguments,
            } => self
                .request_host(HostRequest::Audio {
                    function,
                    arguments: arguments.into_iter().map(to_host).collect(),
                })
                .map(from_host),
            potigol::JerimumRequest::WindowIsOpen => match self
                .request_host(HostRequest::GraphicsWindowIsOpen)?
            {
                portugol::HostValue::Logical(value) => Ok(potigol::JerimumValue::Logical(value)),
                _ => Err("resposta inválida do host para o estado da janela".to_owned()),
            },
            potigol::JerimumRequest::KeyPressed { key } => {
                match self.request_host(HostRequest::KeyboardKeyPressed(key))? {
                    portugol::HostValue::Logical(value) => {
                        Ok(potigol::JerimumValue::Logical(value))
                    }
                    _ => Err("resposta inválida do host para o teclado".to_owned()),
                }
            }
            potigol::JerimumRequest::MouseButtonPressed { button } => {
                match self.request_host(HostRequest::MouseButtonPressed(button))? {
                    portugol::HostValue::Logical(value) => {
                        Ok(potigol::JerimumValue::Logical(value))
                    }
                    _ => Err("resposta inválida do host para o mouse".to_owned()),
                }
            }
            potigol::JerimumRequest::MousePosition => {
                self.request_host(HostRequest::MousePosition).map(from_host)
            }
            potigol::JerimumRequest::ReadFileLines { path } => {
                let lines = self
                    .read_data_file(&path)?
                    .ok_or_else(|| "arquivo de dados indisponível".to_owned())?;
                Ok(potigol::JerimumValue::Array(
                    lines
                        .into_iter()
                        .map(potigol::JerimumValue::String)
                        .collect(),
                ))
            }
        }
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &potigol::ExecutionEvent,
    ) -> Result<potigol::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            potigol::PauseReason::Debugger => DebugPauseReason::Debug,
            potigol::PauseReason::Step => DebugPauseReason::Step,
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
            ControlAction::Continue => potigol::ControlAction::Continue,
            ControlAction::Step => potigol::ControlAction::Step,
            ControlAction::Stop => potigol::ControlAction::Stop,
        })
    }
}

fn gportugol_input_prompt(prompt: &gportugol::InputPrompt) -> InputPrompt {
    InputPrompt {
        variable_name: None,
        data_type: Some(
            match prompt.data_type {
                gportugol::DataType::Integer => "inteiro",
                gportugol::DataType::Real => "real",
                gportugol::DataType::Character => "caractere",
                gportugol::DataType::Literal => "literal",
                gportugol::DataType::Logical => "lógico",
            }
            .to_owned(),
        ),
        immediate_character: false,
    }
}

impl gportugol::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &gportugol::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(gportugol_input_prompt(prompt))
    }

    fn read_line_with_timeout(
        &mut self,
        prompt: &gportugol::InputPrompt,
        timeout: Duration,
    ) -> Result<String, String> {
        self.send_input_prompt_with_timeout(gportugol_input_prompt(prompt), Some(timeout))
    }

    fn should_stop(&mut self) -> bool {
        if self.poll_stop() {
            self.stopped = true;
            true
        } else {
            false
        }
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &gportugol::ExecutionEvent,
    ) -> Result<gportugol::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            gportugol::PauseReason::Debugger => DebugPauseReason::Debug,
            gportugol::PauseReason::Step => DebugPauseReason::Step,
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
        if action == ControlAction::Continue {
            self.debugging = false;
        }
        Ok(match action {
            ControlAction::Continue => gportugol::ControlAction::Continue,
            ControlAction::Step => gportugol::ControlAction::Step,
            ControlAction::Stop => gportugol::ControlAction::Stop,
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
    let (event_sender, events) = mpsc::sync_channel(MAX_EVENTS_PER_UI_TICK * 2);
    let (input, input_receiver) = mpsc::channel();
    let (control, control_receiver) = mpsc::channel();
    let cancellable = gtk::gio::Cancellable::new();
    let worker_cancellable = cancellable.clone();
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
            cancellable: worker_cancellable,
            output_bytes: 0,
            output_pending: String::new(),
            output_truncated: false,
            stopped: false,
            last_visualg_line: None,
            open_portugol_files: HashMap::new(),
        };
        let result = execute_source(language, &source, &mut io);
        let result = match result {
            Err(diagnostics)
                if debugging
                    && diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.kind != DiagnosticKind::Runtime) =>
            {
                let location = diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.kind != DiagnosticKind::Runtime)
                    .map_or((1, 1), |diagnostic| {
                        (diagnostic.line.max(1), diagnostic.column.max(1))
                    });
                let prefix = crate::debugging::prefix(language, &source, location.0);
                match execute_source(language, &prefix, &mut io) {
                    Ok(()) => {
                        let pause = WorkerIo::execution_snapshot(
                            location.0,
                            location.1,
                            [],
                            Some(DebugPauseReason::Debug),
                        );
                        match io.await_action(pause) {
                            Ok(ControlAction::Stop) => Ok(()),
                            Ok(_) => Err(diagnostics),
                            Err(message) => Err(vec![ExecutionDiagnostic {
                                kind: DiagnosticKind::Runtime,
                                message,
                                line: location.0,
                                column: location.1,
                            }]),
                        }
                    }
                    Err(prefix_diagnostics)
                        if prefix_diagnostics.iter().any(|diagnostic| {
                            diagnostic.kind == DiagnosticKind::Runtime
                                && diagnostic.line < location.0
                        }) =>
                    {
                        Err(prefix_diagnostics)
                    }
                    Err(_) => Err(diagnostics),
                }
            }
            result => result,
        };
        io.flush_output();
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
        cancellable,
    }
}

fn execute_source(
    language: Language,
    source: &str,
    io: &mut WorkerIo,
) -> Result<(), Vec<ExecutionDiagnostic>> {
    match language {
        Language::Portugol => portugol::execute(source, io).map_err(convert_portugol_diagnostics),
        Language::Visualg => visualg::execute(source, io).map_err(convert_visualg_diagnostics),
        Language::Potigol => potigol::execute(source, io).map_err(convert_potigol_diagnostics),
        Language::GPortugol => gportugol::execute(source, io)
            .map(|_| ())
            .map_err(convert_gportugol_diagnostics),
        Language::Calango => calango::execute(source, io).map_err(convert_calango_diagnostics),
        Language::Mapler => mapler::execute(source, io).map_err(convert_mapler_diagnostics),
    }
}

fn convert_potigol_diagnostics(diagnostics: Vec<potigol::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                potigol::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                potigol::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                potigol::DiagnosticKind::Type => DiagnosticKind::Type,
                potigol::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.span.start.line,
            column: diagnostic.span.start.column,
        })
        .collect()
}

fn convert_gportugol_diagnostics(
    diagnostics: Vec<gportugol::Diagnostic>,
) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                gportugol::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                gportugol::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                gportugol::DiagnosticKind::Type => DiagnosticKind::Type,
                gportugol::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.span.start.line,
            column: diagnostic.span.start.column,
        })
        .collect()
}

fn convert_calango_diagnostics(diagnostics: Vec<calango::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                calango::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                calango::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                calango::DiagnosticKind::Type => DiagnosticKind::Type,
                calango::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
}

fn convert_mapler_diagnostics(diagnostics: Vec<mapler::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                mapler::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                mapler::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                mapler::DiagnosticKind::Type => DiagnosticKind::Type,
                mapler::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
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
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use gtk::gio::prelude::CancellableExt;

    use super::{
        ControlAction, DiagnosticKind as ExecutionDiagnosticKind, ExecutionEvent, HostRequest,
        MAX_FILE_BYTES, PORTUGOL_FILE_LINE_ENDING, RunningProgram, STOP_SENTINEL, WorkerIo,
        decode_chunked_http_body, parse_http_response, portugol, portugol_file_selection_matches,
        potigol, replace_file_text_regex, send_http_request, send_http_request_cancellable, start,
        visualg,
    };
    use crate::components::content_panel::Language;
    use crate::execution::ExecutionDiagnostic;

    const PORTUGOL_VOTING_SAMPLE: &str = r#"programa {
  funcao inicio() {
    inteiro idade

    escreva("Qual é sua idade? ")
    leia(idade)
    se (idade >= 21) {
      escreva("Pode votar.")
    } senao {
      escreva("Ainda não pode votar.")
    }
  }
}"#;

    struct ProjectTestDirectory(PathBuf);

    impl ProjectTestDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::current_dir()
                .unwrap()
                .join(format!(".potoo-execution-{label}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &PathBuf {
            &self.0
        }
    }

    impl Drop for ProjectTestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn worker_io(
        project_root: Option<PathBuf>,
    ) -> (WorkerIo, mpsc::Sender<ControlAction>, gtk::gio::Cancellable) {
        let (events, _event_receiver) = mpsc::sync_channel(16);
        let (_input_sender, input) = mpsc::channel();
        let (control, control_receiver) = mpsc::channel();
        let cancellable = gtk::gio::Cancellable::new();
        (
            WorkerIo {
                events,
                input,
                control: control_receiver,
                pending_action: None,
                debugging: false,
                project_root,
                cancellable: cancellable.clone(),
                output_bytes: 0,
                output_pending: String::new(),
                output_truncated: false,
                stopped: false,
                last_visualg_line: None,
                open_portugol_files: HashMap::new(),
            },
            control,
            cancellable,
        )
    }

    #[test]
    fn jerimum_file_reads_stay_within_the_active_project() {
        let project = ProjectTestDirectory::new("potigol-jerimum");
        fs::write(project.path().join("dados.txt"), "primeira\r\nsegunda\n").unwrap();
        let (mut io, _, _) = worker_io(Some(project.path().clone()));

        assert_eq!(
            potigol::RuntimeIo::jerimum_request(
                &mut io,
                potigol::JerimumRequest::ReadFileLines {
                    path: "dados.txt".to_owned(),
                },
            ),
            Ok(potigol::JerimumValue::Array(vec![
                potigol::JerimumValue::String("primeira".to_owned()),
                potigol::JerimumValue::String("segunda".to_owned()),
            ]))
        );
        assert!(
            potigol::RuntimeIo::jerimum_request(
                &mut io,
                potigol::JerimumRequest::ReadFileLines {
                    path: "../fora.txt".to_owned(),
                },
            )
            .is_err()
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let outside = project.path().with_extension("fora");
            fs::write(&outside, "fora").unwrap();
            symlink(&outside, project.path().join("atalho.txt")).unwrap();
            assert!(
                potigol::RuntimeIo::jerimum_request(
                    &mut io,
                    potigol::JerimumRequest::ReadFileLines {
                        path: "atalho.txt".to_owned(),
                    },
                )
                .is_err()
            );
            fs::remove_file(outside).unwrap();
        }
    }

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

    fn local_http_server_holding_connection(
        response: Option<Vec<u8>>,
    ) -> (
        String,
        mpsc::Receiver<()>,
        mpsc::Sender<()>,
        thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener should bind");
        let address = listener.local_addr().unwrap();
        let (ready_sender, ready_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("client should connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 2048];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let count = stream
                    .read(&mut buffer)
                    .expect("request should be readable");
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
            }
            if let Some(response) = response {
                stream
                    .write_all(&response)
                    .expect("response should be writable");
            }
            ready_sender.send(()).unwrap();
            let _ = release_receiver.recv_timeout(Duration::from_secs(3));
        });
        (
            format!("http://{address}"),
            ready_receiver,
            release_sender,
            server,
        )
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
    fn host_http_parser_skips_informational_responses_and_decodes_headers_lossily() {
        let response = parse_http_response(
            b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\nX-Byte: \xff\r\n\r\nok",
            "POST",
        )
        .expect("final response should parse");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"ok");
        assert!(
            response
                .headers
                .iter()
                .any(|(name, value)| { name == "X-Byte" && value == "�" })
        );
        assert_eq!(
            parse_http_response(b"HTTP/1.1 204 No Content\r\n\r\n", "GET")
                .unwrap()
                .body,
            b""
        );
        assert_eq!(
            parse_http_response(
                b"HTTP/1.1 304 Not Modified\r\nContent-Length: 4\r\n\r\n",
                "GET"
            )
            .unwrap()
            .body,
            b""
        );
        assert_eq!(
            parse_http_response(b"HTTP/1.1 200 OK\r\n\r\nignored", "HEAD")
                .unwrap()
                .body,
            b""
        );
    }

    #[test]
    fn host_http_client_returns_after_a_framed_response_on_a_persistent_connection() {
        for (method, response, expected_body) in [
            (
                "GET",
                &b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"[..],
                &b"ok"[..],
            ),
            (
                "GET",
                &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nok\r\n0\r\n\r\n"[..],
                &b"ok"[..],
            ),
            (
                "HEAD",
                &b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n"[..],
                &b""[..],
            ),
            ("GET", &b"HTTP/1.1 204 No Content\r\n\r\n"[..], &b""[..]),
            (
                "GET",
                &b"HTTP/1.1 304 Not Modified\r\nContent-Length: 2\r\n\r\n"[..],
                &b""[..],
            ),
        ] {
            let (url, ready, release, server) =
                local_http_server_holding_connection(Some(response.to_vec()));
            let method = method.to_owned();
            let (result_sender, result_receiver) = mpsc::channel();
            let client = thread::spawn(move || {
                let result = send_http_request(portugol::HttpRequest {
                    method,
                    url,
                    headers: Vec::new(),
                    body: Vec::new(),
                    timeout_millis: 2_000,
                });
                result_sender.send(result).unwrap();
            });
            ready
                .recv_timeout(Duration::from_secs(2))
                .expect("server should send the response");
            let completed_while_open = result_receiver.recv_timeout(Duration::from_millis(500));
            let finished_before_close = completed_while_open.is_ok();
            let _ = release.send(());
            server.join().unwrap();
            let result = match completed_while_open {
                Ok(result) => result,
                Err(_) => result_receiver
                    .recv_timeout(Duration::from_secs(2))
                    .expect("client should finish after server closes"),
            };
            client.join().unwrap();

            assert!(
                finished_before_close,
                "framed response should complete before connection close"
            );
            assert_eq!(result.unwrap().body, expected_body);
        }
    }

    #[test]
    fn host_http_timeout_keeps_millisecond_precision() {
        let (url, ready, release, server) = local_http_server_holding_connection(None);
        let (result_sender, result_receiver) = mpsc::channel();
        let started = std::time::Instant::now();
        let client = thread::spawn(move || {
            let result = send_http_request(portugol::HttpRequest {
                method: "GET".to_owned(),
                url,
                headers: Vec::new(),
                body: Vec::new(),
                timeout_millis: 200,
            });
            result_sender.send(result).unwrap();
        });
        ready
            .recv_timeout(Duration::from_secs(2))
            .expect("server should receive the request");
        let completed_while_open = result_receiver.recv_timeout(Duration::from_millis(500));
        let finished_before_close = completed_while_open.is_ok();
        let elapsed = started.elapsed();
        let _ = release.send(());
        server.join().unwrap();
        let result = match completed_while_open {
            Ok(result) => result,
            Err(_) => result_receiver
                .recv_timeout(Duration::from_secs(2))
                .expect("client should finish after server closes"),
        };
        client.join().unwrap();

        assert!(finished_before_close, "HTTP timeout should be below 500 ms");
        assert!(elapsed < Duration::from_millis(500));
        assert!(result.unwrap_err().contains("tempo limite"));
    }

    #[test]
    fn host_http_request_cancellation_interrupts_a_pending_read() {
        let (url, ready, release, server) = local_http_server_holding_connection(None);
        let (result_sender, result_receiver) = mpsc::channel();
        let cancellable = gtk::gio::Cancellable::new();
        let client_cancellable = cancellable.clone();
        let client = thread::spawn(move || {
            let result = send_http_request_cancellable(
                portugol::HttpRequest {
                    method: "GET".to_owned(),
                    url,
                    headers: Vec::new(),
                    body: Vec::new(),
                    timeout_millis: 5_000,
                },
                Some(&client_cancellable),
            );
            result_sender.send(result).unwrap();
        });
        ready
            .recv_timeout(Duration::from_secs(2))
            .expect("server should receive the request");
        cancellable.cancel();
        let completed_while_open = result_receiver.recv_timeout(Duration::from_millis(500));
        let finished_before_close = completed_while_open.is_ok();
        let _ = release.send(());
        server.join().unwrap();
        let result = match completed_while_open {
            Ok(result) => result,
            Err(_) => result_receiver
                .recv_timeout(Duration::from_secs(2))
                .expect("client should finish after server closes"),
        };
        client.join().unwrap();

        assert!(
            finished_before_close,
            "cancellation should interrupt the pending read"
        );
        assert_eq!(result.unwrap_err(), STOP_SENTINEL);
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
    fn portugol_file_replacement_matches_java_escaping_and_latin1_output() {
        assert_eq!(
            replace_file_text_regex("a", "a", r"\\", false).unwrap(),
            "\\"
        );
        assert_eq!(
            replace_file_text_regex("unchanged", "missing", "$9", false).unwrap(),
            "unchanged"
        );
        assert_eq!(replace_file_text_regex("a", "a", "☃", false).unwrap(), "?");
        assert_eq!(
            replace_file_text_regex("aB", r"\p{javaLowerCase}", "x", false).unwrap(),
            "xB"
        );
    }

    #[test]
    fn cancelling_a_portugol_wait_stops_without_waiting_for_its_deadline() {
        let (events, _event_receiver) = mpsc::sync_channel(256);
        let (_input_sender, input) = mpsc::channel();
        let (control, control_receiver) = mpsc::channel();
        let mut io = WorkerIo {
            events,
            input,
            control: control_receiver,
            pending_action: None,
            debugging: false,
            project_root: None,
            cancellable: gtk::gio::Cancellable::new(),
            output_bytes: 0,
            output_pending: String::new(),
            output_truncated: false,
            stopped: false,
            last_visualg_line: None,
            open_portugol_files: HashMap::new(),
        };
        control.send(ControlAction::Stop).unwrap();
        let started = std::time::Instant::now();

        let result = portugol::RuntimeIo::wait(&mut io, Duration::from_secs(30));

        assert_eq!(result, Err(STOP_SENTINEL.to_owned()));
        assert!(io.stopped);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn output_fragments_are_batched_until_a_flush_boundary() {
        let (events, output) = mpsc::sync_channel(256);
        let (_input_sender, input) = mpsc::channel();
        let (_control_sender, control) = mpsc::channel();
        let mut io = WorkerIo {
            events,
            input,
            control,
            pending_action: None,
            debugging: false,
            project_root: None,
            cancellable: gtk::gio::Cancellable::new(),
            output_bytes: 0,
            output_pending: String::new(),
            output_truncated: false,
            stopped: false,
            last_visualg_line: None,
            open_portugol_files: HashMap::new(),
        };
        io.write_limited("prompt: ");
        io.write_limited("value");
        assert!(output.try_recv().is_err());
        io.flush_output();
        assert!(matches!(
            output.try_recv(),
            Ok(ExecutionEvent::Output(text)) if text == "prompt: value"
        ));
        assert!(output.try_recv().is_err());
    }

    #[test]
    fn visualg_data_file_append_respects_the_one_mib_limit() {
        let root = std::env::temp_dir().join(format!(
            "potoo-visualg-append-limit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("dados.txt"),
            vec![b'x'; MAX_FILE_BYTES as usize - 2],
        )
        .unwrap();
        let (events, _event_receiver) = mpsc::sync_channel(256);
        let (_input_sender, input) = mpsc::channel();
        let (_control_sender, control) = mpsc::channel();
        let mut io = WorkerIo {
            events,
            input,
            control,
            pending_action: None,
            debugging: false,
            project_root: Some(root.clone()),
            cancellable: gtk::gio::Cancellable::new(),
            output_bytes: 0,
            output_pending: String::new(),
            output_truncated: false,
            stopped: false,
            last_visualg_line: None,
            open_portugol_files: HashMap::new(),
        };
        visualg::RuntimeIo::append_data_file_line(&mut io, "dados.txt", "x").unwrap();
        assert_eq!(
            fs::metadata(root.join("dados.txt")).unwrap().len(),
            MAX_FILE_BYTES
        );
        assert!(visualg::RuntimeIo::append_data_file_line(&mut io, "dados.txt", "x").is_err());
        assert_eq!(
            fs::metadata(root.join("dados.txt")).unwrap().len(),
            MAX_FILE_BYTES
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn visualg_wait_observes_stop_during_a_long_wait() {
        let (events, _event_receiver) = mpsc::sync_channel(256);
        let (_input_sender, input) = mpsc::channel();
        let (control_sender, control) = mpsc::channel();
        let mut io = WorkerIo {
            events,
            input,
            control,
            pending_action: None,
            debugging: false,
            project_root: None,
            cancellable: gtk::gio::Cancellable::new(),
            output_bytes: 0,
            output_pending: String::new(),
            output_truncated: false,
            stopped: false,
            last_visualg_line: None,
            open_portugol_files: HashMap::new(),
        };
        let stop = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            control_sender.send(ControlAction::Stop).unwrap();
        });
        let started = std::time::Instant::now();
        assert_eq!(
            visualg::RuntimeIo::wait(&mut io, Duration::from_secs(30)),
            Err(STOP_SENTINEL.to_owned())
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        stop.join().unwrap();
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
                ExecutionEvent::LineChanged(_) => {}
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

    fn collect_debug_error(
        program: RunningProgram,
    ) -> (String, Vec<usize>, Vec<ExecutionDiagnostic>, bool) {
        let mut output = String::new();
        let mut debugger_lines = Vec::new();
        loop {
            match program
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("debugging should produce an event")
            {
                ExecutionEvent::Output(text) => output.push_str(&text),
                ExecutionEvent::ClearOutput => output.clear(),
                ExecutionEvent::Debugger(snapshot) => {
                    debugger_lines.push(snapshot.line);
                    program.control.send(ControlAction::Step).unwrap();
                }
                ExecutionEvent::InputRequested(_) => {
                    program.input.send(String::new()).unwrap();
                }
                ExecutionEvent::FileSelectionRequested { .. } => {
                    program.input.send(String::new()).unwrap();
                }
                ExecutionEvent::HostRequest { response, .. } => {
                    let _ = response.send(Err("host de teste sem suporte".to_owned()));
                }
                ExecutionEvent::LineChanged(_) => {}
                ExecutionEvent::Finished { result, stopped } => {
                    let diagnostics =
                        result.expect_err("the malformed source should retain its diagnostic");
                    return (output, debugger_lines, diagnostics, stopped);
                }
            }
        }
    }

    #[test]
    fn debugger_reaches_nested_lexical_errors_in_every_language_profile() {
        let cases = [
            (
                "Portugol Studio 2.0",
                Language::Portugol,
                r#"programa {
  funcao inicio() {
    se (verdadeiro) {
      escreva("antes")
    @
      escreva("depois")
    }
  }
}"#,
                5,
            ),
            (
                "VisuAlg",
                Language::Visualg,
                "algoritmo \"depura\"\ninicio\n  se verdadeiro entao\n    escreval(\"antes\")\n    @\n    escreval(\"depois\")\n  fimse\nfimalgoritmo\n",
                5,
            ),
            (
                "Potigol clássico",
                Language::Potigol,
                "se verdadeiro entao\n escreva \"antes\"\n@\n escreva \"depois\"\nfim",
                3,
            ),
            (
                "Potigol Jerimum",
                Language::Potigol,
                "use \"jerimum\"\nse verdadeiro entao\n escreva \"antes\"\n@\n escreva \"depois\"\nfim",
                4,
            ),
            (
                "G-Portugol",
                Language::GPortugol,
                "algoritmo erro;\ninício\n se verdadeiro então\n  imprima(\"antes\");\n  @\n  imprima(\"depois\");\n fim-se\nfim",
                5,
            ),
            (
                "Calango",
                Language::Calango,
                "algoritmo erro;\nprincipal\n se verdadeiro entao\n  escreval(\"antes\");\n  @\n  escreval(\"depois\");\n fimSe\nfimPrincipal",
                5,
            ),
            (
                "Mapler",
                Language::Mapler,
                "variaveis\ninicio\n se verdadeiro entao\n  escrever \"antes\";\n  @\n  escrever \"depois\";\n fim se;\nfim.",
                5,
            ),
        ];

        for (profile, language, source, error_line) in cases {
            let (output, debugger_lines, diagnostics, stopped) =
                collect_debug_error(start(source.to_owned(), language, None, None, true));

            assert!(!stopped, "{profile}: debugging stopped unexpectedly");
            assert!(
                output.contains("antes"),
                "{profile}: valid prefix did not run"
            );
            assert!(
                !output.contains("depois"),
                "{profile}: ran beyond the error"
            );
            assert_eq!(debugger_lines.last(), Some(&error_line), "{profile}");
            assert_eq!(diagnostics[0].line, error_line, "{profile}");
            assert_ne!(
                diagnostics[0].kind,
                ExecutionDiagnosticKind::Runtime,
                "{profile}"
            );
        }
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
    fn calango_character_input_is_marked_for_immediate_submission() {
        let prompt = super::calango_input_prompt(&super::calango::InputPrompt {
            variable_name: Some("letra".to_owned()),
            data_type: super::calango::DataType::Character,
        });

        assert!(prompt.immediate_character);
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
    fn debugger_runs_portugol_if_after_typed_input_for_both_branches() {
        for (input, expected) in [
            ("21", "Qual é sua idade? Pode votar."),
            ("16", "Qual é sua idade? Ainda não pode votar."),
        ] {
            let (output, events) = collect(
                start(
                    PORTUGOL_VOTING_SAMPLE.into(),
                    Language::Portugol,
                    None,
                    None,
                    true,
                ),
                &[input],
                &[ControlAction::Step; 32],
            );
            assert_eq!(output, expected);
            assert!(events.iter().any(|event| matches!(
                event,
                ExecutionEvent::Debugger(snapshot) if snapshot.line == 7
            )));
        }
    }

    #[test]
    fn continuing_from_a_portugol_if_runs_without_pausing_on_each_branch_line() {
        for (input, expected) in [
            ("21", "Qual é sua idade? Pode votar."),
            ("16", "Qual é sua idade? Ainda não pode votar."),
        ] {
            let program = start(
                PORTUGOL_VOTING_SAMPLE.into(),
                Language::Portugol,
                None,
                None,
                true,
            );
            let mut output = String::new();
            let mut debugger_lines = Vec::new();

            loop {
                match program
                    .events
                    .recv_timeout(Duration::from_secs(2))
                    .expect("the debug session should produce an event")
                {
                    ExecutionEvent::Output(text) => output.push_str(&text),
                    ExecutionEvent::InputRequested(_) => program.input.send(input.into()).unwrap(),
                    ExecutionEvent::Debugger(snapshot) => {
                        debugger_lines.push(snapshot.line);
                        program
                            .control
                            .send(if snapshot.line == 7 {
                                ControlAction::Continue
                            } else {
                                ControlAction::Step
                            })
                            .unwrap();
                    }
                    ExecutionEvent::Finished { result, stopped } => {
                        result.expect("the conditional should execute successfully");
                        assert!(!stopped);
                        break;
                    }
                    _ => {}
                }
            }

            assert_eq!(output, expected);
            assert_eq!(debugger_lines.last(), Some(&7));
        }
    }

    #[test]
    fn runs_potigol_program_with_typed_input_and_debugger_events() {
        let source = "idade = leia_inteiro\nescreva \"dobro: {idade * 2}\"";
        let (output, events) = collect(
            start(source.into(), Language::Potigol, None, None, true),
            &["21"],
            &[ControlAction::Continue; 8],
        );

        assert_eq!(output, "dobro: 42\n");
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutionEvent::InputRequested(prompt)
                if prompt.data_type.as_deref() == Some("inteiro")
        )));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutionEvent::Debugger(_)))
        );
    }

    #[test]
    fn gportugol_forwards_input_and_output() {
        let (output, events) = collect(
            start(
                include_str!("../tests/fixtures/gportugol-voting.gpt").into(),
                Language::GPortugol,
                None,
                None,
                false,
            ),
            &["21"],
            &[],
        );

        assert_eq!(output, "Qual é sua idade?\nPode votar.\n");
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutionEvent::InputRequested(prompt)
                if prompt.data_type.as_deref() == Some("inteiro")
        )));
    }

    #[test]
    fn gportugol_maps_source_diagnostics() {
        let cases = [
            (
                "algoritmo erro;\ninício\n  @\nfim",
                ExecutionDiagnosticKind::Lexical,
                3,
            ),
            (
                "algoritmo erro;\ninício\n  imprima(1)\nfim",
                ExecutionDiagnosticKind::Syntax,
                4,
            ),
            (
                "algoritmo erro;\ninício\n  imprima(nao_existe);\nfim",
                ExecutionDiagnosticKind::Type,
                3,
            ),
            (
                "algoritmo erro;\nvariáveis valor: inteiro; fim-variáveis\ninício\n  valor := 1 / 0;\nfim",
                ExecutionDiagnosticKind::Runtime,
                4,
            ),
        ];

        for (source, expected_kind, expected_line) in cases {
            let program = start(source.into(), Language::GPortugol, None, None, false);
            loop {
                match program
                    .events
                    .recv_timeout(Duration::from_secs(2))
                    .expect("diagnostic execution should finish")
                {
                    ExecutionEvent::Finished { result, stopped } => {
                        assert!(!stopped);
                        let diagnostic = result
                            .expect_err("invalid source should produce a diagnostic")
                            .remove(0);
                        assert_eq!(diagnostic.kind, expected_kind);
                        assert_eq!(diagnostic.line, expected_line);
                        assert!(diagnostic.column > 0);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn gportugol_stop_interrupts_a_loop() {
        let source = "algoritmo loop;\nvariáveis i: inteiro; fim-variáveis\ninício\nenquanto verdadeiro faça\ni := i + 1;\nfim-enquanto\nfim";
        let program = start(source.into(), Language::GPortugol, None, None, false);
        program.control.send(ControlAction::Stop).unwrap();

        let event = program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("stopping should interrupt the G-Portugol loop");
        assert!(matches!(
            event,
            ExecutionEvent::Finished {
                result: Ok(()),
                stopped: true
            }
        ));
    }

    #[test]
    fn gportugol_stop_interrupts_input() {
        let source = "algoritmo entrada;\nvariáveis valor: inteiro; fim-variáveis\ninício\nvalor := leia();\nimprima(valor);\nfim";
        let program = start(source.into(), Language::GPortugol, None, None, false);
        loop {
            match program
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("input prompt should arrive before stopping")
            {
                ExecutionEvent::InputRequested(_) => {
                    program.control.send(ControlAction::Stop).unwrap();
                }
                ExecutionEvent::Finished { result, stopped } => {
                    result.expect("input cancellation should not become a runtime error");
                    assert!(stopped);
                    break;
                }
                _ => {}
            }
        }
    }

    #[test]
    fn gportugol_debugger_reports_variables() {
        let (output, events) = collect(
            start(
                include_str!("../tests/fixtures/gportugol-voting.gpt").into(),
                Language::GPortugol,
                None,
                None,
                true,
            ),
            &["19"],
            &[ControlAction::Step; 32],
        );

        assert_eq!(output, "Qual é sua idade?\nPode votar.\n");
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutionEvent::Debugger(snapshot)
                if snapshot.variables.iter().any(|(name, value)| name == "idade" && value == "19")
        )));
    }

    #[test]
    fn stopping_potigol_interrupts_a_cpu_bound_loop() {
        let source = "var i := 0\nenquanto i < 1000000000 faça\n  i := i + 1\nfim";
        let program = start(source.into(), Language::Potigol, None, None, false);
        program.control.send(ControlAction::Stop).unwrap();

        let event = program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("stopping should interrupt the Potigol loop");
        assert!(matches!(
            event,
            ExecutionEvent::Finished {
                result: Ok(()),
                stopped: true
            }
        ));
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
        let root = ProjectTestDirectory::new("file-library");
        let source = r#"programa {
  inclua biblioteca Arquivos --> a
  funcao inicio() {
    inteiro arquivo = a.abrir_arquivo("subdir/deep/dados.txt", a.MODO_ESCRITA)
    a.escrever_linha("olá", arquivo)
    a.fechar_arquivo(arquivo)
    arquivo = a.abrir_arquivo("subdir/deep/dados.txt", a.MODO_LEITURA)
    escreva(a.ler_linha(arquivo), "|", a.fim_arquivo(arquivo))
    escreva("|", a.ler_linha(arquivo), "|", a.fim_arquivo(arquivo))
    a.fechar_arquivo(arquivo)
  }
}"#;

        let (output, _) = collect(
            start(
                source.into(),
                Language::Portugol,
                None,
                Some(root.path().clone()),
                false,
            ),
            &[],
            &[],
        );

        assert_eq!(output, "olá|falso||verdadeiro");
        assert_eq!(
            fs::read_to_string(root.path().join("subdir/deep/dados.txt")).unwrap(),
            format!("olá{PORTUGOL_FILE_LINE_ENDING}")
        );
    }

    #[test]
    fn portugol_file_reader_splits_java_line_endings_and_replaces_invalid_utf8() {
        let root = ProjectTestDirectory::new("file-read-lines");
        fs::write(root.path().join("lines.txt"), b"A\r\nB\rC\n\xff").unwrap();
        let (mut io, _, _) = worker_io(Some(root.path().clone()));
        let handle = portugol::RuntimeIo::portugol_file_open(
            &mut io,
            "lines.txt",
            portugol::FileAccessMode::Read,
        )
        .unwrap();

        for expected in ["A", "B", "C", "�"] {
            assert_eq!(
                portugol::RuntimeIo::portugol_file_read_line(&mut io, handle).unwrap(),
                expected
            );
            assert!(!portugol::RuntimeIo::portugol_file_is_eof(&mut io, handle).unwrap());
        }
        assert_eq!(
            portugol::RuntimeIo::portugol_file_read_line(&mut io, handle).unwrap(),
            ""
        );
        assert!(portugol::RuntimeIo::portugol_file_is_eof(&mut io, handle).unwrap());
        assert_eq!(
            portugol::RuntimeIo::portugol_file_read_line(&mut io, handle).unwrap(),
            ""
        );
        assert!(portugol::RuntimeIo::portugol_file_is_eof(&mut io, handle).unwrap());
        portugol::RuntimeIo::portugol_file_close(&mut io, handle).unwrap();

        let empty = portugol::RuntimeIo::portugol_file_open(
            &mut io,
            "empty.txt",
            portugol::FileAccessMode::Write,
        )
        .unwrap();
        portugol::RuntimeIo::portugol_file_close(&mut io, empty).unwrap();
        let empty = portugol::RuntimeIo::portugol_file_open(
            &mut io,
            "empty.txt",
            portugol::FileAccessMode::Read,
        )
        .unwrap();
        assert!(!portugol::RuntimeIo::portugol_file_is_eof(&mut io, empty).unwrap());
        assert_eq!(
            portugol::RuntimeIo::portugol_file_read_line(&mut io, empty).unwrap(),
            ""
        );
        assert!(portugol::RuntimeIo::portugol_file_is_eof(&mut io, empty).unwrap());
        portugol::RuntimeIo::portugol_file_close(&mut io, empty).unwrap();
    }

    #[test]
    fn portugol_file_writer_removes_embedded_line_breaks() {
        let root = ProjectTestDirectory::new("file-write-line");
        let (mut io, _, _) = worker_io(Some(root.path().clone()));
        let handle = portugol::RuntimeIo::portugol_file_open(
            &mut io,
            "line.txt",
            portugol::FileAccessMode::Write,
        )
        .unwrap();
        portugol::RuntimeIo::portugol_file_write_line(&mut io, handle, "a\nb\rc").unwrap();
        portugol::RuntimeIo::portugol_file_close(&mut io, handle).unwrap();

        assert_eq!(
            fs::read(root.path().join("line.txt")).unwrap(),
            format!("abc{PORTUGOL_FILE_LINE_ENDING}").as_bytes()
        );
    }

    #[test]
    fn portugol_file_append_creates_missing_parent_directories_safely() {
        let root = ProjectTestDirectory::new("file-append-parent");
        let (mut io, _, _) = worker_io(Some(root.path().clone()));
        let handle = portugol::RuntimeIo::portugol_file_open(
            &mut io,
            "append/deep/line.txt",
            portugol::FileAccessMode::Append,
        )
        .unwrap();
        portugol::RuntimeIo::portugol_file_write_line(&mut io, handle, "ok").unwrap();
        portugol::RuntimeIo::portugol_file_close(&mut io, handle).unwrap();

        assert_eq!(
            fs::read(root.path().join("append/deep/line.txt")).unwrap(),
            format!("ok{PORTUGOL_FILE_LINE_ENDING}").as_bytes()
        );
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
    cadeia formatos[1] = {"Texto|*.txt"}
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
                    assert_eq!(extensions, ["Texto|*.txt"]);
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
    fn portugol_file_selection_normalizes_ui_globs_and_wildcards() {
        let suffix_filter = ["Texto|*.txt,*.csv".to_owned()];
        let wildcard_filter = ["Todos|*.*".to_owned()];

        assert!(portugol_file_selection_matches(
            "dados.txt",
            &suffix_filter,
            false
        ));
        assert!(portugol_file_selection_matches(
            "dados.CSV",
            &suffix_filter,
            false
        ));
        assert!(!portugol_file_selection_matches(
            "imagem.png",
            &suffix_filter,
            false
        ));
        assert!(portugol_file_selection_matches(
            "imagem.png",
            &wildcard_filter,
            false
        ));
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
