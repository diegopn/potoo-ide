pub(crate) const SAMPLE: &str = r#"variaveis
    valor_a: real;
    valor_b: real;
    media: real;
inicio
    escrever "Primeiro valor:";
    ler valor_a;
    escrever "Segundo valor:";
    ler valor_b;
    media <- (valor_a + valor_b) / 2;
    escrever "Média: ", media;
    se media >= 7 entao
        escrever "Resultado suficiente";
    senao
        escrever "Revise o conteúdo";
    fim se;
fim."#;
