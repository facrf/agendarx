# API REST

A base da API é `/api`. Salvo `POST /api/auth/login`, `GET /api/identidade/icone`
e `GET /health`, todas as rotas exigem o cookie de sessão ou
`Authorization: Bearer <token>`.

Erros usam o formato:

```json
{ "erro": "mensagem descritiva" }
```

## Rotas

As sessões incluem `usuario.perfil` (`admin` ou `usuario`). Somente administradores
podem alterar `/api/configuracoes/*` e consultar `/api/configuracoes/admin/*`.
Cada conta pode alterar suas próprias credenciais e ícone em `/api/auth/*`.
Pessoas, dossiês e vínculos são compartilhados; tarefas são privadas por usuário.

`GET /api/configuracoes/admin/usuarios` lista contas sem senhas/hashes.
`POST /api/configuracoes/admin/usuarios` cria uma conta com
`{"login":"novo-login","senha":"senha-inicial","perfil":"usuario"}`.
Login aceita 3–64 caracteres e senha 8–1024. Perfil inválido retorna 400,
login duplicado 409 e acesso sem perfil administrativo 403.

`PUT /api/pessoas/{id}` aceita opcionalmente `contatos`, com `id` para contatos
existentes e `tipo_contato_id`/`valor`. A lista substitui os contatos da pessoa
na mesma transação dos dados básicos; omitir o campo preserva os contatos.
IDs pertencentes a outra pessoa e IDs repetidos são rejeitados.

Uploads respeitam `MAX_UPLOAD_BYTES` (padrão: 26.214.400 bytes / 25 MiB).
Os limites HTTP e dos extratores são alinhados, com 1 MiB adicional para o
envelope multipart. O limite individual continua validado em cada handler.
Um proxy reverso também precisa aceitar esse tamanho de requisição.
Fotos e anexos usam `Cache-Control: private, no-store` para evitar cópias antigas
e persistência de mídia privada no cache do navegador.

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
| Pessoas | `GET, POST /api/pessoas` | Listar/pesquisar (`?busca=`) e criar pessoas |
| Pessoas | `GET, PUT, DELETE /api/pessoas/{id}` | Consultar/editar e mover pessoa para a lixeira |
| Contatos | `GET, POST /api/pessoas/{pessoa_id}/contatos` | Listar/criar contatos |
| Contatos | `GET, PUT, DELETE /api/pessoas/contatos/{id}` | CRUD de contato |
| Dossiê | `GET, POST /api/dossie/pessoas/{id}/anexos` | Listar/upload multipart (`arquivo`) |
| Dossiê | `GET, PUT, DELETE /api/dossie/anexos/{id}` | Metadados/renomeação/exclusão |
| Dossiê | `GET /api/dossie/anexos/{id}/stream` | Conteúdo inline com HTTP Range |
| Dossiê | `GET /api/dossie/anexos/{id}/download` | Download com HTTP Range |
| Dossiê | `GET /api/dossie/anexos/{id}/thumbnail` | Miniatura WebP da imagem |
| Foto | `GET, PUT, POST, DELETE /api/dossie/pessoas/{id}/foto` | Foto principal; `POST` usa multipart `arquivo` |
| Vínculos | `GET, POST /api/vinculos` | Listar/criar vínculos |
| Vínculos | `GET, PUT, DELETE /api/vinculos/{id}` | CRUD de vínculo |
| Vínculos | `GET, POST /api/vinculos/{id}/anexos` | Listar/upload multipart (`arquivo`) |
| Vínculos | `GET, PUT, DELETE /api/vinculos/anexos/{id}` | Metadados/renomeação/exclusão de anexo |
| Vínculos | `GET /api/vinculos/anexos/{id}/stream` | Foto, áudio ou arquivo inline com HTTP Range |
| Vínculos | `GET /api/vinculos/anexos/{id}/download` | Download do anexo com HTTP Range |
| Vínculos | `GET /api/vinculos/anexos/{id}/thumbnail` | Miniatura WebP da imagem |
| Organização | `GET, POST /api/produtividade/etiquetas` | Listar/criar etiquetas |
| Organização | `PUT, DELETE /api/produtividade/etiquetas/{id}` | Editar/excluir etiqueta |
| Organização | `GET, PUT /api/produtividade/pessoas/{id}/etiquetas` | Etiquetas da pessoa |
| Organização | `PUT /api/produtividade/pessoas/{id}/favorito` | Favorito do usuário atual |
| Lixeira | `GET /api/produtividade/lixeira` | Listar pessoas excluídas |
| Lixeira | `POST /api/produtividade/lixeira/{id}/restaurar` | Restaurar pessoa e dados associados |
| Lixeira | `DELETE /api/produtividade/lixeira/{id}` | Exclusão definitiva (administrador) |
| Auditoria | `GET /api/produtividade/auditoria` | Últimas 500 operações (administrador) |
| Grafo | `GET, PUT /api/produtividade/grafo/posicoes/{layout}` | Posições por usuário e layout |
| Backup | `GET, POST /api/configuracoes/backups` | Listar/criar backups (administrador) |
| Backup | `GET, PUT /api/configuracoes/backups/configuracao` | Consultar/alterar horário e retenções automática, diária, semanal e mensal |
| Backup | `DELETE /api/configuracoes/backups/{id}` | Excluir uma cópia armazenada |
| Backup | `GET /api/configuracoes/backups/{id}/download` | Transmitir snapshot SQLite completo |
| Backup | `POST /api/configuracoes/exportacao-segura` | Preparar ZIP AES-256 e retornar token de download válido por 10 minutos |
| Backup | `GET /api/configuracoes/exportacoes/{token}/download` | Transmitir a exportação preparada diretamente para o navegador |
| Backup | `POST /api/configuracoes/restaurar` | Enviar e validar `.db` ou `.zip`; retorna token e prévia |
| Backup | `POST /api/configuracoes/restauracoes/{token}/confirmar` | Confirmar restore físico com `{"confirmacao":"RESTAURAR"}` |
| Backup | `DELETE /api/configuracoes/restauracoes/{token}` | Cancelar uma restauração preparada |
| Grafo | `GET /api/vinculos/grafo` | Nós e arestas para visualização |
| OSINT | `GET, POST /api/osint/parametros/{pessoa_id}` | Listar/criar parâmetros |
| OSINT | `PUT, DELETE /api/osint/parametros/item/{id}` | Atualizar/remover parâmetro |
| OSINT | `POST /api/osint/varrer/{pessoa_id}` | Executar busca e arquivamento |
| OSINT | `GET /api/osint/historico/{pessoa_id}` | Pesquisar e paginar a linha do tempo de achados |
| OSINT | `DELETE /api/osint/historico/item/{id}` | Remover um achado da linha do tempo; o PDF do dossiê é preservado |

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

