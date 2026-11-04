pub(crate) const SAMPLE: &str = r#"algoritmo contagem;
variáveis
    limite, numero : inteiro;
fim-variáveis

início
    imprima("Digite o limite:");
    limite := leia();

    para numero de 1 até limite faça
        se numero % 2 = 0 então
            imprima(numero, " é par.");
        senão
            imprima(numero, " é ímpar.");
        fim-se
    fim-para
fim"#;
