use potoo_portugol_interpreter::{
    CalendarDateTime, DiagnosticKind, ExecutionOptions, FileAccessMode, HostValue, HttpRequest,
    HttpResponse, RuntimeIo, execute, execute_with_options,
};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

#[derive(Default)]
struct TestIo {
    input: Vec<String>,
    next_input: usize,
    output: String,
    waits: Vec<Duration>,
    files: HashMap<String, Vec<String>>,
    open_files: HashMap<i64, MockFile>,
    next_file_handle: i64,
    http_requests: Vec<HttpRequest>,
    http_responses: VecDeque<HttpResponse>,
    downloads: Vec<(String, Vec<u8>)>,
    pressed_keys: Vec<i64>,
    key_to_read: i64,
    pressed_mouse_buttons: Vec<i64>,
    mouse_button_to_read: i64,
    mouse_coordinates: (i64, i64),
    cursor_visibility: Vec<bool>,
    graphics_calls: Vec<(String, Vec<HostValue>)>,
    audio_calls: Vec<(String, Vec<HostValue>)>,
}

struct MockFile {
    path: String,
    mode: FileAccessMode,
    lines: Vec<String>,
    next_line: usize,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self) -> Result<String, String> {
        let value = self
            .input
            .get(self.next_input)
            .cloned()
            .ok_or_else(|| "missing test input".to_owned())?;
        self.next_input += 1;
        Ok(value)
    }

    fn clear_output(&mut self) {
        self.output.clear();
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        self.waits.push(duration);
        std::thread::sleep(duration);
        Ok(())
    }

    fn calendar_now(&mut self) -> Result<CalendarDateTime, String> {
        Ok(CalendarDateTime {
            year: 2026,
            month: 11,
            day: 2,
            weekday: 2,
            hour: 13,
            minute: 4,
            second: 5,
            millisecond: 678,
        })
    }

    fn portugol_file_open(&mut self, path: &str, mode: FileAccessMode) -> Result<i64, String> {
        let lines = match mode {
            FileAccessMode::Read => self
                .files
                .get(path)
                .cloned()
                .ok_or_else(|| "file not found".to_owned())?,
            FileAccessMode::Write => {
                self.files.insert(path.to_owned(), Vec::new());
                Vec::new()
            }
            FileAccessMode::Append => self.files.entry(path.to_owned()).or_default().clone(),
        };
        let handle = self.next_file_handle;
        self.next_file_handle += 1;
        self.open_files.insert(
            handle,
            MockFile {
                path: path.to_owned(),
                mode,
                lines,
                next_line: 0,
            },
        );
        Ok(handle)
    }

    fn portugol_file_close(&mut self, handle: i64) -> Result<(), String> {
        self.open_files
            .remove(&handle)
            .map(|_| ())
            .ok_or_else(|| "invalid file handle".to_owned())
    }

    fn portugol_file_is_eof(&mut self, handle: i64) -> Result<bool, String> {
        let file = self
            .open_files
            .get(&handle)
            .ok_or_else(|| "invalid file handle".to_owned())?;
        if file.mode != FileAccessMode::Read {
            return Err("not opened for reading".to_owned());
        }
        Ok(file.next_line >= file.lines.len())
    }

    fn portugol_file_read_line(&mut self, handle: i64) -> Result<String, String> {
        let file = self
            .open_files
            .get_mut(&handle)
            .ok_or_else(|| "invalid file handle".to_owned())?;
        if file.mode != FileAccessMode::Read {
            return Err("not opened for reading".to_owned());
        }
        let line = file
            .lines
            .get(file.next_line)
            .cloned()
            .ok_or_else(|| "end of file".to_owned())?;
        file.next_line += 1;
        Ok(line)
    }

    fn portugol_file_write_line(&mut self, handle: i64, line: &str) -> Result<(), String> {
        let (path, lines) = {
            let file = self
                .open_files
                .get_mut(&handle)
                .ok_or_else(|| "invalid file handle".to_owned())?;
            if file.mode == FileAccessMode::Read {
                return Err("not opened for writing".to_owned());
            }
            file.lines.push(line.to_owned());
            (file.path.clone(), file.lines.clone())
        };
        self.files.insert(path, lines);
        Ok(())
    }

    fn portugol_http_request(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
        self.http_requests.push(request);
        Ok(self.http_responses.pop_front().unwrap_or(HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: b"resposta".to_vec(),
        }))
    }

    fn portugol_file_write_bytes(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        self.downloads.push((path.to_owned(), bytes.to_vec()));
        Ok(())
    }

    fn portugol_file_write_image(
        &mut self,
        path: &str,
        bytes: &[u8],
        _format: &str,
    ) -> Result<(), String> {
        self.downloads.push((path.to_owned(), bytes.to_vec()));
        Ok(())
    }

    fn portugol_key_pressed(&mut self, key: i64) -> Result<bool, String> {
        Ok(self.pressed_keys.contains(&key))
    }

    fn portugol_any_key_pressed(&mut self) -> Result<bool, String> {
        Ok(!self.pressed_keys.is_empty())
    }

    fn portugol_read_key(&mut self) -> Result<i64, String> {
        Ok(self.key_to_read)
    }

    fn portugol_mouse_button_pressed(&mut self, button: i64) -> Result<bool, String> {
        Ok(self.pressed_mouse_buttons.contains(&button))
    }

    fn portugol_any_mouse_button_pressed(&mut self) -> Result<bool, String> {
        Ok(!self.pressed_mouse_buttons.is_empty())
    }

    fn portugol_read_mouse_button(&mut self) -> Result<i64, String> {
        Ok(self.mouse_button_to_read)
    }

    fn portugol_mouse_position(&mut self) -> Result<(i64, i64), String> {
        Ok(self.mouse_coordinates)
    }

    fn portugol_mouse_set_cursor_visible(&mut self, visible: bool) -> Result<(), String> {
        self.cursor_visibility.push(visible);
        Ok(())
    }

    fn portugol_graphics(
        &mut self,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        self.graphics_calls
            .push((function.to_owned(), arguments.to_vec()));
        let returns_integer = matches!(
            function,
            "renderizar_imagem"
                | "carregar_imagem"
                | "transformar_imagem"
                | "redimensionar_imagem"
                | "obter_cor_pixel"
                | "obter_rgb"
                | "transformar_porcao_imagem"
                | "obter_intervalo_gif"
                | "obter_numero_quadros_gif"
                | "obter_numero_quadro_atual_gif"
                | "obter_quadro_gif"
                | "largura_texto"
                | "altura_texto"
                | "largura_imagem"
                | "altura_imagem"
                | "criar_cor"
                | "largura_janela"
                | "altura_janela"
                | "largura_tela"
                | "altura_tela"
        );
        Ok(if returns_integer {
            HostValue::Integer(42)
        } else {
            HostValue::Null
        })
    }

    fn portugol_audio(
        &mut self,
        function: &str,
        arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        self.audio_calls
            .push((function.to_owned(), arguments.to_vec()));
        Ok(match function {
            "carregar_som"
            | "reproduzir_som"
            | "obter_tamanho_musica"
            | "obter_posicao_atual_musica"
            | "obter_volume"
            | "obter_volume_reproducao" => HostValue::Integer(42),
            _ => HostValue::Null,
        })
    }
}

