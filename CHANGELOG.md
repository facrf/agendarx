# Histórico de alterações

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o
versionamento dos pacotes segue SemVer; tags com sufixo alfabético usam a
grafia de publicação solicitada (por exemplo, `v0.6.5a` = pacote `0.6.5-a`).

## [0.7.3] - 2026-10-26

### Adicionado

- **Editor de documentos de texto estilo Word no Dossiê de Pessoas**:
  - Novo componente de edição de documentos (`DocumentEditorModal`) com layout de folha de papel A4 centralizada, sombra realista e tipografia nítida estilo Microsoft Word / Google Docs.
  - Suporte a arquivos de texto nos formatos **HTML (`.html`)** e **Markdown (`.md`)**, com alternador rápido de formato e conversor bidirecional inteligente (`documentConverter.ts`).
  - Barra de ferramentas rica de formatação (Ribbon):
    - Parágrafos e títulos hierárquicos (H1, H2, H3).
    - Formatação inline: Negrito (`Ctrl+B`), Itálico (`Ctrl+I`), Sublinhado (`Ctrl+U`), Tachado.
    - Alinhamento de parágrafo: Esquerda, Centro, Direita e Justificado.
    - Listas com marcadores e numeradas.
    - Inserção de tabelas com linhas e colunas formatadas, citações em bloco, blocos de código pré-formatados, linhas horizontais divisórias e links.
    - Desfazer (`Ctrl+Z`), Refazer (`Ctrl+Y`) e limpeza rápida de formatação.
  - Modos de exibição alternáveis: **Visual** (WYSIWYG Word), **Código** (edição direta do fonte HTML/Markdown com atalhos), **Dividido** (Split: editor Markdown à esquerda e prévia ao vivo na folha à direita) e **Prévia** (leitura final da folha).
  - Rodapé com estatísticas em tempo real (contagem de palavras e caracteres), indicador de status de gravação ("Salvo" / "Alterações não salvas"), dicas de atalhos e atalho global `Ctrl+S` para salvar.
  - Acesso direto pelo painel Dossiê através do botão **"Escrever documento"**, botão de edição na lista de anexos e botão **"Editar documento"** no modal de pré-visualização de anexo.
  - Exclusão com confirmação tanto no editor quanto na lista do dossiê.

- **Novos endpoints de documentos de texto no backend (`src/handlers/dossie.rs`)**:
  - `POST /api/dossie/pessoas/{pessoa_id}/documentos`: criação de documentos de texto estruturados em JSON, com normalização de nome e extensão (.md/.html), definição automática de `mime_type` e validação do limite de upload.
  - `GET /api/dossie/anexos/{id}/documento`: obtenção do conteúdo textual e metadados do documento em UTF-8.
  - `PUT /api/dossie/anexos/{id}/documento`: alteração do conteúdo, nome e formato do documento existente.
  - Indexação instantânea no motor de busca textual FTS5 do SQLite via triggers automáticos.
  - Cobertura por testes automatizados de integração em `src/integration_tests.rs`.

## [0.7.2] - 2026-10-25

### Segurança

- **Remediação de vulnerabilidades críticas de outubro/2026 no ecossistema Rust**:
  - `zip`: atualização para `9.1.0` eliminando as vulnerabilidades RUSTSEC-2026-0369 (CVSS 9.1 Crítico - heap buffer overflow), RUSTSEC-2026-0368 e RUSTSEC-2026-0367 na restauração e inspeção de backups compactados.
  - `quick-xml`: atualização para `0.42.0` corrigindo vulnerabilidades de negação de serviço e panics RUSTSEC-2026-0195 e RUSTSEC-2026-0194 no parser de feeds OSINT.
  - Dependências de rede e runtime: atualização do lockfile (`cargo update`) mitigando 17 advisories de segurança em `rustls` (0.23.46), `rustls-webpki` (0.103.16), `webpki-root-certs` (1.0.11), `tokio` (1.54.1), `quinn-proto` (0.11.19) e `h2` (0.4.20).
- **Remediação de vulnerabilidades de outubro/2026 no frontend (npm)**:
  - Resolução de todas as vulnerabilidades apontadas pelo `npm audit` (`0 vulnerabilities`), atualizando `vite` para `8.3.5`, `zod` para `4.6.7`, `brace-expansion` para `5.0.12` e `source-map-js` para `1.2.2`.

### Corrigido

