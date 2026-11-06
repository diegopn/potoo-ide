use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::features::media::audio::AudioCommand;
use gtk::gio::prelude::CancellableExt;

use super::{
    ControlAction, DiagnosticKind as ExecutionDiagnosticKind, ExecutionEvent, HostRequest,
    MAX_FILE_BYTES, RunningProgram, STOP_SENTINEL, WorkerIo, portugol, potigol, start, visualg,
};
use crate::features::execution::ExecutionDiagnostic;
use crate::languages::model::Language;
use crate::languages::portugol::studio_20::files::{
    PORTUGOL_FILE_LINE_ENDING, portugol_file_selection_matches, replace_file_text_regex,
};
use crate::languages::portugol::studio_20::http::{
    decode_chunked_http_body, parse_http_response, send_http_request, send_http_request_cancellable,
};

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
        "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_owned(),
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
        parse_http_response(b"HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\nabc", "GET").is_err()
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
        replace_file_text_regex("item 12, item 34", r"item (\d+)", "registro $1", false).unwrap(),
        "registro 12, registro 34"
    );
    assert_eq!(
        replace_file_text_regex("item 12, item 34", r"item (\d+)", "primeiro $1", true).unwrap(),
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
                let _ = response.send(Err("host de teste sem suporte a essa operação".to_owned()));
            }
            ExecutionEvent::AudioRequest { response, .. } => {
                let _ = response.send(Err("host de teste sem suporte a essa operação".to_owned()));
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
            ExecutionEvent::AudioRequest { response, .. } => {
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

fn collect_static_debug_pause(
    source: &str,
    action: ControlAction,
) -> (
    String,
    Vec<crate::features::execution::DebuggerSnapshot>,
    Result<(), Vec<ExecutionDiagnostic>>,
    bool,
) {
    let program = start(source.to_owned(), Language::Portugol, None, None, true);
    let mut output = String::new();
    let mut pauses = Vec::new();

    loop {
        match program
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("the Portugol debug session should produce an event")
        {
            ExecutionEvent::Output(text) => output.push_str(&text),
            ExecutionEvent::ClearOutput => output.clear(),
            ExecutionEvent::Debugger(snapshot) => {
                if snapshot.pause_reason.is_some() {
                    pauses.push(snapshot);
                }
                program.control.send(action).unwrap();
            }
            ExecutionEvent::InputRequested(_) | ExecutionEvent::FileSelectionRequested { .. } => {
                program.input.send(String::new()).unwrap();
            }
            ExecutionEvent::HostRequest { response, .. } => {
                let _ = response.send(Err("host de teste sem suporte".to_owned()));
            }
            ExecutionEvent::AudioRequest { response, .. } => {
                let _ = response.send(Err("host de teste sem suporte".to_owned()));
            }
            ExecutionEvent::LineChanged(_) => {}
            ExecutionEvent::Finished { result, stopped } => {
                return (output, pauses, result, stopped);
            }
        }
    }
}

#[test]
fn portugol_debugger_pauses_on_a_first_line_error_and_preserves_controls_and_diagnostic() {
    let source = "@\nprograma { funcao inicio() {} }";
    let expected = potoo_portugol_interpreter::parse(source).unwrap_err()[0].clone();

    for action in [
        ControlAction::Step,
        ControlAction::Continue,
        ControlAction::Stop,
    ] {
        let (_output, pauses, result, stopped) = collect_static_debug_pause(source, action);
        assert_eq!(pauses.len(), 1, "{action:?}");
        assert_eq!(pauses[0].line, expected.line, "{action:?}");
        assert_eq!(pauses[0].column, expected.column, "{action:?}");
        assert_eq!(
            pauses[0].pause_reason,
            Some(crate::features::execution::DebugPauseReason::StaticErrorNotReached),
            "{action:?}"
        );

        if action == ControlAction::Stop {
            assert!(stopped);
            assert!(result.is_ok());
        } else {
            assert!(!stopped);
            let diagnostics = result.expect_err("Step and Continue retain the original error");
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message, expected.message);
            assert_eq!(diagnostics[0].line, expected.line);
            assert_eq!(diagnostics[0].column, expected.column);
            assert_eq!(diagnostics[0].kind, ExecutionDiagnosticKind::Lexical);
        }
    }
}

#[test]
fn portugol_debugger_executes_only_the_valid_prefix_before_a_nested_error_once() {
    let source = r#"programa {
 funcao inicio() {
  escreva("ANTES")
  se (verdadeiro) {
   se (falso) {
    escreva("NUNCA")
   } senao {
    escreva("ação") @
    escreva("DEPOIS")
   }
  }
 }
}"#;
    let expected = potoo_portugol_interpreter::parse(source).unwrap_err()[0].clone();
    let (output, pauses, result, stopped) = collect_static_debug_pause(source, ControlAction::Step);

    assert_eq!(output, "ANTESação");
    assert_eq!(output.matches("ANTES").count(), 1);
    assert_eq!(output.matches("ação").count(), 1);
    assert!(!output.contains("DEPOIS"));
    assert!(!stopped);
    assert_eq!(pauses.len(), 1);
    assert_eq!(pauses[0].line, expected.line);
    assert_eq!(pauses[0].column, expected.column);
    assert_eq!(
        pauses[0].pause_reason,
        Some(crate::features::execution::DebugPauseReason::Debug)
    );
    let diagnostics = result.expect_err("the original static error remains after the pause");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.line);
    assert_eq!(diagnostics[0].column, expected.column);
}