fn run(source: &str) -> String {
    let mut io = TestIo::default();
    execute(source, &mut io).expect("program should execute");
    io.output
}

#[test]
fn evaluates_arithmetic_precedence_and_assignments() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n inteiro total = 2 + 3 * 4\n total += 2\n escreva(total)\n }\n}"
        ),
        "16"
    );
}

#[test]
fn division_between_integers_discards_the_fractional_part() {
    assert_eq!(
        run("programa { funcao inicio() { escreva(7 / 2, \"|\", 7 / 2.0) } }"),
        "3|3.5"
    );
}

#[test]
fn parses_real_literals_with_an_optional_leading_or_trailing_digit() {
    assert_eq!(
        run("programa { funcao inicio() { escreva(.5 + 1., \"|\", .5) } }"),
        "1.5|0.5"
    );
}

#[test]
fn evaluates_while_do_while_and_for_loops() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n inteiro soma = 0\n para (inteiro i = 1; i <= 3; i++) {\n soma += i\n }\n inteiro n = 0\n enquanto (n < 2) {\n soma += 1\n n++\n }\n faca {\n soma += 1\n n--\n } enquanto (n > 0)\n escreva(soma)\n }\n}"
        ),
        "10"
    );
}

#[test]
fn calls_typed_functions_and_returns_their_values() {
    assert_eq!(
        run(
            "programa {\n funcao inteiro soma(inteiro a, inteiro b) {\n retorne a + b\n }\n funcao inicio() {\n escreva(soma(3, 4))\n }\n}"
        ),
        "7"
    );
}

#[test]
fn reads_and_writes_vectors_and_matrices() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n inteiro valores[3]\n valores[0] = 4\n valores[1] = 5\n inteiro matriz[2][2]\n matriz[1][0] = 6\n escreva(valores[0] + valores[1] + matriz[1][0])\n }\n}"
        ),
        "15"
    );
}

#[test]
fn selects_matching_cases_and_the_default_case() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n inteiro valor = 2\n escolha (valor) {\n caso 1:\n escreva(\"um\")\n pare\n caso 2:\n escreva(\"dois\")\n pare\n caso contrario:\n escreva(\"outro\")\n }\n }\n}"
        ),
        "dois"
    );
}

#[test]
fn choice_cases_fall_through_until_an_unbraced_pare() {
    assert_eq!(
        run(r#"programa {
  funcao inicio() {
    escolha (1) {
      caso 1:
        escreva("A")
      caso 2:
        escreva("B")
        pare
      caso contrario:
        escreva("C")
    }
  }
}"#),
        "AB"
    );
}

#[test]
fn accepts_a_pare_after_a_braced_choice_case() {
    let source = r#"programa {
  funcao inicio() {
    escolha (1) {
      caso 1: {
        escreva("A")
      }
      pare
      caso contrario:
        escreva("B")
    }
  }
}"#;
    assert_eq!(run(source), "AB");
}

#[test]
fn calls_the_math_library_and_concatenates_text() {
    assert_eq!(
        run(
            "programa {\n inclua biblioteca Matematica\n funcao inicio() {\n escreva(\"resultado: \" + Matematica.potencia(2, 3))\n }\n}"
        ),
        "resultado: 8.0"
    );
}

#[test]
fn supports_all_documented_math_library_functions_and_pi() {
    let source = r#"programa {
  inclua biblioteca Matematica --> m
  funcao inicio() {
    escreva(
      m.PI > 3.14, "|", m.potencia(2, 3) == 8, "|",
      m.raiz(9, 2) == 3, "|", m.arredondar(2.345, 2) == 2.35, "|",
      m.logaritmo(8, 2) > 2.99, "|", m.seno(0) == 0, "|",
      m.cosseno(0) == 1, "|", m.tangente(0) == 0, "|",
      m.valor_absoluto(-4) == 4, "|", m.maior_numero(1, 2) == 2, "|",
      m.menor_numero(1, 2) == 1
    )
  }
}"#;
    assert_eq!(
        run(source),
        "verdadeiro|verdadeiro|verdadeiro|verdadeiro|verdadeiro|verdadeiro|verdadeiro|verdadeiro|verdadeiro|verdadeiro|verdadeiro"
    );
}

#[test]
fn supports_portugol_2_0_calendar_day_name_functions() {
    let source = r#"programa {
  inclua biblioteca Calendario --> c
  funcao inicio() {
    escreva(
      c.dia_semana_completo(2, falso, falso), "|",
      c.dia_semana_curto(3, falso, verdadeiro), "|",
      c.dia_semana_abreviado(7, verdadeiro, falso)
    )
  }
}"#;

    assert_eq!(run(source), "Segunda-Feira|terça|SAB");
}

#[test]
fn exposes_portugol_2_0_file_access_constants() {
    let source = r#"programa {
  inclua biblioteca Arquivos --> a
  funcao inicio() {
    escreva(a.MODO_LEITURA, "|", a.MODO_ESCRITA, "|", a.MODO_ACRESCENTAR)
  }
}"#;

    assert_eq!(run(source), "0|1|2");
}

#[test]
fn supports_all_keyboard_and_mouse_functions_and_constants_through_the_host() {
    let source = r#"programa {
  inclua biblioteca Teclado --> t
  inclua biblioteca Mouse --> m
  funcao inicio() {
    escreva(t.TECLA_ENTER, "|", t.TECLA_F24, "|", t.TECLA_NUM_SEPARADOR_DECIMAL, "|")
    escreva(t.tecla_pressionada(t.TECLA_F24), "|", t.alguma_tecla_pressionada(), "|")
    escreva(t.ler_tecla(), "|", t.caracter_tecla(65), "|")
    escreva(m.BOTAO_ESQUERDO, "|", m.BOTAO_DIREITO, "|", m.BOTAO_MEIO, "|")
    escreva(m.botao_pressionado(m.BOTAO_MEIO), "|", m.algum_botao_pressionado(), "|")
    escreva(m.ler_botao(), "|", m.posicao_x(), "|", m.posicao_y())
    m.ocultar_cursor()
    m.exibir_cursor()
  }
}"#;
    let mut io = TestIo {
        pressed_keys: vec![61_451],
        key_to_read: 65,
        pressed_mouse_buttons: vec![2],
        mouse_button_to_read: 2,
        mouse_coordinates: (321, 123),
        ..TestIo::default()
    };

    execute(source, &mut io).expect("keyboard and mouse operations should use the host");

    assert_eq!(
        io.output,
        "10|61451|108|verdadeiro|verdadeiro|65|A|0|1|2|verdadeiro|verdadeiro|2|321|123"
    );
    assert_eq!(io.cursor_visibility, [false, true]);
}