- **Sincronização estrita de versão entre Frontend e Backend**:
  - Resolução de inconsistência onde navegadores retinham e exibiam versões legadas do frontend (ex: `v0.6.9`) decorrente de cache persistente do Service Worker e permissões de build.
  - O Service Worker (`sw.js`) agora adota nomenclatura versionada de cache (`agendarx-shell-v{versao}`) e efetua purga ativa de caches obsoletos no evento `activate`.
  - Injeção dinâmica da versão no Service Worker em tempo de compilação via plugin Vite (`swVersionPlugin`).
  - O componente `AppFooter` agora consulta a versão real do software em tempo de execução via endpoint `/api/identidade/versao` (com fallback imediato para o `package.json`), eliminando disparidades entre o frontend e a API.
  - Enriquecimento das respostas de `/health`, `/api/versao` e `/api/auth/sessao` com o campo `versao`.
  - Adição de testes automatizados de integração validando paridade exata de versão entre `Cargo.toml` e `frontend/package.json` e o retorno do endpoint de versão.

### Modificado

- **Regras operacionais e espelhamento Gitea/GitHub (`AGENTS.md`)**:
  - Inclusão das diretrizes mandatórias de paridade absoluta de versão entre frontend e backend.
  - Documentação formal do espelhamento do repositório Git local no Gitea (`http://192.168.0.10:3010/facrf/agendarx.git`) sincronizado com o GitHub (`https://github.com/facrf/agendarx.git`).
- **Otimização de `.gitignore` e `.dockerignore`**:
  - Inclusão de artefatos de testes (`playwright-report`, `test-results`), logs de gerenciadores (`npm-debug.log*`, etc.) e segredos de deploy (`deploy/.env`).
  - Exclusão do contexto do Docker de diretórios desnecessários (`docs/`, `tests/` e arquivos markdown), agilizando o envio de contexto e construção de imagens OCI.

## [0.7.1] - 2026-10-24

### Segurança

- **Cabeçalhos restritivos CSP e Permissions-Policy**: inclusão de `Content-Security-Policy: default-src 'self' ...` e `Permissions-Policy: camera=(), microphone=(), geolocation=()` no middleware global, mitigando injeção de scripts (XSS), framing e acesso indevido a APIs de dispositivos.
- **Auditoria de tentativas de autenticação (`auth/login`)**: registro persistente de logins bem-sucedidos (`200`), falhas por credenciais inválidas (`401`) e bloqueios por excesso de tentativas (`429`) na tabela `auditoria`, permitindo monitoramento de acessos no painel administrativo em redes locais e VPNs.
- **Auditoria de dependências automatizada no CI**: inclusão de verificação de vulnerabilidades `npm audit --audit-level=high` no fluxo de qualidade do GitHub Actions.
- **Documentação de segurança e modelo de rede privada**: atualização de `docs/SEGURANCA.md` e `docs/API.md` formalizando a operação defensiva em redes locais (LAN) e VPNs.

## [0.7.0] - 2026-10-23

### Adicionado

- **Progressive Web App (PWA)**: manifesto web (`manifest.webmanifest`), meta tags mobile/PWA para iOS/Android, e Service Worker (`sw.js`) com cache shell (Stale-While-Revalidate / Cache-First para `/assets/`) e suporte a navegação resiliente offline, com cabeçalhos `no-cache` no servidor para atualização imediata.
- **Exportação do Grafo em GraphML (`/api/vinculos/grafo/graphml`)**: exportação completa da rede de relacionamentos e dos indicadores psicossociais (HP, HP percentual, toxicidade, classificação de risco, aura e vitalidade) em formato padronizado GraphML XML para análise avançada no Gephi, Cytoscape Desktop, yEd e NetworkX.
- **Exportação do Grafo em GEXF (`/api/vinculos/grafo/gexf`)**: exportação no formato nativo do Gephi (GEXF 1.2) com atributos de visualização (`viz:color`), indicadores e metadados completos.
- **Ações de exportação no mapa de relacionamentos**: botões de download direto para GraphML e GEXF integrados à barra de ferramentas em `GraphPage.tsx`.
- **Notificações PWA com Service Worker**: lembretes de tarefas integrados ao `registration.showNotification` e evento `notificationclick` no Service Worker para abertura direta da tarefa ao clicar.
- **Gerenciador de Instalação PWA (`PwaInstallManager`)**: componente nas Configurações para instalação do aplicativo independente e verificação de status.

### Otimizações

