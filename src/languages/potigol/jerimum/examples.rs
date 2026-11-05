pub(crate) const SAMPLE: &str = r#"use "jerimum"

# Mova o quadrado verde com WASD ou as setas.
var jogador_x := 144
var jogador_y := 84

atualize(): Nada
  jogador_x := se (teclado.TECLA_D ou teclado.TECLA_PARA_DIREITA) e jogador_x < 288 então jogador_x + 4 senão jogador_x fim
  jogador_x := se (teclado.TECLA_A ou teclado.TECLA_PARA_ESQUERDA) e jogador_x > 0 então jogador_x - 4 senão jogador_x fim
  jogador_y := se (teclado.TECLA_S ou teclado.TECLA_PARA_BAIXO) e jogador_y < 168 então jogador_y + 4 senão jogador_y fim
  jogador_y := se (teclado.TECLA_W ou teclado.TECLA_PARA_CIMA) e jogador_y > 0 então jogador_y - 4 senão jogador_y fim
fim

desenhe_personagem(g: Graphics2D): Nada
  g.setColor(Cor.VERDE.color)
  g.fillRect(jogador_x, jogador_y, 32, 32)
  g.setColor(Cor.PRETO.color)
  g.drawString("WASD / setas: mover", 8, 20)
fim

desenhe(): Nada
  Desenho.incluir(1, desenhe_personagem)
fim

jogo.iniciar("Mova o quadrado", 320, 200, atualize, desenhe, 60)"#;