#[test]
fn rejects_unknown_mouse_button_codes() {
    let source = r#"programa {
  inclua biblioteca Mouse
  funcao inicio() {
    Mouse.botao_pressionado(3)
  }
}"#;
    let mut io = TestIo::default();

    let diagnostics = execute(source, &mut io).expect_err("unknown buttons should fail");

    assert!(diagnostics[0].message.contains("botão do mouse válido"));
}

#[test]
fn reads_writes_and_appends_files_through_host_services() {
    let source = r#"programa {
  inclua biblioteca Arquivos --> a
  funcao inicio() {
    inteiro arquivo = a.abrir_arquivo("notas.txt", a.MODO_ESCRITA)
    a.escrever_linha("primeiro", arquivo)
    a.fechar_arquivo(arquivo)
    arquivo = a.abrir_arquivo("notas.txt", a.MODO_ACRESCENTAR)
    a.escrever_linha("segundo", arquivo)
    a.fechar_arquivo(arquivo)
    arquivo = a.abrir_arquivo("notas.txt", a.MODO_LEITURA)
    escreva(a.ler_linha(arquivo), "|", a.ler_linha(arquivo), "|", a.fim_arquivo(arquivo))
    a.fechar_arquivo(arquivo)
  }
}"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the host file service should support the operations");

    assert_eq!(io.output, "primeiro|segundo|verdadeiro");
    assert_eq!(io.files["notas.txt"], ["primeiro", "segundo"]);
}

#[test]
fn creates_objects_from_json_and_reads_each_documented_value_type() {
    let source = r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro pessoa = o.criar_objeto_via_json("{\"nome\":\"Ana\",\"idade\":20,\"altura\":1.7,\"ativo\":true,\"letra\":\"X\",\"notas\":[7,9],\"endereco\":{\"cidade\":\"Recife\"}}")
    inteiro endereco = o.obter_propriedade_tipo_objeto(pessoa, "endereco")
    escreva(
      o.obter_propriedade_tipo_cadeia(pessoa, "nome"), "|",
      o.obter_propriedade_tipo_inteiro(pessoa, "idade"), "|",
      o.obter_propriedade_tipo_real(pessoa, "altura"), "|",
      o.obter_propriedade_tipo_logico(pessoa, "ativo"), "|",
      o.obter_propriedade_tipo_caracter(pessoa, "letra"), "|",
      o.obter_tamanho_vetor_propriedade(pessoa, "notas"), "|",
      o.obter_propriedade_tipo_inteiro_em_vetor(pessoa, "notas", 1), "|",
      o.tipo_propriedade(pessoa, "notas"), "|",
      o.tipo_propriedade(pessoa, "endereco"), "|",
      o.obter_propriedade_tipo_cadeia(endereco, "cidade"), "|",
      o.contem_propriedade(pessoa, "nome")
    )
  }
}"#;

    assert_eq!(
        run(source),
        "Ana|20|1.7|verdadeiro|X|2|9|7|6|Recife|verdadeiro"
    );
}

#[test]
fn supports_object_vectors_assignment_constants_and_json_serialization() {
    let source = r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro pessoa = o.criar_objeto()
    inteiro notas[2]
    notas[0] = 6
    notas[1] = 8
    o.atribuir_propriedade(pessoa, "nome", "Lia")
    o.atribuir_propriedade(pessoa, "notas", notas)
    inteiro lista = o.criar_objeto_via_json("{\"valores\":[true,1.5,\"Z\",\"fim\",{\"ok\":true}]}")
    escreva(
      o.TIPO_INTEIRO, "|", o.TIPO_CADEIA, "|", o.TIPO_CARACTER, "|",
      o.TIPO_REAL, "|", o.TIPO_LOGICO, "|", o.TIPO_OBJETO, "|", o.TIPO_VETOR, "|",
      o.obter_propriedade_tipo_cadeia(pessoa, "nome"), "|",
      o.obter_propriedade_tipo_inteiro_em_vetor(pessoa, "notas", 1), "|",
      o.obter_propriedade_tipo_logico_em_vetor(lista, "valores", 0), "|",
      o.obter_propriedade_tipo_real_em_vetor(lista, "valores", 1), "|",
      o.obter_propriedade_tipo_caracter_em_vetor(lista, "valores", 2), "|",
      o.obter_propriedade_tipo_cadeia_em_vetor(lista, "valores", 3), "|",
      o.obter_propriedade_tipo_objeto_em_vetor(lista, "valores", 4), "|",
      o.obter_json(pessoa)
    )
    o.liberar_objeto(lista)
    o.liberar()
  }
}"#;
    let output = run(source);

    assert!(output.starts_with("1|2|3|4|5|6|7|Lia|8|verdadeiro|1.5|Z|fim|"));
    assert!(output.contains("\"notas\" : [ 6, 8 ]"));
}

#[test]
fn xml_objects_omit_the_root_keep_the_last_repeated_element_and_decode_entities() {
    let source = r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto_via_xml("<?xml version=\"1.0\"?><pessoa><nome>Ana &amp; Bia</nome><telefone>111</telefone><telefone>222</telefone></pessoa>")
    escreva(
      o.obter_propriedade_tipo_cadeia(dados, "nome"), "|",
      o.obter_propriedade_tipo_cadeia(dados, "telefone")
    )
  }
}"#;

    assert_eq!(run(source), "Ana & Bia|222");
}

#[test]
fn nested_object_handles_share_the_original_property_map() {
    let source = r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro pessoa = o.criar_objeto_via_json("{\"endereco\":{\"rua\":\"A\"}}")
    inteiro endereco = o.obter_propriedade_tipo_objeto(pessoa, "endereco")
    o.atribuir_propriedade(endereco, "numero", 12)
    inteiro endereco_atualizado = o.obter_propriedade_tipo_objeto(pessoa, "endereco")
    escreva(o.obter_propriedade_tipo_inteiro(endereco_atualizado, "numero"))
  }
}"#;

    assert_eq!(run(source), "12");
}

#[test]
fn xml_objects_map_namespaced_attributes_and_elements_by_local_name() {
    let source = r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto_via_xml("<teste:pessoa xmlns:teste=\"urn:x\" teste:id=\"42\">Pessoa <teste:nome>Ana</teste:nome><literal><![CDATA[&amp;]]></literal></teste:pessoa>")
    escreva(
      o.obter_propriedade_tipo_cadeia(dados, "id"), "|",
      o.obter_propriedade_tipo_cadeia(dados, ""), "|",
      o.obter_propriedade_tipo_cadeia(dados, "nome"), "|",
      o.obter_propriedade_tipo_cadeia(dados, "literal")
    )
  }
}"#;

    assert_eq!(run(source), "42|Pessoa|Ana|&amp;");
}

