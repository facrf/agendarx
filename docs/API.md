# API REST

A base da API é `/api`. Salvo `POST /api/auth/login`, `GET /api/identidade/icone`
e `GET /health`, todas as rotas exigem o cookie de sessão ou
`Authorization: Bearer <token>`.

Erros usam o formato:

```json
{ "erro": "mensagem descritiva" }
```

## Rotas

| Área | Método e rota | Uso |
|---|---|---|
| Saúde | `GET /health` | Estado básico do processo |
| Auth | `POST /api/auth/login` | Iniciar sessão |
| Auth | `POST /api/auth/logout` | Revogar sessão atual |
| Auth | `GET /api/auth/sessao` | Verificar sessão |
| Auth | `PUT /api/auth/credenciais` | Alterar login/senha e revogar todas as sessões |
| Auth | `GET, PUT, DELETE /api/auth/icone` | Exibir, trocar ou restaurar o ícone privado do administrador |
| Calendário | `GET, POST /api/calendario/tarefas` | Listar por período ou agendar uma tarefa |
| Calendário | `GET, PUT, DELETE /api/calendario/tarefas/{id}` | Consultar, editar ou excluir uma tarefa |
| Calendário | `PATCH /api/calendario/tarefas/{id}/data` | Alterar somente início e término ao mover uma tarefa |
| Calendário | `PATCH /api/calendario/tarefas/{id}/status` | Concluir ou reabrir rapidamente uma tarefa |
| Calendário | `GET /api/calendario/tarefas/{id}/historico` | Consultar as 100 alterações mais recentes |
| Calendário | `GET /api/calendario/pessoas/{pessoa_id}/tarefas` | Listar tarefas vinculadas a uma pessoa |
| Calendário | `GET, POST /api/calendario/tarefas/{id}/anexos` | Listar/upload multipart (`arquivo`) dos anexos da tarefa |
| Calendário | `GET, DELETE /api/calendario/anexos/{id}` | Consultar metadados ou excluir anexo da tarefa |
| Calendário | `GET /api/calendario/anexos/{id}/stream` | Conteúdo inline com HTTP Range |
| Calendário | `GET /api/calendario/anexos/{id}/download` | Download com HTTP Range |
| Calendário | `GET /api/calendario/anexos/{id}/thumbnail` | Miniatura WebP da imagem |
| Calendário | `GET /api/calendario/lembretes` | Listar até 20 lembretes vencidos e ainda não dispensados |
| Calendário | `PATCH /api/calendario/lembretes/{id}/dispensar` | Marcar o lembrete da ocorrência como exibido |
| Calendário | `GET /api/calendario/armazenamento` | Uso e limites dos anexos de tarefas do usuário |
| Configurações | `GET, POST /api/configuracoes/categorias` | Listar/criar categorias |
| Configurações | `GET, PUT, DELETE /api/configuracoes/categorias/{id}` | CRUD de categoria |
| Configurações | `GET, POST /api/configuracoes/tipos-contato` | Listar/criar tipos |
| Configurações | `GET, PUT, DELETE /api/configuracoes/tipos-contato/{id}` | CRUD de tipo |
| Identidade | `GET /api/identidade/icone` | Ícone público usado pela interface e favicon |
| Identidade | `GET /api/configuracoes/identidade` | Estado da identidade visual |
| Identidade | `PUT, DELETE /api/configuracoes/icone` | Trocar/restaurar ícone em bytes brutos |
| Intercâmbio | `POST /api/configuracoes/contatos/importar` | Importar CSV ou vCard no campo multipart `arquivo` |
| Intercâmbio | `GET /api/configuracoes/contatos/exportar/{formato}` | Exportar toda a agenda em `csv` ou `vcf` |
| Pessoas | `GET, POST /api/pessoas` | Listar/criar pessoas |
| Pessoas | `GET, PUT, DELETE /api/pessoas/{id}` | CRUD de pessoa |
| Contatos | `GET, POST /api/pessoas/{pessoa_id}/contatos` | Listar/criar contatos |
| Contatos | `GET, PUT, DELETE /api/pessoas/contatos/{id}` | CRUD de contato |
| Dossiê | `GET, POST /api/dossie/pessoas/{id}/anexos` | Listar/upload multipart (`arquivo`) |
| Dossiê | `GET, PUT, DELETE /api/dossie/anexos/{id}` | Metadados/renomeação/exclusão |
| Dossiê | `GET /api/dossie/anexos/{id}/stream` | Conteúdo inline com HTTP Range |
| Dossiê | `GET /api/dossie/anexos/{id}/download` | Download com HTTP Range |
| Dossiê | `GET /api/dossie/anexos/{id}/thumbnail` | Miniatura WebP da imagem |
| Foto | `GET, PUT, DELETE /api/dossie/pessoas/{id}/foto` | Foto principal em bytes brutos |
| Vínculos | `GET, POST /api/vinculos` | Listar/criar vínculos |
| Vínculos | `GET, PUT, DELETE /api/vinculos/{id}` | CRUD de vínculo |
| Vínculos | `GET, POST /api/vinculos/{id}/anexos` | Listar/upload multipart (`arquivo`) |
| Vínculos | `GET, PUT, DELETE /api/vinculos/anexos/{id}` | Metadados/renomeação/exclusão de anexo |
| Vínculos | `GET /api/vinculos/anexos/{id}/stream` | Foto, áudio ou arquivo inline com HTTP Range |
| Vínculos | `GET /api/vinculos/anexos/{id}/download` | Download do anexo com HTTP Range |
| Vínculos | `GET /api/vinculos/anexos/{id}/thumbnail` | Miniatura WebP da imagem |
| Grafo | `GET /api/vinculos/grafo` | Nós e arestas para visualização |
| OSINT | `GET, POST /api/osint/parametros/{pessoa_id}` | Listar/criar parâmetros |
| OSINT | `PUT, DELETE /api/osint/parametros/item/{id}` | Atualizar/remover parâmetro |
| OSINT | `POST /api/osint/varrer/{pessoa_id}` | Executar busca e arquivamento |
| OSINT | `GET /api/osint/historico/{pessoa_id}` | Linha do tempo de achados |