#[test]
fn portugol_debugger_fails_closed_when_the_prefix_cuts_an_open_expression() {
    let source = "programa {\n funcao inicio() {\n  escreva(\n  @\n }\n}";
    let expected = potoo_portugol_interpreter::parse(source).unwrap_err()[0].clone();
    let (output, pauses, result, stopped) =
        collect_static_debug_pause(source, ControlAction::Continue);

    assert!(output.is_empty());
    assert!(!stopped);
    assert_eq!(pauses.len(), 1);
    assert_eq!(pauses[0].line, expected.line);
    assert_eq!(pauses[0].column, expected.column);
    assert_eq!(
        pauses[0].pause_reason,
        Some(crate::features::execution::DebugPauseReason::StaticErrorNotReached)
    );
    let diagnostics = result.expect_err("an unsafe prefix keeps the source error");
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.line);
    assert_eq!(diagnostics[0].column, expected.column);
}

#[test]
fn portugol_debugger_does_not_run_a_truncated_loop_before_a_static_error() {
    let source = r#"programa {
 funcao inicio() {
  para (inteiro i = 0; i < 2; i++) {
   escreva("ANTES") @
   escreva("DEPOIS")
  }
 }
}"#;
    let expected = potoo_portugol_interpreter::parse(source).unwrap_err()[0].clone();
    let (output, pauses, result, stopped) = collect_static_debug_pause(source, ControlAction::Step);

    assert!(
        output.is_empty(),
        "the open loop must not execute: {output:?}"
    );
    assert!(!stopped);
    assert_eq!(pauses.len(), 1);
    assert_eq!(pauses[0].line, expected.line);
    assert_eq!(pauses[0].column, expected.column);
    assert_eq!(
        pauses[0].pause_reason,
        Some(crate::features::execution::DebugPauseReason::StaticErrorNotReached)
    );
    let diagnostics = result.expect_err("the original source diagnostic must remain");
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.line);
    assert_eq!(diagnostics[0].column, expected.column);
}

#[test]
fn portugol_debugger_does_not_run_a_call_from_a_truncated_array_initializer() {
    let source = r#"programa {
 funcao inteiro marcar() {
  escreva("CHAMADA")
  retorne 1
 }
 funcao inicio() {
  inteiro valores[1] = {marcar() @, 2}
 }
}"#;
    let expected = potoo_portugol_interpreter::parse(source).unwrap_err()[0].clone();
    let (output, pauses, result, stopped) = collect_static_debug_pause(source, ControlAction::Step);

    assert!(
        output.is_empty(),
        "the errored initializer must not run: {output:?}"
    );
    assert!(!stopped);
    assert_eq!(pauses.len(), 1);
    assert_eq!(pauses[0].line, expected.line);
    assert_eq!(pauses[0].column, expected.column);
    assert_eq!(
        pauses[0].pause_reason,
        Some(crate::features::execution::DebugPauseReason::StaticErrorNotReached)
    );
    let diagnostics = result.expect_err("the original source diagnostic must remain");
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.line);
    assert_eq!(diagnostics[0].column, expected.column);
}