#[test]
fn supports_the_internet_library_through_host_http_and_file_services() {
    let source = r#"programa {
  inclua biblioteca Internet --> i
  funcao inicio() {
    i.definir_tempo_limite(1500)
    escreva(i.obter_texto("https://example.test/texto"), "|",
      i.endereco_disponivel("https://example.test/status"), "|",
      i.baixar_imagem("https://example.test/capa", "imagens/capa"))
  }
}"#;
    let mut io = TestIo::default();
    io.http_responses = VecDeque::from([
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: b"Conteudo".to_vec(),
        },
        HttpResponse {
            status: 200,
            headers: vec![("Content-Length".into(), "12".into())],
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            headers: vec![("Content-Type".into(), "image/png".into())],
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            headers: vec![("Content-Type".into(), "image/png".into())],
            body: b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec(),
        },
    ]);

    execute(source, &mut io).expect("internet library calls should use the host");

    assert_eq!(io.output, "Conteudo|verdadeiro|png");
    assert_eq!(
        io.http_requests
            .iter()
            .map(|request| request.method.as_str())
            .collect::<Vec<_>>(),
        ["GET", "HEAD", "HEAD", "GET"]
    );
    assert_eq!(io.http_requests[0].timeout_millis, 1500);
    assert_eq!(io.downloads[0].0, "imagens/capa.png");
    assert_eq!(io.downloads[0].1, b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR");
}

#[test]
fn internet_text_uses_the_declared_or_default_http_charset() {
    let source = r#"programa {
  inclua biblioteca Internet --> i
  funcao inicio() {
    escreva(i.obter_texto("https://example.test/declared"), "|", i.obter_texto("https://example.test/default"))
  }
}"#;
    let mut io = TestIo::default();
    io.http_responses = VecDeque::from([
        HttpResponse {
            status: 200,
            headers: vec![(
                "Content-Type".into(),
                "text/plain; charset=\"ISO-8859-1\"".into(),
            )],
            body: b"caf\xe9".to_vec(),
        },
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: b"caf\xe9".to_vec(),
        },
    ]);

    execute(source, &mut io).expect("HTTP text must honor the response charset");

    assert_eq!(io.output, "café|café");
}

#[test]
fn supports_all_web_service_methods_headers_parameters_and_constants() {
    let source = r#"programa {
  inclua biblioteca ServicosWeb --> w
  funcao inicio() {
    escreva(w.OBTER, "|", w.PUBLICAR, "|", w.ATUALIZAR, "|", w.EXCLUIR, "|")
    escreva(w.obter_dados("https://example.test/get"), "|")
    escreva(w.excluir_dados("https://example.test/delete"), "|")
    escreva(w.publicar_dados("https://example.test/post", "{}"), "|")
    escreva(w.atualizar_dados("https://example.test/put", "{}"), "|")
    w.abrir_conexao("https://example.test/custom")
    w.adicionar_cabecalho("X-Token", "abc")
    w.adicionar_parametros("{\"ok\":true}")
    escreva(w.fazer_requisicao(w.PUBLICAR))
  }
}"#;
    let mut io = TestIo::default();
    io.http_responses = VecDeque::from([
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: b"{\n\"ok\":true\n}".to_vec(),
        },
        HttpResponse {
            status: 204,
            headers: Vec::new(),
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: b"post".to_vec(),
        },
        HttpResponse {
            status: 201,
            headers: Vec::new(),
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: b"custom".to_vec(),
        },
    ]);

    execute(source, &mut io).expect("web service methods should execute through the host");

    assert_eq!(
        io.output,
        "1|2|3|4|{\"ok\":true}|Requisição falhou: código HTTP: 204|post|Requisição falhou: código HTTP: 201|custom"
    );
    assert_eq!(
        io.http_requests
            .iter()
            .map(|request| request.method.as_str())
            .collect::<Vec<_>>(),
        ["GET", "DELETE", "POST", "PUT", "POST"]
    );
    assert_eq!(io.http_requests[2].body, b"{}");
    assert_eq!(io.http_requests[4].body, br#"{"ok":true}"#);
    assert!(
        io.http_requests[4]
            .headers
            .iter()
            .any(|(key, value)| key == "X-Token" && value == "abc")
    );
}

#[test]
fn supports_all_portugol_2_0_calendar_date_and_time_functions() {
    let source = r#"programa {
  inclua biblioteca Calendario --> c
  funcao inicio() {
    escreva(
      c.dia_mes_atual(), "|", c.dia_semana_atual(), "|",
      c.mes_atual(), "|", c.ano_atual(), "|", c.hora_atual(falso), "|",
      c.hora_atual(verdadeiro), "|", c.minuto_atual(), "|",
      c.segundo_atual(), "|", c.milisegundo_atual()
    )
  }
}"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("calendar functions should execute");

    assert_eq!(io.output, "2|2|11|2026|13|1|4|5|678");
}

#[test]
fn supports_utility_helpers_for_vectors_matrices_and_elapsed_time() {
    let source = r#"programa {
  inclua biblioteca Util --> u
  funcao inicio() {
    inteiro valores[3]
    inteiro matriz[2][4]
    u.aguarde(20)
    escreva(u.numero_elementos(valores), "|", u.numero_linhas(matriz), "|", u.numero_colunas(matriz), "|", u.tempo_decorrido())
  }
}"#;
    let mut io = TestIo::default();
    execute(source, &mut io).expect("utility functions should execute");

    let mut pieces = io.output.split('|');
    assert_eq!(pieces.next(), Some("3"));
    assert_eq!(pieces.next(), Some("2"));
    assert_eq!(pieces.next(), Some("4"));
    assert!(pieces.next().unwrap().parse::<i64>().unwrap() >= 10);
}

#[test]
fn delegates_utility_waits_to_the_host_runtime() {
    let source = r#"programa {
  inclua biblioteca Util
  funcao inicio() {
    Util.aguarde(25)
  }
}"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the host should handle the requested wait");

    assert_eq!(io.waits, [Duration::from_millis(25)]);
}

#[test]
fn sorteia_returns_values_inclusive_of_both_bounds() {
    let source = r#"programa {
  inclua biblioteca Util --> u
  funcao inicio() {
    escreva(u.sorteia(-2, 2))
  }
}"#;
    let mut io = TestIo::default();
    execute(source, &mut io).expect("random values should be generated");
    let value = io
        .output
        .parse::<i64>()
        .expect("random result is an integer");

    assert!((-2..=2).contains(&value));
}

#[test]
fn rejects_invalid_random_ranges_and_waits_over_the_configured_limit() {
    let mut io = TestIo::default();
    let random_error = execute(
        r#"programa {
  inclua biblioteca Util
  funcao inicio() {
    Util.sorteia(2, 2)
  }
}"#,
        &mut io,
    )
    .expect_err("an empty random range should be rejected");
    assert!(
        random_error[0]
            .message
            .contains("limites inferior e superior")
    );

    let mut io = TestIo::default();
    let wait_error = execute_with_options(
        r#"programa {
  inclua biblioteca Util
  funcao inicio() {
    Util.aguarde(1)
  }
}"#,
        &mut io,
        ExecutionOptions {
            max_wait_millis: 0,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("wait must respect its configured limit");
    assert!(wait_error[0].message.contains("limite de espera"));
}

