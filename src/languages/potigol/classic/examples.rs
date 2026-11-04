pub(crate) const SAMPLE: &str = r#"media(a: Real, b: Real, c: Real) = (a + b + c) / 3

escreva "Digite três notas:"
a, b, c = leia_numero
resultado = media(a, b, c)

se resultado >= 7 então
  escreva "Aprovada: {resultado}"
senão
  escreva "Em recuperação: {resultado}"
fim"#;
