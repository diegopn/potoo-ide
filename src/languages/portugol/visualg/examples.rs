pub(crate) const SAMPLE: &str = r#"algoritmo "Resultado da turma"
var
    nota1, nota2, nota3, media: real
funcao aprovado(media: real): logico
inicio
    retorne media >= 7
fimfuncao

inicio
    escreval("Digite três notas:")
    leia(nota1, nota2, nota3)
    media <- (nota1 + nota2 + nota3) / 3

    se aprovado(media) entao
        escreval("Aprovado com média ", media:5:2)
    senao
        escreval("Estude mais: média ", media:5:2)
    fimse
fimalgoritmo"#;
