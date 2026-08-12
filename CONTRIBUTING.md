# Contribuindo com o AgendarX

Obrigado por contribuir. Mudanças pequenas, com escopo claro e testes proporcionais
ao risco, são mais fáceis de revisar.

## Preparação

Requisitos de desenvolvimento:

- Rust estável com `rustfmt` e `clippy`;
- Node.js 24 e npm;
- SQLite, fornecido pela dependência Rust;
- Docker, opcional para validar a imagem final.

```bash
git clone https://github.com/facrf/agendarx.git
cd agendarx
cp .env.example .env
cd frontend && npm ci && npm run build && cd ..
cargo run
```

A aplicação inicia em `http://localhost:12000`.

## Antes do pull request

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cd frontend
npm run lint
npm run build
```

Ao alterar tabelas, crie uma nova migração numerada; não modifique migrações já
publicadas. Ao alterar contratos HTTP, atualize os tipos TypeScript e a documentação.

## Commits e pull requests

- Use uma mensagem curta no imperativo, por exemplo: `Adiciona filtro por categoria`.
- Não inclua `.env`, bancos SQLite, anexos, `target`, `node_modules` ou `dist`.
- Explique motivação, impacto, testes executados e qualquer migração necessária.
- Relacione a issue correspondente quando existir.

Não envie dados pessoais reais em testes, exemplos, capturas de tela ou fixtures.
Abra propostas e correções em
[github.com/facrf/agendarx/issues](https://github.com/facrf/agendarx/issues).
