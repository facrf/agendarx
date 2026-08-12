# AgendarX

Agenda de contatos e dossiê interpessoal com autenticação, anexos multimídia,
grafo de relacionamentos e pesquisa em fontes públicas. O projeto reúne uma API
Rust/Axum, SQLite embarcado e uma interface React responsiva em um único serviço.

## Funcionalidades

- Cadastro de pessoas, categorias coloridas e meios de contato dinâmicos.
- Fotos, áudio, PDFs, documentos e outros anexos armazenados no dossiê.
- Streaming de mídia com suporte a HTTP Range.
- Grafo interativo de vínculos com layouts de teia e hierárquico.
- Parâmetros OSINT por pessoa, histórico de achados e arquivamento de PDFs.
- Senhas com Argon2 e sessões JWT revogáveis persistidas no SQLite.
- Interface React, TypeScript, Tailwind CSS, Lucide e Cytoscape.js.
- Migrações automáticas e imagem Docker executada como usuário sem privilégios.

## Início rápido

Requer Rust estável e Node.js 24.

```bash
cp .env.example .env
# Defina JWT_SECRET, ADMIN_LOGIN e ADMIN_PASSWORD em .env.

cd frontend
npm ci
npm run build
cd ..

cargo run
```

Abra `http://localhost:12000`. As migrações são aplicadas automaticamente e o
usuário inicial é criado quando `ADMIN_LOGIN` e `ADMIN_PASSWORD` estão definidos.

Para desenvolver o frontend com hot reload:

```bash
cd frontend
npm run dev
```

O Vite usa `http://localhost:5173` e encaminha `/api` e `/health` para a API na
porta `12000`. Defina `VITE_API_PROXY_TARGET` para usar outro destino.

## Docker

```bash
docker build -t agendarx .
docker run --rm -p 12000:12000 \
  -v agendarx-data:/app/data \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  -e ADMIN_LOGIN=admin \
  -e ADMIN_PASSWORD='uma-senha-forte' \
  agendarx
```

A porta interna e externa padrão é `12000`. O banco e todos os anexos ficam no
volume montado em `/app/data`.

## Exemplo para Portainer

O exemplo completo e endurecido está em
[`deploy/portainer-stack.yml`](deploy/portainer-stack.yml). Um Stack mínimo é:

```yaml
services:
  agendarx:
    image: ghcr.io/seu-usuario/agendarx:latest
    restart: unless-stopped
    ports:
      - "12000:12000"
    environment:
      SERVER_ADDR: 0.0.0.0:12000
      DATABASE_URL: sqlite://data/agendarx.db?mode=rwc
      FRONTEND_DIR: /app/frontend
      JWT_SECRET: troque-por-um-segredo-com-32-ou-mais-caracteres
      ADMIN_LOGIN: admin
      ADMIN_PASSWORD: troque-por-uma-senha-forte
      COOKIE_SECURE: "false"
    volumes:
      - agendarx_data:/app/data

volumes:
  agendarx_data:
```

No Portainer, abra **Stacks > Add stack > Web editor**, cole o arquivo completo e
preencha as variáveis conforme [`portainer.env.example`](deploy/portainer.env.example).
Se o pacote GHCR for privado, registre primeiro as credenciais em **Registries**.

## Pesquisa pública

Defina `SEARXNG_URL` com a URL-base de uma instância SearXNG que tenha a saída JSON
habilitada. Nomes e identificadores são pesquisados entre aspas; termos livres são
enviados como configurados. Achados são deduplicados pela URL.

Downloads automáticos de PDF têm timeout e limite de tamanho, não seguem
redirecionamentos, rejeitam destinos locais/privados/reservados, fixam a resolução
DNS validada e verificam a assinatura do arquivo antes de persistir o BLOB.

Use esse módulo apenas para dados de acesso público, com finalidade legítima e em
conformidade com a legislação aplicável. O sistema não contorna autenticação,
CAPTCHA, paywalls ou controles de acesso.

## Versões e arquiteturas

Ao enviar uma tag `vMAJOR.MINOR.PATCH`, o GitHub Actions valida a versão, cria uma
GitHub Release e publica:

| Arquitetura | Pacote executável | Imagem GHCR |
|---|---:|---:|
| Intel/AMD 64 bits (`amd64`) | Sim | Sim |
| ARM 64 bits (`arm64`) | Sim | Sim |
| ARM 32 bits v7 (`armv7`) | Sim | Sim |
| RISC-V 64 bits (`riscv64gc`) | Sim | Não |

RISC-V é distribuído como pacote binário porque as imagens-base oficiais usadas no
Dockerfile ainda não oferecem essa plataforma. Todos os pacotes recebem checksum
SHA-256. Consulte [Versões e arquiteturas](docs/RELEASES.md) para publicar e instalar.

## Documentação

- [Arquitetura e decisões técnicas](docs/ARQUITETURA.md)
- [API REST](docs/API.md)
- [Implantação, Portainer, backup e atualização](docs/IMPLANTACAO.md)
- [Versões, GHCR e pacotes multi-arquitetura](docs/RELEASES.md)
- [Como contribuir](CONTRIBUTING.md)
- [Política de segurança](SECURITY.md)
- [Histórico de alterações](CHANGELOG.md)

## Estrutura do projeto

```text
src/                 backend Rust/Axum
migrations/          migrações SQLite
frontend/src/        aplicação React/TypeScript
deploy/              arquivos de implantação
docs/                documentação técnica e operacional
.github/workflows/   integração contínua e publicação
```

## Verificação local

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cd frontend && npm run lint && npm run build
```

Detalhes do processo de contribuição estão em [CONTRIBUTING.md](CONTRIBUTING.md).
