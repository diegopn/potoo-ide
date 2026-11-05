pub(crate) const SAMPLE: &str = r#"programa {
    funcao logico eh_par(inteiro numero) {
        retorne numero % 2 == 0
    }

    funcao inicio() {
        inteiro valor
        escreva("Digite um número inteiro: ")
        leia(valor)

        se (eh_par(valor)) {
            escreva("O número é par\n")
        } senao {
            escreva("O número é ímpar\n")
        }
    }
}"#;
