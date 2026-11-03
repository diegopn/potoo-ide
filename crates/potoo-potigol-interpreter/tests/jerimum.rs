use std::time::Duration;

use potoo_potigol_interpreter::{InputPrompt, JerimumRequest, JerimumValue, RuntimeIo, execute};

#[derive(Default)]
struct TestIo {
    output: String,
    requests: Vec<JerimumRequest>,
    open_checks: usize,
    next_handle: i64,
    fail_window_check: bool,
    stop_after_wait: bool,
    stop_requested: bool,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
        Err("unexpected input request".to_owned())
    }

    fn wait(&mut self, _duration: Duration) -> Result<(), String> {
        if self.stop_after_wait {
            self.stop_requested = true;
        }
        Ok(())
    }

    fn should_stop(&mut self) -> bool {
        self.stop_requested
    }

    fn jerimum_request(&mut self, request: JerimumRequest) -> Result<JerimumValue, String> {
        self.requests.push(request.clone());
        if self.fail_window_check && matches!(request, JerimumRequest::WindowIsOpen) {
            return Err("host de teste indisponível".to_owned());
        }
        match request {
            JerimumRequest::Graphics { function, .. } => Ok(match function.as_str() {
                "carregar_imagem" => JerimumValue::Integer(10),
                "largura_imagem" | "altura_imagem" => JerimumValue::Integer(4),
                "obter_cor_pixel" => JerimumValue::Integer(i64::from(0xff000000u32 as i32)),
                "transformar_porcao_imagem" => {
                    self.next_handle += 1;
                    JerimumValue::Integer(self.next_handle + 100)
                }
                _ => JerimumValue::Null,
            }),
            JerimumRequest::Audio {
                function,
                arguments,
            } => Ok(match function.as_str() {
                "carregar_som" | "carregar_musica" => JerimumValue::Integer(20),
                "som_tocando" | "musica_pausada" => JerimumValue::Logical(true),
                "reproduzir_som" => arguments.first().cloned().unwrap_or(JerimumValue::Null),
                _ => JerimumValue::Null,
            }),
            JerimumRequest::WindowIsOpen => {
                self.open_checks += 1;
                Ok(JerimumValue::Logical(self.open_checks <= 2))
            }
            JerimumRequest::KeyPressed { key } => Ok(JerimumValue::Logical(key == 65)),
            JerimumRequest::MouseButtonPressed { button } => Ok(JerimumValue::Logical(button == 1)),
            JerimumRequest::MousePosition => Ok(JerimumValue::Array(vec![
                JerimumValue::Integer(12),
                JerimumValue::Integer(34),
            ])),
            JerimumRequest::ReadFileLines { .. } => Ok(JerimumValue::Array(vec![
                JerimumValue::String("linha 1".to_owned()),
                JerimumValue::String("linha 2".to_owned()),
            ])),
        }
    }
}

#[test]
fn importing_jerimum_enables_game_geometry_aliases() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\nescreva jogo.distancia(0.0, 0.0, 3.0, 4.0)\nescreva jogo.projeção_X(90.0, 2.0)\nescreva jogo.projecao_Y(0.0, 2.0)",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "5.0\n2.0\n-2.0\n");
}

#[test]
fn unknown_library_imports_are_rejected() {
    let mut io = TestIo::default();
    assert!(execute("use \"naoexiste\"\nescreva 1", &mut io).is_err());
    assert!(execute("escreva jogo.distancia(0.0, 0.0, 3.0, 4.0)", &mut io).is_err());
}

#[test]
fn jerimum_color_animation_images_and_project_files_work_with_a_fake_host() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\ncor_rgb: Cor = cor(12, 34, 56)\nimagem = Imagem(\"sprites.png\")\nquadros = imagem.fatie(2, 2)\nanimacao = Animacao(1000, quadros)\nescreva \"{cor_rgb.vermelho},{cor_rgb.verde},{cor_rgb.azul};{imagem.largura},{imagem.altura};{quadros.tamanho};{animacao.imagem().largura}\"\nescreva Arquivo.leia(\"dados.txt\")",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "12,34,56;4,4;4;2\n[linha 1, linha 2]\n");
    assert!(io.requests.contains(&JerimumRequest::ReadFileLines {
        path: "dados.txt".to_owned(),
    }));
}

