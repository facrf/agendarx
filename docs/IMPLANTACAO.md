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
| `TASK_STORAGE_PER_TASK_BYTES` | `104857600` | Cota total de anexos por tarefa, em bytes; não pode ser menor que o limite de upload |
| `TASK_STORAGE_QUOTA_BYTES` | `1073741824` | Cota total dos anexos de tarefas por usuário, em bytes; não pode ser menor que a cota por tarefa |
| `SEARXNG_URL` | não definido | URL-base do SearXNG; a Stack usa o serviço interno por padrão |
| `SEARXNG_IMAGE` | `docker.io/searxng/searxng:latest` | Imagem do SearXNG incluído na Stack |
| `SEARXNG_SECRET` | não definido | Segredo obrigatório do SearXNG incluído na Stack |
| `OPENALEX_API_KEY` | não definido | Chave opcional do OpenAlex, enviada somente no cabeçalho de autenticação |
| `INLABS_USERNAME` | não definido | Usuário do INLABS; obrigatório junto da senha para usar a fonte DOU |
| `INLABS_PASSWORD` | não definido | Senha do INLABS; obrigatório junto do usuário e nunca incluída em logs |
| `INLABS_LOOKBACK_DAYS` | `1` | Janela recente do DOU por varredura, de 1 a 7 dias |
| `OSINT_TIMEOUT_SECONDS` | `20` | Timeout de cada requisição externa |
| `OSINT_MAX_RESULTS` | `15` | Resultados por parâmetro, de 1 a 100 |
| `OSINT_MAX_PDF_BYTES` | `20971520` | Limite de PDF, menor ou igual ao upload |
| `RUST_LOG` | `agendarx=info,tower_http=info` | Filtro de logs |

`ADMIN_LOGIN` e `ADMIN_PASSWORD` devem ser informados juntos. Eles criam o usuário
somente quando a tabela de usuários ainda está vazia; depois disso, login e senha
são alterados em **Configurações > Administrador**. Alterar as variáveis não redefine
credenciais existentes nem recria o login antigo após uma troca pela interface.

`INLABS_USERNAME` e `INLABS_PASSWORD` devem ser configurados juntos. Sem ambos, a
fonte INLABS mostra um aviso amigável e as pesquisas das outras fontes continuam.
Os ZIPs diários são baixados uma vez por varredura e reutilizados para todos os
parâmetros INLABS daquela execução.

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

Imagem gerada pelo workflow de versões: `ghcr.io/facrf/agendarx:0.4.0`. É preciso
publicar ao menos uma tag de versão antes do primeiro pull.

Na primeira versão, confirme no GitHub que o pacote `agendarx` foi marcado como
**Public**. O repositório público não torna automaticamente público um pacote GHCR
novo; enquanto ele estiver privado, faça login no registry antes do pull.

```bash
docker pull ghcr.io/facrf/agendarx:0.4.0
docker run --rm -p 12000:12000 \
  -v agendarx-data:/app/data \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  -e ADMIN_LOGIN=admin \
  -e ADMIN_PASSWORD='uma-senha-forte' \
  ghcr.io/facrf/agendarx:0.4.0
```

O contêiner executa sem privilégios e todo o estado fica em `/app/data`.

## Portainer

O arquivo [portainer-stack.yml](../deploy/portainer-stack.yml) pode ser colado no
editor Web de uma Stack. No Portainer:

1. registre o GHCR em **Registries** se a imagem for privada;
2. acesse **Stacks > Add stack > Web editor**;
3. cole o YAML de `deploy/portainer-stack.yml`;
4. cadastre as variáveis de `deploy/portainer.env.example` na seção da Stack,
   incluindo um `SEARXNG_SECRET` independente;
5. mantenha `AGENDARX_IMAGE=ghcr.io/facrf/agendarx:0.4.0` e atualize a versão
   conscientemente;
6. implante e acesse `http://IP_DO_SERVIDOR:12000`.

As variáveis `JWT_SECRET`, `ADMIN_PASSWORD` e `SEARXNG_SECRET` são obrigatórias. A
Stack sobe um SearXNG privado, com JSON habilitado e sem publicar sua porta. Para
usar uma instância externa, preencha `SEARXNG_URL`; se a variável ficar vazia, a
aplicação usa `http://searxng:8080`.

O contêiner principal aplica filesystem raiz somente leitura, remove capabilities,
impede elevação de privilégio e mantém apenas o volume de dados gravável.

## Erro HTTP 403 do SearXNG

A API de pesquisa usa `GET /search?format=json`. De acordo com a
[documentação da API do SearXNG](https://docs.searxng.org/dev/search_api.html),
solicitar um formato ausente da configuração resulta em `403 Forbidden`. Confirme
o arquivo efetivo (ajuste o nome do contêiner se necessário):

```bash
docker compose -f deploy/portainer-stack.yml exec searxng \
  sed -n '1,160p' /etc/searxng/settings.yml
```

Ele precisa conter, no mínimo:

```yaml
use_default_settings: true
search:
  formats:
    - html
    - json
server:
  limiter: false
```

Teste a API diretamente pela rede da Stack:

```bash
docker compose -f deploy/portainer-stack.yml exec searxng \
  wget -qO- --proxy=off --header='X-Forwarded-For: 127.0.0.1' \
  'http://127.0.0.1:8080/search?q=teste&format=json'
```

Se HTML funcionar e JSON retornar 403, o problema é `search.formats`, não o valor
pesquisado (`NOME`, CPF etc.). Atualize o YAML da Stack com a versão deste repositório
e recrie o serviço `searxng`. Em instalações antigas, remova o mapeamento de volume
para `/etc/searxng` que esteja preservando um `settings.yml` gerado anteriormente;
o arquivo deve vir do bloco `configs.searxng_settings` da Stack.

Caso JSON já esteja habilitado e os logs mostrem ausência de `X-Forwarded-For` ou
`X-Real-IP`, desative o limiter na instância privada como no exemplo acima ou
configure esses cabeçalhos no proxy conforme a
[documentação do limiter](https://docs.searxng.org/admin/searx.limiter.html).

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