#[test]
fn portugol_debugger_reports_an_earlier_same_line_runtime_failure_first() {
    let source = "programa { funcao inicio() { inteiro valores[1] escreva(valores[2]) @ } }";
    let static_diagnostic = potoo_portugol_interpreter::parse(source).unwrap_err()[0].clone();
    let (_output, pauses, result, stopped) =
        collect_static_debug_pause(source, ControlAction::Step);

    assert!(pauses.is_empty());
    assert!(!stopped);
    let diagnostics = result.expect_err("the earlier runtime failure takes precedence");
    assert_eq!(diagnostics[0].kind, ExecutionDiagnosticKind::Runtime);
    assert_eq!(diagnostics[0].line, static_diagnostic.line);
    assert!(diagnostics[0].column < static_diagnostic.column);
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
    let prompt = crate::languages::calango::runtime::calango_input_prompt(
        &potoo_calango_interpreter::InputPrompt {
            variable_name: Some("letra".to_owned()),
            data_type: super::calango::DataType::Character,
        },
    );

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
fn stopping_interrupts_a_pending_audio_host_request() {
    let (events, event_receiver) = mpsc::sync_channel(8);
    let (_input_sender, input) = mpsc::channel();
    let (control, control_receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
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
        io.request_audio_host(AudioCommand::GetMasterVolume)
    });

    let ExecutionEvent::AudioRequest { command, response } = event_receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("audio host request should arrive")
    else {
        panic!("expected an audio host request");
    };
    assert_eq!(command, AudioCommand::GetMasterVolume);
    control.send(ControlAction::Stop).unwrap();
    assert_eq!(worker.join().unwrap(), Err(STOP_SENTINEL.to_owned()));
    drop(response);
}

#[test]
fn runs_visualg_voting_sample_for_both_inputs() {
    let source = include_str!("../../../tests/fixtures/visualg-voting.alg");
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
            include_str!("../../../tests/fixtures/gportugol-voting.gpt").into(),
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
            include_str!("../../../tests/fixtures/gportugol-voting.gpt").into(),
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
fn gportugol_debugger_does_not_execute_truncated_loops_before_static_errors() {
    let sources = [
        "algoritmo loop;\nvariáveis i: inteiro; fim-variáveis\ninício\n enquanto verdadeiro faça\n  imprima(\"DENTRO\"); @\n  imprima(\"DEPOIS\");\n fim-enquanto\nfim",
        "algoritmo loop;\nvariáveis i: inteiro; fim-variáveis\ninício\n para i de 1 até 2 faça\n  imprima(\"DENTRO\"); @\n  imprima(\"DEPOIS\");\n fim-para\nfim",
        "algoritmo loop;\ninício\n repita\n  imprima(\"DENTRO\"); @\n até verdadeiro;\nfim",
    ];

    for source in sources {
        let expected = potoo_gportugol_interpreter::parse(source)
            .expect_err("the loop body contains a lexical error")
            .remove(0);
        let (output, debugger_lines, diagnostics, stopped) = collect_debug_error(start(
            source.to_owned(),
            Language::GPortugol,
            None,
            None,
            true,
        ));

        assert!(
            output.is_empty(),
            "a prefix cut inside an open loop must not execute its body: {output:?}"
        );
        assert!(!stopped);
        assert_eq!(debugger_lines.last(), Some(&expected.span.start.line));
        assert_eq!(diagnostics[0].kind, ExecutionDiagnosticKind::Lexical);
        assert_eq!(diagnostics[0].message, expected.message);
        assert_eq!(diagnostics[0].line, expected.span.start.line);
        assert_eq!(diagnostics[0].column, expected.span.start.column);
    }
}

#[test]
fn mapler_debugger_does_not_execute_truncated_loops_before_static_errors() {
    let sources = [
        "variaveis i: inteiro;\ninicio\ni <- 0;\nenquanto i < 1 faca\ni <- i + 1;\nescrever \"DENTRO\";\n@;\nescrever \"DEPOIS\";\nfim enquanto;\nfim.",
        "variaveis i: inteiro;\ninicio\npara i de 1 ate 2 faca\nescrever \"DENTRO\";\n@;\nescrever \"DEPOIS\";\nfim para;\nfim.",
        "variaveis i: inteiro;\ninicio\ni <- 0;\nrepita\ni <- i + 1;\nescrever \"DENTRO\";\n@;\nescrever \"DEPOIS\";\nate i >= 1;\nfim.",
    ];

    for source in sources {
        let expected = potoo_mapler_interpreter::parse(source)
            .expect_err("the loop body contains a lexical error")
            .remove(0);
        let (output, debugger_lines, diagnostics, stopped) =
            collect_debug_error(start(source.to_owned(), Language::Mapler, None, None, true));

        assert!(
            output.is_empty(),
            "a prefix cut inside an open loop must not execute its body: {output:?}"
        );
        assert!(!stopped);
        assert_eq!(debugger_lines.last(), Some(&expected.line));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, ExecutionDiagnosticKind::Lexical);
        assert_eq!(diagnostics[0].message, expected.message);
        assert_eq!(diagnostics[0].line, expected.line);
        assert_eq!(diagnostics[0].column, expected.column);
    }
}