- **Modo WAL e `synchronous=NORMAL` no SQLite**: habilitação de Write-Ahead Logging (`journal_mode=WAL`) e `synchronous=NORMAL` para bancos em arquivo em `src/db/mod.rs`, permitindo leituras e escritas concorrentes sem bloqueios e reduzindo fsyncs no disco.
- **Eliminação de N+1 queries no calendário (`calendario.rs`)**: carregamento em lote de pessoas vinculadas, anexos e contagem de séries para listagens mensais e lembretes, reduzindo de centenas para apenas 3 a 4 consultas SQL por requisição.
- **Otimização da contagem paginada de pessoas (`pessoas.rs`)**: separação dos predicados da cláusula de contagem (`COUNT(*)` e fotos), eliminando subconsultas custosas de etiquetas e favoritos na paginação.
- **Cálculo localizado de HP psicossocial para perfil único (`hp_psicossocial.rs` / `pessoas.rs`)**: introdução de `calcular_hp_pessoa`, restringindo consultas e processamento estritamente ao subgrafo relevante (até 2º grau) da pessoa consultada, eliminando varreduras do banco inteiro em visualizações de perfil.
- **Cache e Throttling no frontend**: adição de cache client-side para tipos de contato e contatos no calendário, além de controle de frequência (throttling de 30s) na recarga por foco e visibilidade no grafo e no perfil de contato.
- **Cabeçalhos de cache imutável para assets estáticos (`main.rs`)**: injeção de `Cache-Control: public, max-age=31536000, immutable` para bundles de assets gerados pelo Vite e `no-cache` para o documento HTML principal.
- **Otimização de cache de camadas no Dockerfile**: preservação da camada de build de dependências do Rust quando apenas migrações são alteradas.

### Segurança

- **Sanitização CRLF no vCard (`exportar_vcard`)**: purga caracteres `\r` (retorno de carro) em `escapar_vcard`, mitigando injeção de propriedades e delimitações falsas em clientes de contatos.
- **Limites de comprimento em campos de texto (CWE-400)**: validação de tamanho máximo em `nome` da pessoa (255 caracteres), `valor` de contato (500 caracteres), `tipo_vinculo` (100 caracteres), `nome_categoria` (100 caracteres) e `nome_tipo` de contato (100 caracteres), prevenindo ataques de negação de serviço e sobrecarga de banco de dados.
- **Cabeçalhos globais de segurança HTTP**: middleware global com `X-Frame-Options: SAMEORIGIN`, `X-Content-Type-Options: nosniff` e `Referrer-Policy: strict-origin-when-cross-origin`, mitigando Clickjacking/UI Redressing e MIME-sniffing.
- **Configuração de produção reforçada**: documentação e exemplos atualizados com `COOKIE_SECURE=true` para proteção de cookies de sessão em tráfego HTTPS/TLS.
- **Documentação de segurança**: relatório técnico de auditoria publicado em `docs/SEGURANCA.md`.

## [0.6.9] - 2026-10-15

### Modificado

- **Política de backup automático desativada por padrão (opt-in explícito)**: migração `0025` adiciona `politica_confirmada`, mantendo a rotina automática desativada por padrão até decisão explícita do administrador.
- **Exportação protegida sem cópias locais extras**: o download do backup completo (`.zip` criptografado) utiliza snapshot temporário, sem salvar nem acumular arquivos `.db` permanentes no servidor.
- **Prevenção de disparos duplicados após restauração**: a configuração da política de backup do administrador é preservada após restaurar snapshots antigos e `ultima_tentativa_em` é atualizada preventivamente antes da gravação do snapshot.
- **Retenção de backups de segurança**: backups automáticos criados antes de restaurações (`seguranca`) agora respeitam retenção máxima (5 cópias mais recentes), evitando acúmulo infinito em disco.
- **Melhorias de UX em Backup e Restauração**: painel com status claro da política (manual vs automática ativa), campos desabilitados visualmente quando inativos, indicador de alterações pendentes de confirmação e resumo de armazenamento em disco com opção de exibir todos os backups.

## [0.6.8] - 2026-10-15

### Adicionado

- Criação contextual de vínculos com formulário reutilizável e atalhos diretos a partir de cartões em Pessoas, perfil e nó em foco no grafo.
- Menus de contexto no mapa de vínculos (grafo) com ações rápidas para pessoas (criar vínculo, editar cadastro, abrir perfil e focar conexões) e vínculos (editar vínculo, ver detalhes e lixeira).
- Rascunhos persistidos por pessoa de origem para edição e criação de vínculos.
- Acessibilidade completa nos menus de contexto com navegação por teclado (setas, Home, End, Tab, Escape) e suporte a toque em telas móveis.

### Alterado

