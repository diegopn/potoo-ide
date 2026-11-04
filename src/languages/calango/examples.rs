pub(crate) const SAMPLE: &str = r#"algoritmo mediaTurma;
principal
    inteiro i;
    real notas[3];
    real soma;
    soma = 0;
    para i de 0 ate 2 passo 1 faca
        escreva("Nota: ");
        leia(notas[i]);
        soma = soma + notas[i];
    fimPara
    se (soma / 3 >= 7) entao
        escreval("Aprovado");
    senao
        escreval("Continue estudando");
    fimSe
fimPrincipal"#;