#[test]
fn supports_integer_remainder_inequality_and_logical_or() {
    let source = r#"programa {
  funcao inicio() {
    escreva(7 % 2, "|", 7 != 2, "|", falso ou verdadeiro)
  }
}"#;
    assert_eq!(run(source), "1|verdadeiro|verdadeiro");
}

#[test]
fn short_circuit_logic_does_not_evaluate_the_right_side() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n se (falso e (1 / 0 == 1)) {\n escreva(\"erro\")\n } senao {\n escreva(\"certo\")\n }\n }\n}"
        ),
        "certo"
    );
}

#[test]
fn supports_the_portuguese_not_operator() {
    assert_eq!(
        run("programa {\n funcao inicio() {\n se (nao falso) {\n escreva(\"certo\")\n }\n }\n}"),
        "certo"
    );
}

#[test]
fn reports_runtime_errors_with_the_source_line() {
    let source = "programa {\n funcao inicio() {\n inteiro x = 4 / 0\n }\n}";
    let mut io = TestIo::default();
    let error = execute(source, &mut io).expect_err("division by zero should fail");

    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert_eq!(error[0].line, 3);
    assert!(error[0].message.contains("divisão por zero"));
}

#[test]
fn stops_an_infinite_loop_at_the_configured_step_limit() {
    let source = "programa {\n funcao inicio() {\n enquanto (verdadeiro) {}\n }\n}";
    let mut io = TestIo::default();
    let error = execute_with_options(
        source,
        &mut io,
        ExecutionOptions {
            max_steps: 16,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("infinite loop should hit the execution limit");

    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("limite de instruções"));
}

#[test]
fn handles_break_inside_loops_and_rejects_it_outside() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n inteiro i = 0\n enquanto (verdadeiro) {\n i++\n se (i == 3) {\n pare\n }\n }\n escreva(i)\n }\n}"
        ),
        "3"
    );
    let mut io = TestIo::default();
    let error = execute("programa {\n funcao inicio() {\n pare\n }\n}", &mut io)
        .expect_err("break outside a loop should fail");
    assert_eq!(error[0].kind, DiagnosticKind::Type);
}

#[test]
fn supports_the_else_if_form() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n inteiro nota = 8\n se (nota >= 9) {\n escreva(\"A\")\n } senao se (nota >= 7) {\n escreva(\"B\")\n } senao {\n escreva(\"C\")\n }\n }\n}"
        ),
        "B"
    );
}

#[test]
fn preserves_statement_boundaries_inside_multiline_block_comments() {
    assert_eq!(
        run(
            "programa {\n funcao inicio() {\n inteiro valor = 5 /* comentário\n ainda comentário */ escreva(valor)\n }\n}"
        ),
        "5"
    );
}

#[test]
fn reports_a_trailing_escape_instead_of_panicking() {
    let mut io = TestIo::default();
    let source = r#"programa {
 funcao inicio() {
 escreva("texto\"#;
    let error = execute(source, &mut io).expect_err("unterminated escape should be rejected");

    assert_eq!(error[0].kind, DiagnosticKind::Lexical);
}

#[test]
fn reports_excessively_nested_expressions_without_overflowing_the_parser_stack() {
    let expression = format!("{}1{}", "(".repeat(300), ")".repeat(300));
    let source = format!("programa {{ funcao inicio() {{ escreva({expression}) }} }}");
    let mut io = TestIo::default();
    let error = execute(&source, &mut io).expect_err("deep expression must be rejected");

    assert_eq!(error[0].kind, DiagnosticKind::Syntax);
    assert!(error[0].message.contains("aninhamento"));
}

#[test]
fn reports_excessively_nested_blocks_without_overflowing_the_parser_stack() {
    let nested = "se (verdadeiro) { ".repeat(300);
    let closes = " }".repeat(300);
    let source = format!("programa {{ funcao inicio() {{ {nested}escreva(1){closes} }} }}");
    let mut io = TestIo::default();
    let error = execute(&source, &mut io).expect_err("deep blocks must be rejected");

    assert_eq!(error[0].kind, DiagnosticKind::Syntax);
    assert!(error[0].message.contains("blocos aninhados"));
}

#[test]
fn decodes_unicode_octal_and_control_character_escapes() {
    let source = r#"programa {
  funcao inicio() {
    escreva("A\tB|\u00E9|", '\u0021', "|\101|\uD83D\uDE00", "|", '\uD83D\uDE00')
  }
}"#;
    assert_eq!(run(source), "A\tB|é|!|A|😀|😀");
}

#[test]
fn rejects_unicode_escapes_that_are_not_valid_scalar_values() {
    let mut io = TestIo::default();
    let error = execute(
        r#"programa { funcao inicio() { escreva("\uD800") } }"#,
        &mut io,
    )
    .expect_err("unpaired UTF-16 surrogate cannot be represented as a Portugol character");

    assert_eq!(error[0].kind, DiagnosticKind::Lexical);
}

#[test]
fn integer_overflow_wraps_at_the_reference_width() {
    assert_eq!(
        run("programa { funcao inicio() { escreva(2147483647 + 1) } }"),
        "-2147483648"
    );
}

#[test]
fn reads_real_and_string_values_without_losing_text() {
    let mut io = TestIo {
        input: vec!["3,5".to_owned(), "  Ana Maria  ".to_owned()],
        ..TestIo::default()
    };
    execute(
        "programa {\n funcao inicio() {\n real nota\n cadeia nome\n leia(nota)\n leia(nome)\n escreva(nota, \"|\", nome)\n }\n}",
        &mut io,
    )
    .expect("typed input should execute");

    assert_eq!(io.output, "3.5|  Ana Maria  ");
}

#[test]
fn character_input_rejects_supplementary_code_points_that_need_two_utf16_units() {
    let source = "programa { funcao inicio() { caracter valor leia(valor) } }";
    let mut io = TestIo {
        input: vec!["😀".to_owned()],
        ..TestIo::default()
    };

    let errors = execute(source, &mut io).expect_err("Java char accepts one UTF-16 code unit");

    assert!(errors[0].message.contains("exatamente um caractere"));
}

#[test]
fn limpa_clears_output_through_the_host_runtime() {
    assert_eq!(
        run("programa { funcao inicio() { escreva(\"antes\") limpa() escreva(\"depois\") } }"),
        "depois"
    );
}

#[test]
fn rejects_arrays_that_exceed_the_runtime_memory_limit() {
    let mut io = TestIo::default();
    let source = "programa { funcao inicio() { inteiro valores[100001] } }";
    let error = execute(source, &mut io).expect_err("oversized arrays must be rejected");

    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("limite de elementos"));
}