- Pesquisa de pessoas (`/api/pessoas` e `/api/pessoas/paginadas`) restrita aos dados cadastrais e ao dossiê da própria pessoa, isolando tarefas e vínculos na busca global (`/api/busca`).

## [0.6.7] - 2026-10-11

### Corrigido

- Retentativas de cadastro após alteração de foto mantêm a versão atual sem ocultar
  edições concorrentes; rascunhos recuperados preservam o ID do cadastro parcial e
  continuam apagados após uma gravação concluída.
- Leituras com `Range` em anexos de dossiê, vínculos e tarefas consultam somente o
  trecho solicitado e mantêm as verificações de propriedade e vínculo ativo.
- O bootstrap rejeita credenciais administrativas fora dos limites aceitos pelo login.

### Segurança

- `JWT_SECRET` passa a ser obrigatório. `ALLOW_INSECURE_DEV_JWT=true` habilita o
  segredo público apenas para desenvolvimento local.

## [0.6.6] - 2026-10-07

### Adicionado

- Histórico e restauração de versões de pessoas, tarefas e vínculos, com controle
  de edição concorrente por versão e preservação dos rascunhos após conflito.
- Mesclagem de contatos com prévia, escolha de campos e arquivo de notas/fotos/vínculos.
- Horários silenciosos por conta e adiamento persistente de lembretes.
- Painel de saúde com armazenamento, falhas de pesquisa e verificação de backup.
- Prévia de importação com coincidências por email/telefone, ações por registro,
  confirmação transacional e preservação de dados ao atualizar contatos.
- Trabalhos persistentes de pesquisa pública com progresso, cancelamento e retomada.
- Workflow de qualidade compartilhado por CI e publicação, incluindo Chromium.
- Plano e acompanhamento em `A_FAZER.md`.

### Corrigido

- Consulta SQL de exportação CSV/vCard com JOINs antes do filtro de lixeira.
- Limites de tentativas, concorrência e corpo do login; emissão de sessões protegida
  contra alteração concorrente de credenciais.
- Uploads interrompidos no teto por arquivo; limite de anexos validado na transação.
- Respostas antigas do calendário descartadas e lembretes com reconhecimento de versão.
- Foco e navegação por teclado em modais, inclusive sobrepostos.

### Alterado

- Importação retorna prévia, exigindo confirmação; varredura retorna 202 com trabalho
  consultável. Clientes de API devem seguir os contratos em `docs/API.md`.

## [0.6.5e] - 2026-10-06

### Adicionado

- Painel inicial Seu dia com tarefas privadas de hoje, atrasadas e próximos sete dias.
- Busca global por pessoas, tarefas, vínculos e arquivos, com trechos e links diretos.
- Índice FTS5 atualizado por triggers, incluindo contatos, etiquetas, notas e arquivos de texto.
- Paginação e filtros de pessoas no servidor, com cancelamento de buscas anteriores.
- Rascunhos locais por conta e formulário, recuperação explícita e expiração em 30 dias.
- Salvamento e recuperação manual de posições do grafo por conta e layout.
- Auditoria paginada com filtros, método, duração e detalhes sem conteúdo de requisições.
- Guia de uso em `docs/PRODUTIVIDADE.md` e documentação dos novos endpoints.

### Alterado

- Grafo com canvas ocupando sua área, zoom com margens menores, controles de aproximação
  e gerenciamento lateral recolhível; títulos e barras entram no enquadramento.
- Agrupamento de categorias dimensionado por grupo e formato da área disponível.
- Carregamento do grafo reaproveita as arestas para listar vínculos e adia consultas
  de pessoas/lixeira até abrir o gerenciamento; eventos de atualização são agrupados.
- GETs simultâneos compartilham respostas; caches curtos de seletores são invalidados
  em escritas, uploads e encerramento de sessão.
- Migrações 20 e 21 acrescentam busca indexada e detalhes de auditoria; validação de
  backups reconhece o esquema atualizado.

### Corrigido

- Zoom manual é preservado durante redimensionamento e limpar foco reenquadra o mapa.
- Destaque de conexões respeita a profundidade selecionada.
- Margens de enquadramento não alteram os limites internos do Cytoscape ao focar
  repetidamente uma pessoa sem vínculos.
- Busca de pessoas não expõe conteúdo de tarefas e anexos privados de outra conta.

## [0.6.5d] - 2026-10-03

### Alterado

- Um clique em uma pessoa no grafo centraliza o nó e ajusta o zoom para exibir
  suas conexões até o segundo grau, ativando apenas conexões e tela cheia.
