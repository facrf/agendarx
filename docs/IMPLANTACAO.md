# Implantação e operação

## Variáveis de ambiente

| Variável | Padrão | Descrição |
|---|---|---|
| `SERVER_ADDR` | `0.0.0.0:12000` | Endereço e porta HTTP |
| `DATABASE_URL` | `sqlite://data/agendarx.db?mode=rwc` | Banco SQLite |
| `FRONTEND_DIR` | `frontend/dist` | Diretório da SPA compilada |
| `JWT_SECRET` | somente desenvolvimento | Segredo com pelo menos 32 caracteres |
| `JWT_TTL_MINUTES` | `480` | Duração máxima da sessão |
| `COOKIE_SECURE` | `false` | Envia cookie apenas por HTTPS quando `true` |
| `ADMIN_LOGIN` | não definido | Login criado na primeira inicialização |
| `ADMIN_PASSWORD` | não definido | Senha do usuário inicial |
| `MAX_UPLOAD_BYTES` | `26214400` | Limite geral de upload, em bytes |
| `SEARXNG_URL` | não definido | URL-base opcional do SearXNG |
| `OSINT_TIMEOUT_SECONDS` | `20` | Timeout de cada requisição externa |
| `OSINT_MAX_RESULTS` | `15` | Resultados por parâmetro, de 1 a 100 |
| `OSINT_MAX_PDF_BYTES` | `20971520` | Limite de PDF, menor ou igual ao upload |
| `RUST_LOG` | `agendarx=info,tower_http=info` | Filtro de logs |

`ADMIN_LOGIN` e `ADMIN_PASSWORD` devem ser informados juntos. Eles criam o usuário
somente se o login ainda não existir; alterar a variável não redefine uma senha.

## Desenvolvimento local

```bash
cp .env.example .env
cd frontend
npm ci
npm run build
cd ..
cargo run
```

A aplicação fica em `http://localhost:12000`. Durante o desenvolvimento do React,
`npm run dev` continua usando `5173` e encaminha a API para `12000`.

## Docker

Imagem gerada pelo workflow de versões: `ghcr.io/facrf/agendarx:latest`. É preciso
publicar ao menos uma tag de versão antes do primeiro pull.

Na primeira versão, confirme no GitHub que o pacote `agendarx` foi marcado como
**Public**. O repositório público não torna automaticamente público um pacote GHCR
novo; enquanto ele estiver privado, faça login no registry antes do pull.

```bash
docker pull ghcr.io/facrf/agendarx:latest
docker run --rm -p 12000:12000 \
  -v agendarx-data:/app/data \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  -e ADMIN_LOGIN=admin \
  -e ADMIN_PASSWORD='uma-senha-forte' \
  ghcr.io/facrf/agendarx:latest
```

O contêiner executa sem privilégios e todo o estado fica em `/app/data`.

## Portainer

O arquivo [portainer-stack.yml](../deploy/portainer-stack.yml) pode ser colado no
editor Web de uma Stack. No Portainer:

1. registre o GHCR em **Registries** se a imagem for privada;
2. acesse **Stacks > Add stack > Web editor**;
3. cole o YAML de `deploy/portainer-stack.yml`;
4. cadastre as variáveis de `deploy/portainer.env.example` na seção da Stack;
5. mantenha `AGENDARX_IMAGE=ghcr.io/facrf/agendarx:latest` ou fixe uma versão;
6. implante e acesse `http://IP_DO_SERVIDOR:12000`.

As variáveis `JWT_SECRET` e `ADMIN_PASSWORD` são obrigatórias. A Stack aplica
filesystem raiz somente leitura, remove capabilities, impede elevação de privilégio
e mantém apenas o volume de dados gravável.

## Proxy reverso e HTTPS

O proxy deve encaminhar para `agendarx:12000` na mesma rede Docker ou para a porta
publicada no host. Preserve `Host`, `X-Forwarded-For` e `X-Forwarded-Proto`. Quando o
acesso externo for exclusivamente HTTPS, configure `COOKIE_SECURE=true`.

Não publique a aplicação diretamente na internet sem autenticação de borda,
controle de acesso e uma política de backups adequada ao conteúdo do dossiê.

## Backup e restauração

Para uma cópia consistente e simples, interrompa temporariamente a Stack e copie o
volume `agendarx_data`. Em uma instalação sem Docker, copie `data/agendarx.db` e os
arquivos `-wal`/`-shm` juntos, ou use uma ferramenta de backup online do SQLite.

Antes de atualizar:

1. faça backup do volume;
2. leia o `CHANGELOG.md` da versão;
3. altere a tag de `AGENDARX_IMAGE`;
4. solicite ao Portainer o pull e a recriação da Stack;
5. confira `/health`, login, anexos e logs.

As migrações são aplicadas automaticamente no primeiro início da nova versão.
