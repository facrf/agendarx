# AgendarX

Aplicação Web para agenda de contatos, anexos de dossiê e relações interpessoais.
O backend usa Rust, Axum, SQLite/SQLx, Argon2 e sessões JWT revogáveis. A interface
usa React, TypeScript, Tailwind CSS e Cytoscape.js. O módulo opcional de pesquisa
pública consulta uma instância SearXNG e arquiva PDFs encontrados no dossiê.

## Execução local

Requer a versão estável atual do Rust. Crie a configuração e inicie:

```bash
cp .env.example .env
# Altere JWT_SECRET e ADMIN_PASSWORD antes de iniciar.
cargo run
```

Na primeira inicialização, as migrações são aplicadas automaticamente. Quando
`ADMIN_LOGIN` e `ADMIN_PASSWORD` estão definidos, o usuário inicial é criado se
ainda não existir. Alterar essas variáveis depois não troca a senha de um usuário
existente.

Teste o serviço em `GET http://localhost:3000/health`.

### Frontend em desenvolvimento

Com a API Rust em `localhost:3000`, execute o Vite em outro terminal:

```bash
cd frontend
npm install
npm run dev
```

A interface ficará em `http://localhost:5173`. O proxy do Vite encaminha `/api`
e `/health` para o backend, mantendo o cookie de sessão na mesma origem. Para
usar outro endereço da API durante o desenvolvimento, defina
`VITE_API_PROXY_TARGET`.

Para gerar os arquivos que o próprio Axum entrega em produção:

```bash
cd frontend
npm run build
```

O diretório servido pode ser alterado por `FRONTEND_DIR`; o padrão local é
`frontend/dist`. Rotas da SPA têm fallback para `index.html`.

## Autenticação

Faça login e guarde o token retornado:

```bash
curl -i http://localhost:3000/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"login":"admin","senha":"troque-esta-senha"}'
```

O login retorna o JWT no JSON e também em um cookie `HttpOnly` com
`SameSite=Strict`. Clientes sem cookie podem enviar
`Authorization: Bearer <token>`. O logout remove a sessão persistida no SQLite,
revogando o token antes de sua expiração.

Para HTTPS, use `COOKIE_SECURE=true`. Em produção, `JWT_SECRET` deve ser um valor
aleatório com no mínimo 32 caracteres e o diretório `data/` deve estar em um
volume persistente.

## Rotas

Todas as rotas abaixo, exceto login e `/health`, exigem autenticação.

| Área | Método e rota | Uso |
|---|---|---|
| Auth | `POST /api/auth/login` | Iniciar sessão |
| Auth | `POST /api/auth/logout` | Revogar sessão |
| Auth | `GET /api/auth/sessao` | Verificar sessão |
| Configurações | `GET, POST /api/configuracoes/categorias` | Listar/criar categorias |
| Configurações | `GET, PUT, DELETE /api/configuracoes/categorias/{id}` | CRUD de categoria |
| Configurações | `GET, POST /api/configuracoes/tipos-contato` | Listar/criar tipos |
| Configurações | `GET, PUT, DELETE /api/configuracoes/tipos-contato/{id}` | CRUD de tipo |
| Pessoas | `GET, POST /api/pessoas` | Listar/criar pessoas |
| Pessoas | `GET, PUT, DELETE /api/pessoas/{id}` | CRUD de pessoa |
| Contatos | `GET, POST /api/pessoas/{pessoa_id}/contatos` | Listar/criar contatos |
| Contatos | `GET, PUT, DELETE /api/pessoas/contatos/{id}` | CRUD de contato |
| Dossiê | `GET, POST /api/dossie/pessoas/{id}/anexos` | Listar/upload multipart (`arquivo`) |
| Dossiê | `GET, DELETE /api/dossie/anexos/{id}` | Metadados/exclusão |
| Dossiê | `GET /api/dossie/anexos/{id}/stream` | Mídia inline com HTTP Range |
| Dossiê | `GET /api/dossie/anexos/{id}/download` | Download com HTTP Range |
| Foto | `GET, PUT, DELETE /api/dossie/pessoas/{id}/foto` | Foto principal em bytes brutos |
| Vínculos | `GET, POST /api/vinculos` | Listar/criar vínculos |
| Vínculos | `GET, PUT, DELETE /api/vinculos/{id}` | CRUD de vínculo |
| Grafo | `GET /api/vinculos/grafo` | Nós e arestas para visualização |
| OSINT | `GET, POST /api/osint/parametros/{pessoa_id}` | Listar/criar parâmetros |
| OSINT | `PUT, DELETE /api/osint/parametros/item/{id}` | Atualizar/remover parâmetro |
| OSINT | `POST /api/osint/varrer/{pessoa_id}` | Executar varredura e arquivamento |
| OSINT | `GET /api/osint/historico/{pessoa_id}` | Linha do tempo de achados |