- Abrir no mapa pelo perfil usa o ID da pessoa, entra em tela cheia e isola seus
  relacionamentos; filtros salvos de categoria, vínculo e período são limpos
  nessa abertura para não ocultar a pessoa ou suas conexões.
- O foco permanece centralizado ao redimensionar a janela; o enquadramento
  reserva espaço para nomes, auras, títulos de categorias e barras de vitalidade.
- Um novo clique na mesma pessoa recentraliza o mapa após ajustes manuais.
- A imagem GHCR reutiliza os pacotes compilados por `cross`, com verificação de
  checksums, evitando recompilar Rust por emulação ARM. Um workflow manual permite
  publicar a imagem e a Release a partir de artefatos já validados. O runtime usa
  Debian Trixie e a publicação testa `/health` nas três arquiteturas.

### Corrigido

- A imagem de pacotes ARMv7 ajusta o limite de backup para no máximo 4 GiB menos
  1 byte, evitando falha de inicialização com o padrão de 5 GiB em 32 bits.
- Pessoas com nomes iguais são selecionadas pelo ID no clique e em Abrir no mapa.
- Pessoas sem vínculos recebem um enquadramento com aproximação limitada.
- Testes de navegador validam a centralização, a abertura automática em tela
  cheia, a saída por Esc e a seleção de vínculos pelo ID após isolar conexões.

## [0.6.5c] - 2026-09-23

### Adicionado

- Lixeira de vínculos no grafo, com restauração dos anexos preservados e exclusão
  definitiva restrita a administradores.
- Migração automática para o esquema 19 e rotas para listar, restaurar e apagar
  vínculos da lixeira.

### Corrigido

- Vínculos na lixeira deixam de aparecer no grafo, no cálculo psicossocial, na
  pesquisa e nas rotas de acesso aos anexos.
- A criação de um vínculo já enviado à lixeira orienta a restauração do registro.

## [0.6.5b] - 2026-09-18

### Adicionado

- Botões Enquadrar tudo e Tela cheia no grafo, com saída por Esc; enquadramento
  preserva as posições dos nós e se adapta ao tamanho da tela.
- Agrupamento por categoria com títulos e destaque das conexões ao selecionar
  uma pessoa, suavizando o restante da rede; opção para isolar as conexões.

### Corrigido

- Grafo organizado e enquadrado automaticamente ao abrir, trocar o layout ou
  alterar as conexões visíveis, sem restaurar posições antigas; enquadramento
  adaptado ao tamanho do quadro e botão Organizar grafo para reajuste manual.
- Visualizador da galeria centralizado na janela, com margens verticais e altura
  limitada; modais renderizados fora dos contêineres da página.
- Todo o quadro do administrador abre Configurações; o botão Sair mantém sua
  ação independente, inclusive no menu móvel.

## [0.6.5a] - 2026-09-17

### Adicionado

- Rodapé discreto com a versão e o link www.fabianocesar.com, inclusive no login.
- Justificativa e data de revisão no cadastro de risco psicossocial.
- Histórico de revisões com autor, instante e valores anteriores/novos, preservado
  nos backups e na lixeira; migração automática para o esquema 18.
- Prévia antes de salvar usando o cálculo real da rede, com HP previsto e lista
  de pessoas afetadas, sem persistir a simulação.
- Composição explícita do HP: base, risco próprio, vínculos e piso mínimo.
- Botão Atualizar mapa e sincronização das barras após aplicar cada snapshot.

### Alterado

- Risco próprio também reduz o HP da pessoa cadastrada, além dos vínculos.
- Barras do grafo com metade da largura, preenchimento sólido e percentual abaixo:
  verde a partir de 70%, amarelo de 40% a menos de 70%, vermelho abaixo de 40%.
- Nome dos nós sem HP duplicado; terminologia neutra para classificação e intensidade.
- Ícone do usuário autenticado abre Configurações, fechando o menu móvel.
- Publicação aceita tags com uma letra de revisão e gera imagens AMD64, ARM64 e
  ARMv7 no GHCR, além dos pacotes binários para quatro arquiteturas.

### Corrigido

- Consultas de indicadores sem reutilização do cache HTTP e atualização entre abas.
- Salvamento bloqueado quando o cadastro falha ao carregar; respostas antigas de
  outra pessoa em edição são descartadas.
- Gravação do histórico e dos metadados na mesma transação da classificação,
  evitando revisões parciais ou duplicadas quando os valores não mudam.

## [0.6.5] - 2026-09-17

### Adicionado

- HP psicossocial calculado em Rust a partir dos vínculos, com penalidades
  cumulativas, propagação residual de segundo grau e limite mínimo de segurança.
