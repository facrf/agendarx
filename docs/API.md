# API REST

A base da API é `/api`. Salvo `POST /api/auth/login` e `GET /health`, todas as
rotas exigem o cookie de sessão ou `Authorization: Bearer <token>`.

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
| Dossiê | `GET /api/dossie/anexos/{id}/stream` | Conteúdo inline com HTTP Range |
| Dossiê | `GET /api/dossie/anexos/{id}/download` | Download com HTTP Range |
| Foto | `GET, PUT, DELETE /api/dossie/pessoas/{id}/foto` | Foto principal em bytes brutos |
| Vínculos | `GET, POST /api/vinculos` | Listar/criar vínculos |
| Vínculos | `GET, PUT, DELETE /api/vinculos/{id}` | CRUD de vínculo |
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

No `PUT` da foto, envie bytes de uma imagem reconhecida e um `Content-Type`
`image/*`. Uploads multipart devem usar o campo `arquivo`.