## Exemplos

### Login

```bash
curl -i http://localhost:12000/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"login":"admin","senha":"sua-senha"}'
```

### Parâmetro de pesquisa

```json
{
  "tipo": "NOME",
  "valor": "Maria da Silva",
  "provider": "QUERIDO_DIARIO",
  "ativo": true
}
```

Os tipos aceitos são `NOME`, `CPF`, `CNPJ`, `EMAIL`, `TELEFONE` e `TERMO`.
Os providers são `SEARXNG`, `QUERIDO_DIARIO`, `INLABS` e `OPENALEX`. A omissão de
`provider` usa `SEARXNG`, preservando clientes e registros anteriores. O mesmo
campo é aceito no `PUT` e devolvido na listagem; assim a edição conserva a fonte.

A varredura pesquisa nomes com e sem correspondência exata e normaliza CPF, CNPJ
e telefone nas formas formatada e somente com dígitos. A resposta informa se a
execução foi `concluida`, `parcial` ou `inconclusiva`:

```json
{
  "situacao": "parcial",
  "parametros_processados": 2,
  "parametros_inconclusivos": 0,
  "resultados_encontrados": 15,
  "novos_achados": 4,
  "pdfs_arquivados": 1,
  "fontes_indisponiveis": 2,
  "avisos": [
    "NOME: fontes temporariamente indisponíveis: duckduckgo (tempo esgotado)"
  ]
}
```

Uma resposta `inconclusiva` significa que as fontes falharam; zero resultados
nesse estado não comprova a ausência de achados. O histórico inclui `provider`,
`fonte`, `data_publicacao` e `detalhes`; estes dois últimos podem ser nulos.

### Credenciais do administrador

```json
{
  "login": "novo-admin",
  "senha_atual": "senha-em-uso",
  "nova_senha": "nova-senha-forte"
}
```

`nova_senha` pode ser omitida quando somente o login for alterado. A senha atual
é sempre obrigatória. Após uma alteração válida, todas as sessões do usuário são
revogadas e o cliente deve autenticar novamente.

### Tarefa do calendário

```json
{
  "titulo": "Retornar ligação",
  "descricao": "Confirmar os detalhes da reunião",
  "inicio_em": "2026-08-12T18:00:00.000Z",
  "fim_em": "2026-08-12T18:30:00.000Z",
  "dia_inteiro": false,
  "status": "PENDENTE",
  "prioridade": "ALTA",
  "cor_hex": "#13716D",
  "pessoas_ids": [1, 2],
  "recorrencia": "SEMANAL",
  "recorrencia_fim_em": "2026-10-31T23:59:59.999Z",
  "lembrete_minutos": 30
}
```

Datas com horário usam ISO 8601 e são normalizadas para UTC. Os status aceitos são
`PENDENTE`, `EM_ANDAMENTO` e `CONCLUIDA`; prioridades podem ser `BAIXA`, `NORMAL`
ou `ALTA`. A listagem aceita os filtros opcionais `inicio` e `fim`, também em ISO
8601. Cada tarefa pertence ao usuário autenticado e pode vincular até 50 pessoas.
A resposta inclui os metadados de `anexos`; cada tarefa aceita até 30 arquivos. Para
movê-la sem reenviar os demais campos, use `PATCH /tarefas/{id}/data` com
`inicio_em` e `fim_em`, preservando a duração no cliente.

`recorrencia` aceita `NENHUMA`, `DIARIA`, `SEMANAL` ou `MENSAL`. Uma série pode
abranger no máximo 366 dias e é materializada como ocorrências independentes;
editar, mover, concluir ou excluir uma delas não altera as demais. A recorrência
mensal preserva o dia-base quando possível (dia 31 usa o último dia nos meses mais
curtos e volta ao dia 31 no seguinte). Os anexos enviados durante a criação ficam
na primeira ocorrência. `lembrete_minutos` aceita `null` ou de 0 a 525600 minutos
antes do início.

O corpo da alteração rápida de status é `{"status":"CONCLUIDA"}` (também aceita
`PENDENTE` e `EM_ANDAMENTO`). Movimentações, mudanças de status, edições e inclusão
ou exclusão de anexos alimentam o histórico da ocorrência.

O endpoint de armazenamento retorna os bytes usados, a quantidade de anexos e os
limites por arquivo, tarefa e usuário. Além do limite individual de
`MAX_UPLOAD_BYTES`, o servidor aplica `TASK_STORAGE_PER_TASK_BYTES` e
`TASK_STORAGE_QUOTA_BYTES` em cada upload.

### Grafo

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

No `PUT` da foto ou do ícone, envie os bytes de uma imagem reconhecida. Uploads
multipart devem usar o campo `arquivo`. A importação reconhece vCard (`.vcf`), CSV
do Google Contacts, CSV do Outlook e CSV genérico com cabeçalho de nome. Para
renomear um anexo do dossiê ou vínculo, o corpo é `{"nome_arquivo":"novo.pdf"}`.
Os anexos de tarefas podem ser visualizados, baixados e excluídos, mas não renomeados.

As respostas de metadados dos anexos incluem `url_thumbnail` para imagens raster
suportadas e `null` para os demais formatos. A miniatura tem no máximo 512 px em
cada lado; anexos anteriores à implantação geram e armazenam esse cache no
primeiro acesso à URL.