- Classificação de risco no cadastro e parâmetros de toxicidade, pesos dos
  vínculos, propagação, HP mínimo e cores configuráveis pelo administrador.
- Auras de risco, barra segmentada de vitalidade, métricas e contribuições no
  dossiê, com legenda no grafo e respeito à preferência por movimento reduzido.
- Especificação do cálculo e dois mockups do perfil e do grafo em
  `docs/HP_PSICOSSOCIAL.md` e `docs/mockups/`.
- Pesquisa de processos públicos no DataJud por número CNJ, com configuração
  opcional de `DATAJUD_API_KEY`.
- Criação opcional de eventos na agenda ao cadastrar pessoas, vínculos, arquivos
  e notas, preservando o contexto e permitindo novas tentativas seguras.

### Alterado

- Atualizações dos indicadores preservam o enquadramento do grafo.
- Backup e restauração incluem os dados e parâmetros psicossociais; as migrações
  são aplicadas automaticamente até a versão de esquema 17.
- Publicação no GitHub busca as tags antes de gerar as notas da Release.
- Exemplos de implantação e Portainer usam `ghcr.io/facrf/agendarx:0.6.5`.

### Corrigido

- Inicialização da identificação de backups criptografados compatível com a
  verificação do Clippy no Rust estável usado pela CI.

## [0.6.4] - 2026-09-15

### Adicionado

- Política de backup configurável pelo administrador, com horário, retenções
  diária, semanal e mensal, próximo agendamento e última falha visíveis.
- Manifesto, SHA-256, verificações de integridade, prévia do conteúdo e progresso
  de upload para restaurações.
- Limite independente `BACKUP_MAX_UPLOAD_BYTES` e transmissão de arquivos grandes
  sem manter todo o banco na memória do servidor.

### Alterado

- A restauração substitui o SQLite completo pela API nativa de backup, aplica
  migrações compatíveis, encerra sessões e recupera automaticamente a cópia de
  segurança quando a validação posterior falha.

## [0.6.3] - 2026-09-15

### Adicionado

- Galeria de imagens com navegação pelo teclado e por botões, além de notas em Markdown para anexos.
- Descrições Markdown de até 50.000 caracteres com pré-visualização.
- Etiquetas combináveis, favoritos por usuário e busca em contatos, descrições, notas e arquivos de texto.
- Lixeira recuperável para pessoas e auditoria das operações autenticadas.
- Backup SQLite diário com retenção de sete cópias, restauração e exportação ZIP AES-256.
- Legenda, filtros por tipo/data de vínculo e posições persistentes por usuário no grafo.
- Teste permanente de navegador para os principais fluxos regressivos.
- Dossiê exibe progresso por arquivo e permite reenviar uma falha ou todas, preservando os envios concluídos.
- Cadastro de administradores e usuários nas configurações, com autorização no servidor.
- Migração preserva as contas existentes como administradoras; novas contas têm perfil explícito.
- Busca, categoria, grau e layout do grafo são lembrados por conta neste navegador, com ação para limpar.
- Teste de integração para upload, recuperação de arquivos, miniaturas, limites, transações e permissões.

### Corrigido

- Foto principal otimizada e enviada por multipart com progresso e nova tentativa segura.
- Cor da categoria preservada quando um nó do grafo recebe foco.
- Filtros e agrupamento da lista de pessoas persistidos por usuário.
- Limite interno de 2 MiB que rejeitava fotos e anexos antes do limite configurado.
- Mensagens de upload agora mostram o motivo da falha no dossiê e preservam HTTP 413.
- Nova tentativa após falha na foto reutiliza a pessoa e os contatos já salvos.
- Edição de dados e contatos passa a ser transacional, sem alterações parciais.
- Fotos e anexos não persistem no cache HTTP do navegador.
- Anexos de vínculos que falham no formulário do grafo permanecem selecionados para nova tentativa.

### Alterado

- Páginas autenticadas carregadas sob demanda, reduzindo em cerca de 55% o JavaScript inicial.

## [0.6.2] - 2026-08-14

### Adicionado

- Detecção local de geotag em imagens JPEG, TIFF, PNG e WebP, exibindo mapa,
  coordenadas, altitude e dados disponíveis da captura.

### Corrigido

- Pessoas jurídicas mantêm o formato quadrado na tela do grafo, inclusive ao
  receber foco ou destaque.
- Imagens, vídeos, textos e PDFs permanecem alinhados e enquadrados na área útil
  do visualizador em telas grandes e pequenas.