#[test]
fn mapler_debugger_keeps_completed_loops_before_a_later_static_error() {
    let cases = [
        (
            "variaveis i: inteiro;\ninicio\ni <- 0;\nenquanto i < 1 faca\ni <- i + 1;\nescrever \"LAÇO\";\nfim enquanto;\nescrever \"ANTES\";\n@;\nescrever \"DEPOIS\";\nfim.",
            "LAÇO\nANTES\n",
        ),
        (
            "variaveis i: inteiro;\ninicio\npara i de 1 ate 1 passo 1 faca\nescrever \"LAÇO\";\nfim para;\nescrever \"ANTES\";\n@;\nescrever \"DEPOIS\";\nfim.",
            "LAÇO\nANTES\n",
        ),
        (
            "variaveis i: inteiro;\ninicio\ni <- 0;\nrepita\ni <- i + 1;\nescrever \"LAÇO\";\nate i < 1;\nescrever \"ANTES\";\n@;\nescrever \"DEPOIS\";\nfim.",
            "LAÇO\nANTES\n",
        ),
    ];

    for (case_index, (source, expected_output)) in cases.into_iter().enumerate() {
        let expected = potoo_mapler_interpreter::parse(source)
            .expect_err("the later invalid token should be diagnosed")
            .remove(0);
        let (output, _debugger_lines, diagnostics, stopped) =
            collect_debug_error(start(source.to_owned(), Language::Mapler, None, None, true));

        assert_eq!(
            output,
            expected_output,
            "case {case_index} repeated unexpectedly; output length {} and prefix {:?}",
            output.len(),
            output.chars().take(80).collect::<String>()
        );
        assert!(!output.contains("DEPOIS"));
        assert!(!stopped);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, expected.message);
        assert_eq!(diagnostics[0].line, expected.line);
        assert_eq!(diagnostics[0].column, expected.column);
    }
}

#[test]
fn mapler_debugger_executes_valid_same_line_prefix_and_keeps_parser_positions() {
    let lf = "variaveis\ninicio\nescrever \"ação\"; @; escrever \"DEPOIS\";\nfim.";
    let crlf = lf.replace('\n', "\r\n");
    let bare_cr = "variaveis\ninicio\rescrever \"ação\"; @; escrever \"DEPOIS\";\nfim.";

    for source in [lf.to_owned(), crlf, bare_cr.to_owned()] {
        let expected = potoo_mapler_interpreter::parse(&source)
            .expect_err("the standalone invalid token should be diagnosed")
            .remove(0);
        let (output, debugger_lines, diagnostics, stopped) =
            collect_debug_error(start(source.clone(), Language::Mapler, None, None, true));

        assert_eq!(output, "ação\n", "{source:?}");
        assert!(!output.contains("DEPOIS"));
        assert!(!stopped);
        assert!(debugger_lines.contains(&expected.line), "{source:?}");
        assert_eq!(diagnostics.len(), 1, "{source:?}");
        assert_eq!(diagnostics[0].kind, ExecutionDiagnosticKind::Lexical);
        assert_eq!(diagnostics[0].message, expected.message);
        assert_eq!(diagnostics[0].line, expected.line);
        assert_eq!(diagnostics[0].column, expected.column);
    }
}

#[test]
fn mapler_debugger_fails_closed_for_empty_and_ambiguous_prefixes() {
    let sources = [
        "@\nvariaveis\ninicio\nescrever \"DEPOIS\";\nfim.",
        "variaveis x: inteiro;\ninicio\nescrever \"ANTES\";\nx <- (1 + @;\nescrever \"DEPOIS\";\nfim.",
    ];

    for source in sources {
        let expected = potoo_mapler_interpreter::parse(source)
            .expect_err("the invalid or incomplete prefix must keep its diagnostic")
            .remove(0);
        let (output, _debugger_lines, diagnostics, stopped) =
            collect_debug_error(start(source.to_owned(), Language::Mapler, None, None, true));

        assert!(output.is_empty(), "unsafe code must not run: {output:?}");
        assert!(!stopped);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, expected.message);
        assert_eq!(diagnostics[0].line, expected.line);
        assert_eq!(diagnostics[0].column, expected.column);
    }
}

