# AgendarX

Agenda de contatos e dossiê interpessoal com autenticação, anexos multimídia,
grafo de relacionamentos e pesquisa em fontes públicas. O projeto reúne uma API
Rust/Axum, SQLite embarcado e uma interface React responsiva em um único serviço.

Código-fonte e versões: [github.com/facrf/agendarx](https://github.com/facrf/agendarx).
Automação de publicação: [workflow Publicar versão](https://github.com/facrf/agendarx/actions/workflows/release.yml).

## Funcionalidades

- Cadastro de pessoas, categorias coloridas e meios de contato dinâmicos.
- Calendário mensal responsivo para criar, editar e concluir tarefas com horários,
  prioridades, cores e vínculo com uma ou mais pessoas cadastradas.
- Upload múltiplo de imagens, vídeos, áudio, PDFs, textos e outros anexos, com
  miniaturas, pré-visualização, renomeação e download no dossiê.
- Streaming de mídia com suporte a HTTP Range.
- Grafo interativo de vínculos com layouts de teia e hierárquico; ao selecionar
  uma aresta, o painel permite editar a relação e gerenciar seus arquivos.
- Importação de Google Contacts, Outlook, CSV e vCard, com exportação CSV/vCard.
- Ícone visual configurável, aplicado também ao favicon do navegador.
- Usuário e senha do administrador alteráveis nas configurações, com revogação
  das sessões existentes.
- Ícone privado do administrador personalizável, independente do ícone global do
  sistema e visível no painel da sessão.
- Parâmetros OSINT por pessoa, histórico de achados e arquivamento de PDFs.
- Senhas com Argon2 e sessões JWT revogáveis persistidas no SQLite.
- Interface React, TypeScript, Tailwind CSS, Lucide e Cytoscape.js.
- Migrações automáticas e imagem Docker executada como usuário sem privilégios.

## Início rápido

Requer Rust estável e Node.js 24.

```bash
git clone https://github.com/facrf/agendarx.git
cd agendarx
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

Após a publicação de uma versão pelo GitHub Actions, execute a imagem com:

```bash
docker pull ghcr.io/facrf/agendarx:0.3.0
docker run --rm -p 12000:12000 \
  -v agendarx-data:/app/data \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  -e ADMIN_LOGIN=admin \
  -e ADMIN_PASSWORD='uma-senha-forte' \
  ghcr.io/facrf/agendarx:0.3.0
```

O primeiro pacote GHCR de uma conta pessoal costuma nascer privado. Depois do
workflow, abra **Packages > agendarx > Package settings > Change visibility** no
GitHub e marque-o como **Public** para permitir pulls anônimos. Essa alteração é
permanente; se preferir mantê-lo privado, autentique o Docker/Portainer no GHCR.

Para gerar a imagem a partir do código local:

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
    image: ghcr.io/facrf/agendarx:0.3.0
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
      SEARXNG_URL: http://searxng:8080
    volumes:
      - agendarx_data:/app/data

  searxng:
    image: docker.io/searxng/searxng:latest
    restart: unless-stopped
    environment:
      SEARXNG_SECRET: troque-por-outro-segredo-com-32-ou-mais-caracteres
      SEARXNG_LIMITER: "false"
    configs:
      - source: searxng_settings
        target: /etc/searxng/settings.yml

volumes:
  agendarx_data:

configs:
  searxng_settings:
    content: |
      use_default_settings: true
      search:
        formats: [html, json]
      server:
        limiter: false
```

No Portainer, abra **Stacks > Add stack > Web editor**, cole o arquivo completo e
preencha as variáveis conforme [`portainer.env.example`](deploy/portainer.env.example).
Se o pacote GHCR for privado, registre primeiro as credenciais em **Registries**.

## Pesquisa pública

A Stack completa do Portainer já inclui uma instância SearXNG privada, acessível
somente pela rede interna do Docker, e configura `SEARXNG_URL` automaticamente.
Em outros modos de instalação, defina essa variável com a URL-base de uma instância
SearXNG que tenha a saída JSON habilitada. Nomes e identificadores são pesquisados
entre aspas; termos livres são enviados como configurados. Achados são deduplicados
pela URL.

Se a varredura informar `HTTP 403 Forbidden`, verifique primeiro o `settings.yml`
realmente carregado pelo contêiner. O SearXNG rejeita com 403 um formato que não
esteja em `search.formats`; o padrão costuma habilitar somente HTML. A configuração
deve conter `formats: [html, json]`. Se o limiter estiver ativo, os cabeçalhos do
proxy (`X-Forwarded-For` e `X-Real-IP`) também precisam estar corretos. Consulte o
[diagnóstico operacional](docs/IMPLANTACAO.md#erro-http-403-do-searxng).

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
| RISC-V 64 bits (`riscv64gc`) | Sim | Sim |

RISC-V é distribuído como pacote binário porque as imagens-base oficiais usadas no
Dockerfile. Todos os pacotes recebem checksum
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