- Drag-and-drop copia os arquivos enquanto o evento ainda está disponível,
  tornando anexos arbitrários compatíveis com Firefox, Chromium e gerenciadores
  de arquivos.

## [0.6.1] - 2026-08-14

### Adicionado

- Identificação de pessoas jurídicas com apresentação própria nos perfis e listas.
- Diagnóstico administrativo das configurações e serviços da aplicação.
- Leitura inicial de metadados EXIF e localização GPS nas imagens anexadas.

## [0.6.0] - 2026-08-14

### Adicionado

- Descrição livre de até 5.000 caracteres no cadastro, edição e visualização do
  perfil de cada pessoa.
- Exportação do mapa interpessoal pela impressão do navegador, pronta para salvar
  como PDF, com imagem da rede e listas de pessoas, contatos e vínculos visíveis.

### Alterado

- O relatório do mapa respeita categoria, pessoa em foco, grau de conexão e layout,
  e inclui nomes, descrições, categorias, meios de contato e detalhes dos vínculos.
- A API do grafo passa a fornecer descrição e meios de contato dos nós para
  relatórios.

### Corrigido

- Drag-and-drop de anexos no painel “Adicionar e editar arquivos” quando o navegador
  expõe itens protegidos, mas mantém os arquivos válidos no fallback do `DataTransfer`.

## [0.5.0] - 2026-08-14

### Adicionado

- Pesquisa textual na linha do tempo de achados, com paginação configurável em
  10, 50, 100 ou todos os resultados e contagem do intervalo exibido.
- Exclusão individual de achados da linha do tempo, preservando explicitamente
  eventuais PDFs já arquivados no dossiê.

### Alterado

- O visualizador compartilhado de anexos agora ocupa somente a área útil da
  janela, mantém PDFs dentro da tela e oferece ações para abrir em nova aba ou baixar.
- A extração de arquivos arrastados passou a usar as entradas nativas do
  `DataTransfer`, com fallback compatível e aviso para pastas.

### Corrigido

- Upload por drag-and-drop na agenda, no dossiê, nos vínculos e na foto de perfil,
  evitando que entradas de diretório sejam interpretadas como arquivos vazios.

## [0.4.0] - 2026-08-13

### Adicionado

- Fontes Querido Diário, INLABS/DOU e OpenAlex na Pesquisa Pública, selecionáveis
  individualmente por pesquisa, com resultados e metadados normalizados.
- Edição de pesquisas salvas com preservação da fonte escolhida e retrocompatibilidade
  automática das configurações anteriores com SearXNG.
- Configuração opcional de chave OpenAlex e credenciais/janela recente do INLABS,
  com falhas isoladas e mensagens amigáveis por provider.
- Arquivos e fotos anexáveis às tarefas, com miniaturas, pré-visualização e download.
- Aba de tarefas agendadas no perfil de cada pessoa.
- Movimentação de tarefas entre dias do calendário por arrastar e soltar, preservando
  horário e duração.
- Link direto das tarefas no perfil da pessoa para sua edição no calendário.
- Busca e filtros por pessoa, status, prioridade, anexos e período na visão mensal.
- Ações rápidas para concluir e reabrir tarefas e alternativa móvel para alterar a data.
- Upload de anexos com progresso, nova tentativa, cotas configuráveis e indicadores
  de armazenamento.
- Recorrências diárias, semanais e mensais por até um ano, com ocorrências independentes.
- Lembretes internos e notificações opcionais do navegador.
- Histórico de criação, edição, movimentação, status e anexos por tarefa.

### Corrigido

- Áreas de upload agora tratam o drag-and-drop de arquivos no Linux e a aplicação
  bloqueia a abertura acidental de arquivos soltos fora de uma área válida.

## [0.3.0] - 2026-08-12

### Adicionado

- Calendário mensal responsivo com tarefas de dia inteiro ou com horário, status,
  prioridade, cores e vínculo com até 50 pessoas cadastradas.
- Ícone privado e personalizável para o administrador, independente da identidade
  visual pública do sistema.

## [0.2.1] - 2026-08-12

### Adicionado

- A varredura OSINT agora informa execuções concluídas, parciais ou inconclusivas,
  identifica mecanismos temporariamente indisponíveis e registra métricas técnicas
  sem incluir o valor pesquisado nos logs.

### Alterado

- Nomes são pesquisados com e sem correspondência exata; CPF, CNPJ e telefone são
  consultados nas formas formatada e somente com dígitos, com eliminação de URLs
  duplicadas entre as variantes.

## [0.2.0] - 2026-08-12

### Adicionado

