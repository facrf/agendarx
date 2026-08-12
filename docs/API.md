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
  "ativo": true
}
```

Os tipos aceitos são `NOME`, `CPF`, `CNPJ`, `EMAIL`, `TELEFONE` e `TERMO`.

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
nesse estado não comprova a ausência de achados.

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

As respostas de metadados dos anexos incluem `url_thumbnail` para imagens raster
suportadas e `null` para os demais formatos. A miniatura tem no máximo 512 px em
cada lado; anexos anteriores à implantação geram e armazenam esse cache no
primeiro acesso à URL.