O histórico aceita `busca`, `pagina` e `por_pagina`. A quantidade pode ser `10`,
`50`, `100` ou `0` para retornar todos os achados filtrados. `busca` procura em
título, trecho, fonte, provider, parâmetro, detalhes e URL. A resposta é paginada:

```json
{
  "itens": [],
  "total": 0,
  "pagina": 1,
  "por_pagina": 10,
  "total_paginas": 0
}
```

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

Pessoas aceitam `descricao` opcional no `POST` e no `PUT`; textos vazios são
normalizados para `null` e o limite é de 50.000 caracteres. O campo aceita Markdown e também é
devolvido nas listagens e no perfil detalhado.

### Notas de anexos e foto principal

Os endpoints abaixo aceitam `GET` para consultar e `PUT` para salvar
`{"notas":"Texto em Markdown"}` (até 50.000 caracteres; string vazia remove as notas):

- `/api/dossie/anexos/{id}/notas`
- `/api/vinculos/anexos/{id}/notas`
- `/api/calendario/anexos/{id}/notas`

As notas persistem no banco junto ao anexo. As notas de tarefas respeitam a
propriedade da tarefa. A migração `0013_notas_anexos.sql` adiciona os campos sem
alterar os arquivos existentes.

`POST /api/dossie/pessoas/{id}/foto` aceita multipart com o campo `arquivo`.
O `PUT` com bytes da imagem continua disponível. Ambos validam formato e
`MAX_UPLOAD_BYTES`. A interface otimiza a foto principal para até 1600 pixels,
mostra progresso e permite repetir um envio que falhou sem duplicar a pessoa.

### Exemplo do grafo

```json
{
  "nodes": [
    {
      "id": 1,
      "label": "Ana",
      "color": "#EF4444",
      "foto_url": "/api/dossie/pessoas/1/foto",
      "categoria": "Família",
      "descricao": "Descrição do perfil de Ana",
      "contatos": [
        { "tipo": "WhatsApp", "valor": "+55 11 99999-0000" }
      ]
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