- Anexos de fotos, áudios e arquivos no histórico dos vínculos, com galeria,
  ampliação de imagens, reprodução e download.
- Ícone visual configurável, compartilhado pelo menu, login e favicon.
- Importação de contatos em vCard, Google/Outlook CSV e CSV genérico, além de
  exportação da agenda em CSV e vCard.
- Pré-visualizador compartilhado para imagens, vídeos, áudio, PDFs, textos e outros
  arquivos, com upload múltiplo e renomeação no dossiê.
- Alteração protegida do usuário e da senha do administrador pelas configurações,
  com revogação de todas as sessões.
- Miniaturas WebP geradas no backend para imagens do dossiê e dos vínculos, com
  cache SQLite e preenchimento automático dos anexos anteriores à atualização.

### Alterado

- Workflows restritos ao GitHub, impedindo execuções duplicadas no Gitea e Forgejo.
- A Stack do Portainer agora inclui um SearXNG privado com saída JSON habilitada e
  configura automaticamente a varredura, mantendo suporte a uma URL externa.
- O painel aberto ao clicar em uma aresta do grafo agora edita a relação e permite
  adicionar, visualizar, renomear e excluir anexos.

### Corrigido

- O seletor de foto da pessoa agora implementa drag-and-drop real e aceita imagens
  reconhecidas mesmo quando o Linux não informa o MIME do arquivo.
- O aviso HTTP 403 da varredura agora explica a saída JSON/limiter do SearXNG, e o
  healthcheck da Stack rejeita um `settings.yml` sem o formato `json`.

## [0.1.0] - 2026-08-11

### Adicionado

- Agenda de pessoas, categorias e meios de contato.
- Dossiê com imagens, áudio e anexos binários.
- Grafo interativo de vínculos interpessoais.
- Pesquisa pública via SearXNG e arquivamento automático de PDFs.
- Autenticação com Argon2, JWT e sessões revogáveis.
- Stack de implantação para Portainer.
- CI para Rust e React.
- Publicação multi-arquitetura no GHCR e pacotes de GitHub Release.
- Documentação de arquitetura, implantação, contribuição e segurança.

### Alterado

- Porta padrão da aplicação para `12000`.
- Referências de código, releases e imagens para `github.com/facrf/agendarx` e
  `ghcr.io/facrf/agendarx`.

### Corrigido

- Uploads ignoram campos multipart que não se chamem `arquivo`.
- A varredura OSINT bloqueia também as faixas IPv4 reservadas `0.0.0.0/8` e
  `240.0.0.0/4`, inclusive quando representadas em IPv6.
- O login administrativo vindo do ambiente é normalizado e limites inválidos
  de sessão ou upload são rejeitados na inicialização.
- O SQLx habilita somente SQLite, sem incluir drivers MySQL/PostgreSQL não usados.
- Actions, ferramenta de cross-compilação e imagens-base foram fixadas por SHA/digest
  para tornar a cadeia de publicação reproduzível e resistente a tags mutáveis.

[0.7.2]: https://github.com/facrf/agendarx/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/facrf/agendarx/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/facrf/agendarx/compare/v0.6.9...v0.7.0
[0.6.9]: https://github.com/facrf/agendarx/compare/v0.6.8...v0.6.9
[0.6.8]: https://github.com/facrf/agendarx/compare/v0.6.7...v0.6.8
[0.6.7]: https://github.com/facrf/agendarx/compare/v0.6.6...v0.6.7
[0.6.6]: https://github.com/facrf/agendarx/compare/v0.6.5e...v0.6.6
[0.6.5e]: https://github.com/facrf/agendarx/compare/v0.6.5d...v0.6.5e
[0.6.5d]: https://github.com/facrf/agendarx/compare/v0.6.5c...v0.6.5d
[0.6.5c]: https://github.com/facrf/agendarx/compare/v0.6.5b...v0.6.5c
[0.6.5b]: https://github.com/facrf/agendarx/compare/v0.6.5a...v0.6.5b
[0.6.5a]: https://github.com/facrf/agendarx/compare/v0.6.5...v0.6.5a
[0.6.5]: https://github.com/facrf/agendarx/compare/v0.6.4...v0.6.5
[0.6.4]: https://github.com/facrf/agendarx/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/facrf/agendarx/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/facrf/agendarx/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/facrf/agendarx/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/facrf/agendarx/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/facrf/agendarx/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/facrf/agendarx/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/facrf/agendarx/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/facrf/agendarx/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/facrf/agendarx/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/facrf/agendarx/releases/tag/v0.1.0