#[test]
fn jerimum_companions_aliases_and_public_case_class_fields_work() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\nescreva Cor.BRANCO.vermelho\nimagem = Imagem(\"sprites.png\")\nfatiar = Imagem.fatie\nquadros = fatiar(\"sprites.png\", 2, 2)\nanimacao = Animação(100, quadros)\nfonte = Fonte(18)\nsom = Som(\"efeito.wav\")\nmusica = Música(\"fundo.wav\")\nescreva \"{imagem.caminho};{quadros.tamanho};{animacao.velocidade};{animacao.imagens.tamanho};{animacao.imagem().largura};{fonte.tamanho};{som.arquivo};{musica.arquivo}\"\nescreva Relógio.milisegundos\nescreva relógio.milisegundos\nescreva Relogio.milisegundos\nescreva relogio.milisegundos",
        &mut io,
    )
    .unwrap();

    assert_eq!(
        io.output,
        "255\nsprites.png;4;100;4;2;18;efeito.wav;fundo.wav\n0\n0\n0\n0\n"
    );
    assert_eq!(
        io.requests
            .iter()
            .filter(|request| matches!(request, JerimumRequest::Graphics { function, .. } if function == "carregar_imagem"))
            .count(),
        1
    );
}

#[test]
fn jerimum_keyboard_and_mouse_aliases_query_the_host() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\nescreva teclado.TECLA_A\nescreva mouse.BOTAO_ESQUERDO\nescreva \"{mouse.x},{mouse.y}\"",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "verdadeiro\nverdadeiro\n12,34\n");
}

#[test]
fn jerimum_static_game_properties_and_name_aliases_work() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\njogo.titulo := \"Teste\"\njogo.largura := 800\njogo.altura := 600\njogo.fps := 30\nescreva Jogo.distancia(0.0, 0.0, 3.0, 4.0)\nescreva \"{jogo.titulo};{jogo.largura};{jogo.altura};{jogo.fps}\"\nescreva teclado.TECLA_0\nescreva Teclado.TECLA_ESPAÇO\nescreva mouse.BOTAO_DIREITO\nescreva Mouse.BOTAO_ESQUERDO",
        &mut io,
    )
    .unwrap();

    assert_eq!(
        io.output,
        "5.0\nTeste;800;600;30\nfalso\nfalso\nfalso\nverdadeiro\n"
    );
}

#[test]
fn jerimum_type_names_expose_only_companion_members() {
    let mut io = TestIo::default();
    assert!(execute("use \"jerimum\"\nf = Fonte.desenhe", &mut io).is_err());
    assert!(
        execute(
            "use \"jerimum\"\ncor = Cor(1, 2, 3)\nescreva cor.AMARELO",
            &mut io
        )
        .is_err()
    );
}

#[test]
fn jerimum_cor_color_exposes_native_rgb_channels() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\ncor = Cor(12, 34, 56)\nescreva \"{cor.color.getRed()},{cor.color.getGreen()},{cor.color.getBlue()},{cor.color.getAlpha()}\"",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "12,34,56,255\n");
}

#[test]
fn jerimum_image_buffer_exposes_dimensions_and_draws_through_the_host() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\nimagem = Imagem(\"sprites.png\")\nbuffer = imagem.buffer\nrecorte = buffer.getSubimage(1, 1, 2, 2)\nescreva \"{buffer.getWidth()},{buffer.getHeight()},{recorte.getWidth()},{recorte.getHeight()},{buffer.getRGB(0, 0)}\"",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "4,4,2,2,-16777216\n");
    assert!(io.requests.iter().any(|request| matches!(
        request,
        JerimumRequest::Graphics { function, .. } if function == "obter_cor_pixel"
    )));
}

#[test]
fn jerimum_tela_exposes_native_frame_and_canvas_handles() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\ntela = Tela(\"Janela\", 320, 200)\nescreva \"{tela.frame.getTitle()};{tela.frame.getWidth()},{tela.frame.getHeight()};{tela.canvas.getWidth()},{tela.canvas.getHeight()}\"",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "Janela;320,200;320,200\n");
}