#[test]
fn mapler_debugger_reports_an_earlier_same_line_runtime_failure_first() {
    let source =
        "variaveis valores: vetor [1..2] de inteiro;\ninicio\nescrever valores[3]; @;\nfim.";
    let static_diagnostic = potoo_mapler_interpreter::parse(source)
        .expect_err("the invalid token should be diagnosed")
        .remove(0);
    let (_output, _debugger_lines, diagnostics, stopped) =
        collect_debug_error(start(source.to_owned(), Language::Mapler, None, None, true));

    assert!(!stopped);
    assert_eq!(diagnostics[0].kind, ExecutionDiagnosticKind::Runtime);
    assert!(diagnostics[0].message.contains("índice fora dos limites"));
    assert_eq!(diagnostics[0].line, static_diagnostic.line);
    assert!(diagnostics[0].column < static_diagnostic.column);
}

#[test]
fn mapler_debugger_does_not_run_a_truncated_module_called_by_main() {
    let source = "variaveis executar: modulo;\ninicio\nexecutar;\nfim.\nmodulo executar;\nescrever \"DENTRO\";\n@;\nfim modulo;";
    let expected = potoo_mapler_interpreter::parse(source)
        .expect_err("the invalid token should make the module incomplete")
        .remove(0);
    let (output, _debugger_lines, diagnostics, stopped) =
        collect_debug_error(start(source.to_owned(), Language::Mapler, None, None, true));

    assert!(
        output.is_empty(),
        "an incomplete module must not run: {output:?}"
    );
    assert!(!stopped);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.line);
    assert_eq!(diagnostics[0].column, expected.column);
}

#[test]
fn gportugol_debugger_executes_the_valid_prefix_once_before_a_static_error() {
    let source = "algoritmo teste;\ninício\n imprima(\"ANTES\");\n imprima(\"ação\"); @; imprima(\"DEPOIS\");\nfim";
    let expected = potoo_gportugol_interpreter::parse(source)
        .expect_err("the standalone invalid token should be diagnosed")
        .remove(0);
    assert_eq!(expected.span.start.column, 19);
    let (output, debugger_lines, diagnostics, stopped) = collect_debug_error(start(
        source.to_owned(),
        Language::GPortugol,
        None,
        None,
        true,
    ));

    assert_eq!(output, "ANTES\nação\n");
    assert!(!output.contains("DEPOIS"));
    assert!(!stopped);
    assert!(debugger_lines.contains(&expected.span.start.line));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].kind, ExecutionDiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.span.start.line);
    assert_eq!(diagnostics[0].column, expected.span.start.column);
}

#[test]
fn gportugol_debugger_does_not_run_a_truncated_function_called_by_the_main_block() {
    let source = "algoritmo teste;\ninício\nimprima(f());\nfim\nfunção f(): inteiro\ninício\nimprima(\"antes\");\n@;\nretorne 1;\nfim";
    let expected = potoo_gportugol_interpreter::parse(source)
        .expect_err("the invalid function body should prevent execution")
        .remove(0);
    let (output, debugger_lines, diagnostics, stopped) = collect_debug_error(start(
        source.to_owned(),
        Language::GPortugol,
        None,
        None,
        true,
    ));

    assert!(
        output.is_empty(),
        "an incomplete function must not run through its caller: {output:?}"
    );
    assert!(!stopped);
    assert_eq!(debugger_lines.last(), Some(&expected.span.start.line));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, expected.message);
    assert_eq!(diagnostics[0].line, expected.span.start.line);
    assert_eq!(diagnostics[0].column, expected.span.start.column);
}

#[test]
fn gportugol_debugger_recognizes_subtraction_after_the_if_keyword() {
    let source = "algoritmo teste;\ninício\nse-1 < 0 então\nimprima(\"ANTES\");\n@;\nfim-se\nfim";
    let expected = potoo_gportugol_interpreter::parse(source)
        .expect_err("the invalid token should be diagnosed")
        .remove(0);
    let (output, debugger_lines, diagnostics, stopped) = collect_debug_error(start(
        source.to_owned(),
        Language::GPortugol,
        None,
        None,
        true,
    ));

    assert_eq!(output, "ANTES\n");
    assert!(!stopped);
    assert!(debugger_lines.contains(&expected.span.start.line));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, expected.message);
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
    let root = std::env::temp_dir().join(format!("potoo-portugol-picker-{}", std::process::id()));
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
    let root = std::env::temp_dir().join(format!("potoo-portugol-file-ops-{}", std::process::id()));
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