#[test]
fn counts_array_allocations_across_the_whole_program() {
    let mut io = TestIo::default();
    let options = ExecutionOptions {
        max_array_elements: 3,
        ..ExecutionOptions::default()
    };
    let error = execute_with_options(
        "programa { funcao inicio() { inteiro a[2] inteiro b[2] } }",
        &mut io,
        options,
    )
    .expect_err("total array allocations should respect the configured limit");

    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("limite de elementos"));
}

#[test]
fn supports_the_text_library_through_an_alias() {
    let source = r#"programa {
  inclua biblioteca Texto --> t
  funcao inicio() {
    cadeia palavra = "Portugol"
    escreva(
      t.caixa_alta(palavra), "|",
      t.numero_caracteres(palavra), "|",
      t.obter_caracter(palavra, 1), "|",
      t.posicao_texto("g", palavra, 0), "|",
      t.extrair_subtexto(palavra, 0, 4), "|",
      t.substituir(palavra, "P", "C"), "|",
      t.preencher_a_esquerda('*', 10, palavra)
    )
  }
}"#;
    assert_eq!(run(source), "PORTUGOL|8|o|5|Port|Cortugol|**Portugol");
}

#[test]
fn stops_execution_when_the_configured_output_limit_is_reached() {
    let mut io = TestIo::default();
    let options = ExecutionOptions {
        max_output_bytes: 5,
        ..ExecutionOptions::default()
    };
    let error = execute_with_options(
        "programa { funcao inicio() { escreva(\"12345\") escreva(\"6\") } }",
        &mut io,
        options,
    )
    .expect_err("output beyond the configured limit should be rejected");

    assert_eq!(io.output, "12345");
    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("limite de saída"));
}

#[test]
fn supports_common_type_conversions_from_the_types_library() {
    let source = r#"programa {
  inclua biblioteca Tipos --> t
  funcao inicio() {
    cadeia hexadecimal = t.inteiro_para_cadeia(255, 16)
    inteiro valor = t.cadeia_para_inteiro(hexadecimal, 16)
    real nota = t.cadeia_para_real("3.5")
    logico valido = t.cadeia_e_logico("verdadeiro")
    escreva(hexadecimal, "|", valor, "|", nota, "|", valido, "|", t.caracter_para_logico('S'), "|", t.inteiro_para_caracter(7), "|", t.cadeia_e_real("3.5"))
  }
}"#;
    assert_eq!(
        run(source),
        "000000FF|255|3.5|verdadeiro|verdadeiro|7|verdadeiro"
    );
}

#[test]
fn exercises_every_supported_types_library_conversion() {
    let source = r#"programa {
  inclua biblioteca Tipos --> t
  funcao inicio() {
    escreva(
      t.cadeia_e_inteiro("-12", 10), "|", t.cadeia_e_real("-3.5"), "|",
      t.cadeia_e_logico("falso"), "|", t.cadeia_e_caracter("x"), "|",
      t.cadeia_para_caracter("z"), "|", t.cadeia_para_inteiro("-12", 10), "|",
      t.cadeia_para_real("2.5"), "|", t.cadeia_para_logico("falso"), "|",
      t.inteiro_e_caracter(8), "|", t.inteiro_para_cadeia(10, 10), "|",
      t.inteiro_para_caracter(7), "|", t.inteiro_para_logico(-1), "|",
      t.inteiro_para_real(7), "|", t.caracter_e_inteiro('3'), "|",
      t.caracter_e_logico('N'), "|", t.caracter_para_cadeia('a'), "|",
      t.caracter_para_inteiro('8'), "|", t.caracter_para_logico('s'), "|",
      t.logico_para_cadeia(verdadeiro), "|", t.logico_para_inteiro(falso), "|",
      t.logico_para_caracter(falso), "|", t.real_para_cadeia(2.5), "|",
      t.real_para_inteiro(2.9)
    )
  }
}"#;
    assert_eq!(
        run(source),
        "verdadeiro|verdadeiro|verdadeiro|verdadeiro|z|-12|2.5|falso|verdadeiro|10|7|falso|7.0|verdadeiro|verdadeiro|a|8|verdadeiro|verdadeiro|0|N|2.5|2"
    );
}

#[test]
fn supports_binary_and_prefixed_hexadecimal_type_conversions() {
    let source = r#"programa {
  inclua biblioteca Tipos --> t
  funcao inicio() {
    inteiro binario = t.cadeia_para_inteiro("0b1010", 2)
    inteiro hexadecimal = t.cadeia_para_inteiro("0xFF", 16)
    escreva(t.cadeia_e_inteiro("0b1010", 2), "|", binario, "|", hexadecimal, "|", t.inteiro_para_cadeia(-1, 2))
  }
}"#;
    assert_eq!(
        run(source),
        "verdadeiro|10|255|11111111111111111111111111111111"
    );
}

#[test]
fn passes_variables_and_array_elements_by_reference() {
    let source = r#"programa {
  funcao vazio zera_valor(inteiro &valor) {
    valor = 0
  }
  funcao vazio define_valor(inteiro &valor) {
    valor = 9
  }
  funcao inicio() {
    inteiro numero = 123
    inteiro valores[2]
    zera_valor(numero)
    define_valor(valores[1])
    escreva(numero, "|", valores[1])
  }
}"#;
    assert_eq!(run(source), "0|9");
}

#[test]
fn passes_vectors_and_matrices_to_function_parameters() {
    let source = r#"programa {
  funcao inteiro soma_vetor(inteiro valores[]) {
    retorne valores[0] + valores[1]
  }
  funcao inteiro obter_valor(inteiro matriz[][]) {
    retorne matriz[1][0]
  }
  funcao inicio() {
    inteiro valores[2]
    inteiro matriz[2][2]
    valores[0] = 4
    valores[1] = 5
    matriz[1][0] = 6
    escreva(soma_vetor(valores), "|", obter_valor(matriz))
  }
}"#;
    assert_eq!(run(source), "9|6");
}

#[test]
fn passing_an_array_by_value_shares_the_reference_backing_array() {
    let source = r#"programa {
  funcao vazio alterar(inteiro valores[]) {
    valores[0] = 99
  }
  funcao inicio() {
    inteiro valores[1]
    valores[0] = 4
    alterar(valores)
    escreva(valores[0])
  }
}"#;

    assert_eq!(run(source), "99");
}

#[test]
fn supports_prefix_and_postfix_increment_and_unary_plus() {
    let source = r#"programa {
  funcao inicio() {
    inteiro valor = 1
    escreva(++valor, "|", valor--, "|", valor, "|", +valor)
  }
}"#;
    assert_eq!(run(source), "2|2|1|1");
}

#[test]
fn reads_but_does_not_allow_writing_constants() {
    let source = r#"programa {
  const inteiro resposta = 42
  funcao inicio() {
    escreva(resposta)
    resposta = 43
  }
}"#;
    let mut io = TestIo::default();
    let error = execute(source, &mut io).expect_err("constants must not be writable");

    assert!(io.output.is_empty());
    assert_eq!(error[0].kind, DiagnosticKind::Type);
    assert!(error[0].message.contains("constante"));
}

