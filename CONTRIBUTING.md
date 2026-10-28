# Como contribuir

Toda ajuda é bem-vinda: relatar um problema, testar o aplicativo, melhorar a documentação ou contribuir com código. A estrutura ainda está no começo, então sugestões sobre a experiência de estudantes e professores também ajudam.

## Relatar um problema

Abra uma [issue](https://github.com/diegopn/potoo-ide/issues) com:

- sistema operacional e versão;
- o que você tentou fazer;
- o que esperava que acontecesse e o que aconteceu;
- passos para reproduzir o problema.

Use exemplos criados para o relato. Não envie documentos pessoais, dados privados ou credenciais.

## Preparar o ambiente

Instale Rust 1.92 ou posterior e as bibliotecas de desenvolvimento de GTK 4.20 e libadwaita 1.8 disponíveis para sua distribuição. Depois, execute:

```sh
cargo run --locked
```

Antes de abrir um pull request, formate o código e confira se o projeto compila:

```sh
cargo fmt --all
cargo check --locked
```

## Pull requests

- Mantenha cada pull request focado em uma mudança.
- Descreva o problema ou melhoria e como revisar o resultado.
- Informe os comandos de verificação que executou.
- Direcione as mudanças de desenvolvimento para `develop`; `main` fica reservada para versões estáveis.
- Ao incluir código ou arquivos de terceiros, confira a compatibilidade de licença e preserve os avisos de autoria e licença.

## Licença

As contribuições ao código do Potoo IDE serão distribuídas sob a GNU GPL versão 3 ou posterior, conforme [LICENSE](LICENSE).
