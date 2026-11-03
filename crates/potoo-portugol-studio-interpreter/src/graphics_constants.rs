/// Returns the integer value of a Portugol 2.0 `Graficos` constant.
pub(crate) fn constant_value(name: &str) -> Option<i64> {
    Some(match name {
        "COR_PRETO" => -16_777_216,
        "COR_BRANCO" => -1,
        "COR_AZUL" => -16_776_961,
        "COR_VERMELHO" => -65_536,
        "COR_VERDE" => -16_711_936,
        "COR_AMARELO" => -256,
        "GRADIENTE_DIREITA" => 0,
        "GRADIENTE_ESQUERDA" => 1,
        "GRADIENTE_ACIMA" => 2,
        "GRADIENTE_ABAIXO" => 3,
        "GRADIENTE_INFERIOR_DIREITO" => 4,
        "GRADIENTE_INFERIOR_ESQUERDO" => 5,
        "GRADIENTE_SUPERIOR_DIREITO" => 6,
        "GRADIENTE_SUPERIOR_ESQUERDO" => 7,
        "CANAL_R" => 0,
        "CANAL_G" => 1,
        "CANAL_B" => 2,
        _ => return None,
    })
}