#[test]
fn initializes_vectors_and_matrices_from_literal_values() {
    let source = r#"programa {
  funcao inicio() {
    inteiro valores[3] = {1, 2, 3}
    inteiro matriz[2][2] = {{4, 5}, {6, 7}}
    escreva(valores[2], "|", matriz[1][0])
  }
}"#;
    assert_eq!(run(source), "3|6");
}

#[test]
fn infers_an_unsized_vector_length_from_its_initializer() {
    let source = r#"programa {
  funcao inicio() {
    inteiro valores[] = {8, 9, 10}
    escreva(valores[0], "|", valores[2])
  }
}"#;
    assert_eq!(run(source), "8|10");
}

#[test]
fn uses_constant_expressions_for_array_dimensions() {
    let source = r#"programa {
  const inteiro TAMANHO = 3
  funcao inicio() {
    inteiro valores[TAMANHO] = {4, 5, 6}
    escreva(valores[2])
  }
}"#;
    assert_eq!(run(source), "6");
}

#[test]
fn supports_hexadecimal_bitwise_and_shift_operators() {
    let source = r#"programa {
  funcao inicio() {
    escreva(0x10, "|", 5 & 3, "|", 5 | 2, "|", 5 ^ 1, "|", 1 << 3, "|", 8 >> 2, "|", ~1)
  }
}"#;
    assert_eq!(run(source), "16|1|7|4|8|2|-2");
}

#[test]
fn compares_text_and_character_values() {
    let source = r#"programa {
  funcao inicio() {
    caracter letra = 'a'
    se (letra < 'z' e "ana" < "bia") {
      escreva("certo")
    }
  }
}"#;
    assert_eq!(run(source), "certo");
}

#[test]
fn resolves_overloaded_functions_by_argument_type() {
    let source = r#"programa {
  funcao cadeia identificar(inteiro valor) {
    retorne "inteiro"
  }
  funcao cadeia identificar(cadeia valor) {
    retorne "cadeia"
  }
  funcao inicio() {
    escreva(identificar(7), "|", identificar("sete"))
  }
}"#;
    assert_eq!(run(source), "inteiro|cadeia");
}

#[test]
fn resolves_overloads_by_best_numeric_match() {
    let source = r#"programa {
  funcao cadeia classificar(real valor) {
    retorne "real"
  }
  funcao cadeia classificar(inteiro valor) {
    retorne "inteiro"
  }
  funcao cadeia somente_real(real valor) {
    retorne "real"
  }
  funcao inicio() {
    escreva(classificar(2), "|", somente_real(2))
  }
}"#;
    assert_eq!(run(source), "inteiro|real");
}

#[test]
fn supports_recursive_functions() {
    let source = r#"programa {
  funcao inteiro fatorial(inteiro valor) {
    se (valor <= 1) {
      retorne 1
    }
    retorne valor * fatorial(valor - 1)
  }
  funcao inicio() {
    escreva(fatorial(6))
  }
}"#;
    assert_eq!(run(source), "720");
}

#[test]
fn stops_recursive_calls_at_the_safety_boundary() {
    let source = r#"programa {
  funcao vazio continuar(inteiro valor) {
    se (valor < 600) {
      continuar(valor + 1)
    }
  }
  funcao inicio() {
    continuar(0)
  }
}"#;
    let mut io = TestIo::default();
    let error = execute(source, &mut io).expect_err("unbounded recursion must stop safely");

    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("chamadas recursivas"));
}

#[test]
fn rejects_a_value_returned_from_a_void_function() {
    let source = r#"programa {
  funcao vazio incorreta() {
    retorne 4
  }
  funcao inicio() {
    incorreta()
  }
}"#;
    let mut io = TestIo::default();
    let error = execute(source, &mut io).expect_err("a void function cannot return a value");

    assert_eq!(error[0].kind, DiagnosticKind::Type);
    assert!(error[0].message.contains("não pode retornar um valor"));
}

#[test]
fn allows_statements_after_an_if_without_an_else_branch() {
    let source = r#"programa {
  funcao inicio() {
    se (verdadeiro) {
      escreva("A")
    }
    escreva("B")
  }
}"#;
    assert_eq!(run(source), "AB");
}

#[test]
fn enforces_the_configured_string_limit_before_output() {
    let mut io = TestIo::default();
    let error = execute_with_options(
        r#"programa { funcao inicio() { escreva("texto") } }"#,
        &mut io,
        ExecutionOptions {
            max_string_bytes: 4,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("string values must respect the configured limit");

    assert!(io.output.is_empty());
    assert!(error[0].message.contains("limite de 4 bytes"));
}

#[test]
fn bounds_large_text_padding_and_expanding_replacements() {
    let mut io = TestIo::default();
    let padding_error = execute_with_options(
        r#"programa {
  inclua biblioteca Texto --> t
  funcao inicio() {
    escreva(t.preencher_a_esquerda('x', 10, ""))
  }
}"#,
        &mut io,
        ExecutionOptions {
            max_string_bytes: 4,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("large padding must be rejected without allocating it");
    assert!(padding_error[0].message.contains("limite de"));

    let mut io = TestIo::default();
    let replacement_error = execute_with_options(
        r#"programa {
  inclua biblioteca Texto --> t
  funcao inicio() {
    escreva(t.substituir("aaaa", "a", "1234"))
  }
}"#,
        &mut io,
        ExecutionOptions {
            max_string_bytes: 12,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("an expanding replacement must respect the string limit");
    assert!(replacement_error[0].message.contains("limite de 12 bytes"));
}

#[test]
fn text_library_uses_documented_search_order_and_replaces_every_match() {
    let source = r#"programa {
  inclua biblioteca Texto --> t
  funcao inicio() {
    escreva(t.posicao_texto("a", "banana", 2), "|", t.substituir("banana", "a", "o"))
  }
}"#;
    assert_eq!(run(source), "3|bonono");
}

#[test]
fn math_library_preserves_java_nan_and_infinity_results() {
    let source = r#"programa {
  inclua biblioteca Matematica
  funcao inicio() {
    escreva(Matematica.raiz(-8, 2), "|", Matematica.potencia(10, 1000))
  }
}"#;
    assert_eq!(run(source), "NaN|Infinity");
}

#[test]
fn uses_java_pow_and_math_round_for_negative_values() {
    let source = r#"programa {
  inclua biblioteca Matematica
  funcao inicio() {
    escreva(Matematica.raiz(-27, 3), "|", Matematica.arredondar(-1.5, 0))
  }
}"#;
    assert_eq!(run(source), "NaN|-1.0");
}

#[test]
fn leia_rejects_constants_before_requesting_input() {
    let mut io = TestIo {
        input: vec!["9".to_owned()],
        ..TestIo::default()
    };
    let source = r#"programa {
  const inteiro bloqueado = 3
  funcao inicio() {
    leia(bloqueado)
  }
}"#;
    let error = execute(source, &mut io).expect_err("leia must reject a constant target");

    assert_eq!(io.next_input, 0);
    assert!(error[0].message.contains("constante"));
}