O endpoint do grafo usa um contrato direto para Cytoscape/Vis/React Flow:

```json
{
  "nodes": [
    {
      "id": 1,
      "label": "Ana",
      "color": "#EF4444",
      "foto_url": "/api/dossie/pessoas/1/foto",
      "categoria": "Família"
    }
  ],
  "edges": [
    {
      "id": 1,
      "source": 1,
      "target": 2,
      "label": "Irmão",
      "descricao": "Histórico completo da relação"
    }
  ]
}
```

Na tela `/grafo`, os nós podem ser arrastados livremente. O visualizador alterna
entre teia de força e árvore hierárquica, abre os detalhes da relação ao clicar
em uma aresta e navega ao perfil com duplo clique. A busca foca uma pessoa e o
seletor de grau limita a vizinhança exibida.

No `PUT` da foto, envie os bytes da imagem e o `Content-Type: image/png` (ou
outro tipo `image/*`). Uploads e fotos respeitam `MAX_UPLOAD_BYTES`, cujo padrão é
25 MiB. Respostas de mídia aceitam um único intervalo no cabeçalho HTTP `Range`.

## Pesquisa pública e arquivamento

Defina `SEARXNG_URL` com a URL-base de uma instância SearXNG que tenha a saída
JSON habilitada. A aplicação acrescenta `/search` quando necessário. Uma
varredura pesquisa os parâmetros ativos (nome, CPF, CNPJ, e-mail, telefone ou
termo), registra resultados inéditos e tenta arquivar URLs que apontam
diretamente para `.pdf`.

`OSINT_MAX_RESULTS` limita resultados por parâmetro, `OSINT_TIMEOUT_SECONDS`
limita cada requisição e `OSINT_MAX_PDF_BYTES` limita o PDF. O limite de PDF não
pode superar `MAX_UPLOAD_BYTES`. Downloads automáticos não seguem
redirecionamentos, bloqueiam IPs locais/privados/reservados, fixam a resolução
DNS já validada e só persistem conteúdo com assinatura PDF. Falhas no PDF são
retornadas como avisos sem descartar o achado textual.

Use o módulo apenas para fontes de acesso público, com finalidade legítima e em
conformidade com a legislação de proteção de dados aplicável. O sistema não
contorna autenticação, paywalls, CAPTCHA ou controles de acesso das fontes.

## Docker

Gere a imagem e execute com volume persistente:

```bash
docker build -t agendarx .
docker run --rm -p 3000:3000 \
  -v agendarx-data:/app/data \
  -e JWT_SECRET='um-segredo-aleatorio-com-pelo-menos-32-caracteres' \
  -e ADMIN_LOGIN='admin' \
  -e ADMIN_PASSWORD='uma-senha-forte' \
  agendarx
```

O `Dockerfile` compila o React e o Rust em estágios separados, copia somente o
binário e os arquivos estáticos para a imagem final e executa a aplicação como
usuário sem privilégios. A interface e a API ficam disponíveis na porta `3000`.