#[test]
fn jerimum_drawing_callbacks_receive_a_context_and_draw_in_reference_order() {
    let mut io = TestIo::default();
    execute(
        r#"use "jerimum"
imagem = Imagem("sprites.png")
mais_antigo(g: Graphics2D): Nada
  g.setColor(Cor.VERMELHO.color)
  g.drawLine(1, 2, 3, 4)
fim
mais_novo(g: Graphics2D): Nada
  g.setColor(Cor.AZUL.color)
  g.fillRect(5, 6, 7, 8)
  g.drawImage(imagem.buffer, 10, 20)
  g.setColor(Cor.AZUL.color)
fim
baixa(g: Graphics2D): Nada
  g.drawString("baixo", 2, 3)
  Desenho.desenhe(g)
fim
atualize(): Nada
fim
desenhe(): Nada
  Desenho.incluir(9, mais_antigo)
  Desenho.incluir(9, mais_novo)
  Desenho.incluir(2, baixa)
fim
jogo.iniciar("Teste", 320, 200, atualize, desenhe, 60)"#,
        &mut io,
    )
    .unwrap();

    let draw_calls = io
        .requests
        .iter()
        .filter_map(|request| match request {
            JerimumRequest::Graphics { function, .. }
                if matches!(
                    function.as_str(),
                    "desenhar_texto" | "desenhar_retangulo" | "desenhar_linha" | "desenhar_imagem"
                ) =>
            {
                Some(function.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        draw_calls,
        [
            "desenhar_texto",
            "desenhar_retangulo",
            "desenhar_imagem",
            "desenhar_linha"
        ]
    );
    let colors = io
        .requests
        .iter()
        .filter_map(|request| match request {
            JerimumRequest::Graphics {
                function,
                arguments,
            } if function == "definir_cor" => arguments.first().cloned(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(colors.windows(4).any(|window| {
        window
            == [
                JerimumValue::Integer(i64::from(0xff000000u32 as i32)),
                JerimumValue::Integer(i64::from(0xff0000ffu32 as i32)),
                JerimumValue::Integer(i64::from(0xff0000ffu32 as i32)),
                JerimumValue::Integer(i64::from(0xffff0000u32 as i32)),
            ]
    }));
}

#[test]
fn jerimum_game_runs_update_draw_and_stops_when_the_window_closes() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\nvar quadros := 0\nvar desenhos := 0\natualize(): Nada\n  quadros := quadros + 1\nfim\ntexto(g: Graphics2D): Nada\n  desenhos := desenhos + 1\n  g.drawLine(1, 2, 3, 4)\n\nfim\ndesenhe(): Nada\n  Desenho.incluir(3, texto)\n  Fonte(12).desenhe(\"olá\", 20.0, 30.0, 2)\nfim\njogo.iniciar(\"Teste\", 320, 200, atualize, desenhe, 60)\nescreva \"{quadros}:{desenhos}\"",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "2:1\n");
    assert!(io.requests.contains(&JerimumRequest::WindowIsOpen));
    assert!(io.requests.iter().any(|request| matches!(
        request,
        JerimumRequest::Graphics { function, .. } if function == "jerimum_desenhar_texto"
    )));
}

#[test]
fn jerimum_tela_initializes_the_native_graphics_host() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\ntela = Tela(\"Janela\", 320, 200)\nescreva tela.frame.getTitle()",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "Janela\n");
    assert!(io.requests.contains(&JerimumRequest::Graphics {
        function: "iniciar_modo_grafico".to_owned(),
        arguments: vec![JerimumValue::Logical(true)],
    }));
    assert!(io.requests.contains(&JerimumRequest::Graphics {
        function: "definir_dimensoes_janela".to_owned(),
        arguments: vec![JerimumValue::Integer(320), JerimumValue::Integer(200)],
    }));
}

#[test]
fn jerimum_game_observes_cancellation_and_propagates_host_errors() {
    let mut stopped = TestIo {
        stop_after_wait: true,
        ..TestIo::default()
    };
    let stopped_error = execute(
        "use \"jerimum\"\nvar quadros := 0\natualize(): Nada\n  quadros := quadros + 1\nfim\njogo.iniciar(\"Teste\", 320, 200, atualize)\nescreva quadros",
        &mut stopped,
    )
    .unwrap_err();
    assert!(stopped_error[0].message.contains("execução interrompida"));

    let mut failing = TestIo {
        fail_window_check: true,
        ..TestIo::default()
    };
    let error = execute("use \"jerimum\"\njogo.iniciar(\"Teste\")", &mut failing).unwrap_err();
    assert!(error[0].message.contains("host de teste indisponível"));
}

#[test]
fn jerimum_sound_and_music_methods_are_host_backed() {
    let mut io = TestIo::default();
    execute(
        "use \"jerimum\"\nsom = Som(\"efeito.wav\")\nsom.toque()\nescreva som.tocando\nmusica = Música(\"fundo.wav\")\nmusica.toque(falso)\nescreva musica.pausado\nmusica.pause\nmusica.continue\nmusica.pare\nmusica.feche",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "verdadeiro\nverdadeiro\n");
    assert!(io.requests.iter().any(|request| matches!(
        request,
        JerimumRequest::Audio { function, .. } if function == "continuar_som"
    )));
    assert!(io.requests.iter().any(|request| matches!(
        request,
        JerimumRequest::Audio { function, .. } if function == "carregar_musica"
    )));
}