#[test]
fn math_round_saturates_to_java_long_before_scaling() {
    let source = "programa { inclua biblioteca Matematica funcao inicio() { escreva(Matematica.arredondar(100000000000000000000.0, 0)) } }";
    assert_eq!(run(source), "9.223372036854776E18");
}

#[test]
fn dispatches_every_documented_graphics_function_and_preserves_arrays() {
    let source = r#"programa {
  inclua biblioteca Graficos --> g
  funcao inicio() {
    inteiro pontos[2][2] = {{0, 0}, {1, 1}}
    g.iniciar_modo_grafico(verdadeiro)
    g.fechar_janela()
    g.minimizar_janela()
    g.restaurar_janela()
    g.ocultar_borda_janela()
    g.exibir_borda_janela()
    g.encerrar_modo_grafico()
    g.definir_dimensoes_janela(1, 1)
    g.definir_titulo_janela("teste")
    g.limpar()
    g.renderizar()
    g.renderizar_imagem(1, 1)
    g.desenhar_retangulo(1, 1, 1, 1, verdadeiro, verdadeiro)
    g.desenhar_poligono(pontos, verdadeiro)
    g.desenhar_elipse(1, 1, 1, 1, verdadeiro)
    g.desenhar_ponto(1, 1)
    g.desenhar_linha(1, 1, 1, 1)
    g.carregar_imagem("teste")
    g.proximo_frame_gif(1)
    g.transformar_imagem(1, verdadeiro, verdadeiro, 1, 1)
    g.redimensionar_imagem(1, 1, 1, verdadeiro)
    g.obter_cor_pixel(1, 1, 1)
    g.obter_RGB(1, 1)
    g.transformar_porcao_imagem(1, 1, 1, 1, 1, verdadeiro, verdadeiro, 1, 1)
    g.desenhar_imagem(1, 1, 1)
    g.salvar_imagem(1, "teste")
    g.desenhar_quadro_atual_gif(1, 1, 1)
    g.desenhar_porcao_imagem(1, 1, 1, 1, 1, 1, 1)
    g.obter_intervalo_gif(1)
    g.obter_numero_quadros_gif(1)
    g.obter_numero_quadro_atual_gif(1)
    g.obter_quadro_gif(1, 1)
    g.definir_quadro_gif(1, 1)
    g.liberar_imagem(1)
    g.desenhar_texto(1, 1, "teste")
    g.definir_cor(1)
    g.definir_gradiente(1, 1, 1)
    g.definir_fonte_texto("teste")
    g.definir_tamanho_texto(1.5)
    g.definir_estilo_texto(verdadeiro, verdadeiro, verdadeiro)
    g.largura_texto("teste")
    g.altura_texto("teste")
    g.largura_imagem(1)
    g.altura_imagem(1)
    g.criar_cor(1, 1, 1)
    g.carregar_fonte("teste")
    g.definir_icone_janela(1)
    g.definir_opacidade(1)
    g.definir_rotacao(1)
    g.largura_janela()
    g.altura_janela()
    g.largura_tela()
    g.altura_tela()
    g.entrar_modo_tela_cheia()
    g.sair_modo_tela_cheia()
    escreva(g.COR_PRETO, "|", g.COR_BRANCO, "|", g.COR_AZUL, "|", g.COR_VERMELHO, "|", g.COR_VERDE, "|", g.COR_AMARELO, "|", g.GRADIENTE_SUPERIOR_ESQUERDO, "|", g.CANAL_B)
  }
}"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("graphics functions should be delegated to the host");

    let names = io
        .graphics_calls
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "iniciar_modo_grafico",
            "fechar_janela",
            "minimizar_janela",
            "restaurar_janela",
            "ocultar_borda_janela",
            "exibir_borda_janela",
            "encerrar_modo_grafico",
            "definir_dimensoes_janela",
            "definir_titulo_janela",
            "limpar",
            "renderizar",
            "renderizar_imagem",
            "desenhar_retangulo",
            "desenhar_poligono",
            "desenhar_elipse",
            "desenhar_ponto",
            "desenhar_linha",
            "carregar_imagem",
            "proximo_frame_gif",
            "transformar_imagem",
            "redimensionar_imagem",
            "obter_cor_pixel",
            "obter_rgb",
            "transformar_porcao_imagem",
            "desenhar_imagem",
            "salvar_imagem",
            "desenhar_quadro_atual_gif",
            "desenhar_porcao_imagem",
            "obter_intervalo_gif",
            "obter_numero_quadros_gif",
            "obter_numero_quadro_atual_gif",
            "obter_quadro_gif",
            "definir_quadro_gif",
            "liberar_imagem",
            "desenhar_texto",
            "definir_cor",
            "definir_gradiente",
            "definir_fonte_texto",
            "definir_tamanho_texto",
            "definir_estilo_texto",
            "largura_texto",
            "altura_texto",
            "largura_imagem",
            "altura_imagem",
            "criar_cor",
            "carregar_fonte",
            "definir_icone_janela",
            "definir_opacidade",
            "definir_rotacao",
            "largura_janela",
            "altura_janela",
            "largura_tela",
            "altura_tela",
            "entrar_modo_tela_cheia",
            "sair_modo_tela_cheia",
        ]
    );
    let points = io
        .graphics_calls
        .iter()
        .find(|(name, _)| name == "desenhar_poligono")
        .unwrap();
    assert_eq!(
        points.1[0],
        HostValue::Array(vec![
            HostValue::Array(vec![HostValue::Integer(0), HostValue::Integer(0)]),
            HostValue::Array(vec![HostValue::Integer(1), HostValue::Integer(1)]),
        ])
    );
    assert_eq!(
        io.output,
        "-16777216|-1|-16776961|-65536|-16711936|-256|7|2"
    );
}

#[test]
fn dispatches_every_documented_audio_function() {
    let source = r#"programa {
  inclua biblioteca Sons --> s
  funcao inicio() {
    s.carregar_som("teste")
    s.liberar_som(1)
    s.reproduzir_som(1, verdadeiro)
    s.pausar_som(1)
    s.interromper_som(1)
    s.obter_tamanho_musica(1)
    s.obter_posicao_atual_musica(1)
    s.definir_posicao_atual_musica(1, 1)
    s.definir_volume_reproducao(1, 1)
    s.definir_volume(1)
    s.obter_volume()
    s.obter_volume_reproducao(1)
  }
}"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("audio functions should be delegated to the host");

    let names = io
        .audio_calls
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "carregar_som",
            "liberar_som",
            "reproduzir_som",
            "pausar_som",
            "interromper_som",
            "obter_tamanho_musica",
            "obter_posicao_atual_musica",
            "definir_posicao_atual_musica",
            "definir_volume_reproducao",
            "definir_volume",
            "obter_volume",
            "obter_volume_reproducao",
        ]
    );
}
