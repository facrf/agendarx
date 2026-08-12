# Arquitetura

## Visão geral

O AgendarX é uma aplicação monolítica leve. Um único processo Axum entrega a API
REST e os arquivos compilados do React. O SQLite permanece em disco local e também
armazena fotos e anexos como BLOBs.

```mermaid
flowchart LR
    U[Navegador] -->|HTTP :12000| A[Axum]
    A --> F[React estático]
    A --> M[Middleware de sessão]
    M --> H[Handlers REST]
    H --> S[(SQLite)]
    H --> X[SearXNG configurado]
    X --> P[Fontes públicas]
    P -->|PDF validado| H
    H -->|BLOB e histórico| S
```

## Componentes

| Componente | Responsabilidade |
|---|---|
| `src/main.rs` | Inicialização, composição das rotas e entrega da SPA |
| `src/config.rs` | Variáveis de ambiente e limites operacionais |
| `src/db/` | Conexão SQLite, migrações automáticas e usuário inicial |
| `src/models/` | Entidades persistidas e contratos JSON |
| `src/handlers/` | Autenticação, pessoas, configurações, dossiê, vínculos e OSINT |
| `src/middleware/` | Validação do JWT e da sessão revogável |
| `migrations/` | Evolução versionada do banco |
| `frontend/src/` | Rotas, páginas, componentes e serviço HTTP React |
| `deploy/` | Exemplos de implantação, incluindo Portainer |

## Persistência

As migrações são executadas automaticamente na inicialização. O SQLite usa chaves
estrangeiras e exclusões em cascata para os registros dependentes de uma pessoa.
Fotos e anexos ficam no banco para simplificar backup e portabilidade.

Em Docker, todo o estado persistente está sob `/app/data`. O volume precisa ser
mantido entre recriações do contêiner. Como os anexos são BLOBs, o tamanho do banco
cresce junto com o dossiê; monitore o volume e faça backups regulares.

## Autenticação

Senhas são derivadas com Argon2. No login, a aplicação cria um JWT e uma sessão no
SQLite. O cliente recebe um cookie `HttpOnly` com `SameSite=Strict`; clientes de API
também podem usar `Authorization: Bearer`. O logout remove a sessão persistida e
revoga o token antes da expiração.

## Arquivos e mídia

Uploads respeitam `MAX_UPLOAD_BYTES`. A API detecta o tipo do conteúdo, armazena o
BLOB e disponibiliza streaming inline ou download. O streaming aceita um intervalo
HTTP `Range`, permitindo reprodução de áudio e visualização de mídia pelo navegador.

## Pesquisa pública

O SearXNG é um serviço externo configurado pelo operador. A varredura:

1. lê até 50 parâmetros ativos da pessoa;
2. pesquisa cada valor e limita resultados por configuração;
3. deduplica achados pela URL de origem;
4. tenta baixar resultados identificados como PDF;
5. valida DNS, IP, tamanho e assinatura do arquivo;
6. persiste o histórico e o anexo em uma transação.

Downloads automáticos bloqueiam endereços locais, privados e reservados, fixam a
resolução DNS validada e não seguem redirecionamentos.

## Grafo de vínculos

`PessoaVinculo` representa arestas direcionadas entre pessoas. O endpoint de grafo
combina pessoas, categorias e vínculos em nós e arestas. O Cytoscape.js executa os
layouts e filtros no navegador; nenhuma posição visual é persistida.
