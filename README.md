# Potoo IDE

Uma IDE desktop para aprender lógica de programação usando pseudolinguagens em português.

O projeto busca oferecer um espaço simples e intuitivo para estudantes praticarem e professores acompanharem os conceitos apresentados no código. A interface é feita em Rust, GTK 4 e libadwaita, com inspiração no GNOME Builder.

## Linguagens

- **Portugol** e **VisuAlg** são as linguagens previstas para a primeira etapa.
- **PSeInt** e **Linotte** são possibilidades para uma etapa futura.

A estrutura atual é inicial e ainda não inclui editor de código nem interpretadores. O trabalho de compatibilidade e integração será feito em etapas.

## Plataformas

O objetivo é oferecer o Potoo IDE para Linux, Windows e macOS. Esta base prepara primeiro a distribuição Linux via Flatpak; o suporte e os instaladores para as outras plataformas ainda serão desenvolvidos e validados.

## Desenvolvimento

Requisitos para executar a versão nativa: Rust 1.92 ou posterior, GTK 4.20 e libadwaita 1.8 com as bibliotecas de desenvolvimento da plataforma.

```sh
cargo run --locked
```

Para gerar e instalar uma versão Flatpak localmente, com Flatpak e Flatpak Builder instalados:

```sh
flatpak-builder --user --install --force-clean _build io.github.diegopn.potooide.json
```

## Contribuição

Relatos de problemas, sugestões e contribuições são bem-vindos. Consulte [CONTRIBUTING.md](CONTRIBUTING.md) antes de abrir uma issue ou pull request.

## Licença

O código do Potoo IDE é distribuído sob a licença GNU GPL versão 3 ou, a seu critério, qualquer versão posterior. Consulte [LICENSE](LICENSE).

## Links

- [Site do projeto](https://diegopn.github.io/potoo-site/)
- [Repositório no GitHub](https://github.com/diegopn/potoo-ide)
- [Patrocine o desenvolvimento](https://github.com/sponsors/diegopn)
