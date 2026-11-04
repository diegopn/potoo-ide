use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use gtk::gio::prelude::*;

use crate::features::execution::STOP_SENTINEL;
use potoo_portugol_interpreter as portugol;

pub(crate) const MAX_HTTP_BODY_BYTES: usize = 10 * 1024 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;

#[cfg(test)]
pub(crate) fn send_http_request(
    request: portugol::HttpRequest,
) -> Result<portugol::HttpResponse, String> {
    send_http_request_cancellable(request, None)
}

pub(crate) fn send_http_request_cancellable(
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

pub(crate) fn parse_http_response(
    bytes: &[u8],
    method: &str,
) -> Result<portugol::HttpResponse, String> {
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

pub(crate) fn decode_chunked_http_body(bytes: &[u8]) -> Result<Vec<u8>, String> {
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
