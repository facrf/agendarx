# A fazer — melhorias do AgendarX

Plano autorizado em 06/10/2026 e concluído em 07/10/2026. Todos os itens abaixo
foram implementados e verificados.

- [x] **Exportação CSV/vCard:** corrigir a ordem de JOIN/WHERE e cobrir os downloads com teste HTTP.
- [x] **Autenticação:** limitar tentativas, concorrência e corpo do login; usar verificação de senha equivalente para contas inexistentes; coordenar troca de credenciais e emissão de sessões.
- [x] **Uploads:** limitar o recebimento por rota e por arquivo antes de acumular conteúdo excessivo em memória; preservar o limite independente de restauração.
- [x] **Agenda:** impedir respostas antigas ao trocar o mês e permitir lembretes após reagendamento, com recuperação de falhas transitórias.
- [x] **Publicação:** executar regressões de navegador na CI e exigir as verificações de qualidade antes de publicar versões.
- [x] **Importação:** apresentar prévia, identificar coincidências por telefone/e-mail e permitir ignorar, atualizar ou criar cada contato.
- [x] **Pesquisas públicas:** persistir trabalhos com progresso, cancelamento e recuperação, respeitando autenticação e manutenção do banco.
- [x] **Modais:** gerenciar foco inicial, Tab/Shift+Tab, retorno ao acionador e diálogos sobrepostos.

## Critérios de conclusão

- Testes Rust e regressões de navegador relevantes aprovados.
- `cargo fmt --all -- --check`, Clippy, lint e build frontend aprovados.
- Documentação de uso e contratos HTTP atualizada para os novos fluxos.
- Todas as alterações e artefatos locais limitados à árvore deste projeto.

## Validação

Verificações locais aprovadas em 07/10/2026:

- `cargo test --locked --offline`: 67 testes aprovados.
- `cargo clippy --locked --offline --all-targets -- -D warnings`: sem avisos.
- `cargo fmt --all -- --check`: aprovado.
- `npm run lint` e `npm run build`: aprovados.
- Regressão completa `frontend/tests/browser-regression.mjs` com Chromium do
  Playwright: aprovada, incluindo os oito fluxos e os recursos existentes.
- Actionlint para os workflows e `git diff --check`: aprovados.

Os testes HTTP cobrem exportação, prévia sem gravação, confirmação e revalidação de
coincidências, isolamento dos trabalhos por conta, limites de login e uploads
concorrentes. Testes do worker cobrem retomada do checkpoint, cancelamento em voo e
rejeição de resultados de uma geração antiga do banco. Os testes de navegador
cobrem respostas antigas do calendário, lembretes com falha transitória, foco em
modais sobrepostos e os fluxos de importação e pesquisa.

## Documentação e compatibilidade

- [Guia de uso das melhorias](docs/MELHORIAS.md).
- [Contratos HTTP](docs/API.md) e [arquitetura](docs/ARQUITETURA.md) atualizados.
- Migrações 22 e 23 acrescentam prévias e trabalhos; são aplicadas na inicialização.
- A importação retorna uma prévia e exige confirmação. A varredura pública retorna
  HTTP 202 com um trabalho consultável. Clientes da API devem adotar esses contratos.
- Pesquisas com parâmetro interrompido podem ter contagens parciais; os achados já
  arquivados permanecem disponíveis, conforme explicado no guia.
- Alterações registradas no [histórico](CHANGELOG.md), na seção não publicada.


## Segunda rodada — confiabilidade e organização

Autorizada e concluída em 07/10/2026.

- [x] Histórico de edições e restauração de versões de pessoas, tarefas e vínculos.
- [x] Mesclagem revisável de contatos duplicados, preservando notas e arquivos.
- [x] Detecção de conflitos de edição com versões verificadas na transação.
- [x] Horários silenciosos por conta e adiamento de lembretes.
- [x] Painel de saúde com armazenamento, pesquisas e verificação de backup.

Validação desta rodada aprovada:

- 72 testes Rust, incluindo conflitos concorrentes, desfazer, isolamento do
  histórico de tarefas, mesclagem de vínculos e arquivos, adiamento com
  reconhecimento tardio e detecção de backup corrompido.
- Clippy sem avisos, formatação Rust, lint e build frontend aprovados.
- Regressão completa de navegador aprovada; os novos fluxos também passaram
  separadamente após os ajustes finais.
- Actionlint e `git diff --check` aprovados.

O [guia de uso](docs/MELHORIAS.md) e os [contratos HTTP](docs/API.md) explicam os
novos fluxos. A migração 24 é aplicada na inicialização. Clientes de API devem
usar `If-Match` para proteger edições concorrentes. Mesclagem exige revisão e
confirmação e não é desfeita pelo histórico; os dados anteriores ficam arquivados.

## Terceira rodada — endurecimento de segurança e auditoria

Autorizada e concluída em 23/10/2026.

- [x] **Sanitização CRLF no vCard:** remoção de retornos de carro (`\r`) em `escapar_vcard` para prevenir injeção de propriedades e cartões maliciosos na exportação de contatos.
- [x] **Limites de tamanho em campos textuais (CWE-400):** validação de comprimento máximo em `nome` (255 caracteres), `valor` de contato (500 caracteres), `tipo_vinculo` (100 caracteres), `nome_categoria` (100 caracteres) e `nome_tipo` (100 caracteres) para mitigar sobrecarga de memória e banco de dados.
- [x] **Cabeçalhos globais de segurança HTTP:** aplicação de middleware injetando `X-Frame-Options: SAMEORIGIN`, `X-Content-Type-Options: nosniff` e `Referrer-Policy: strict-origin-when-cross-origin` para proteção contra Clickjacking/UI Redressing e MIME-sniffing.
- [x] **Configuração e templates de implantação:** reforço da diretriz de segurança com `COOKIE_SECURE=true` para ambientes protegidos por TLS nos exemplos de deploy.
- [x] **Documentação de segurança:** elaboração do relatório técnico em `docs/SEGURANCA.md`.

Validação desta rodada aprovada:
- 81 testes Rust executados e aprovados (`cargo test`), incluindo teste de regressão para os novos cabeçalhos globais de segurança.
- `cargo clippy --all-targets -- -D warnings`: aprovado sem avisos.
- `cargo fmt --all -- --check`: aprovado.

## Quarta rodada — otimizações de banco de dados, consultas e frontend

Autorizada e concluída em 23/10/2026.

- [x] **Modo WAL e synchronous no SQLite:** ativação de `journal_mode=WAL` e `synchronous=NORMAL` em `src/db/mod.rs` para permitir leituras e escritas concorrentes sem travamento e diminuir I/O de disco.
- [x] **Eliminação de N+1 queries no calendário:** implementação de carregamento em lote (`montar_respostas_em_lote`) para pessoas vinculadas, anexos e contagem de séries em `src/handlers/calendario.rs`, reduzindo centenas de roundtrips para 3 consultas no banco.
- [x] **Otimização da contagem de paginação de contatos:** separação da consulta de contagem em `src/handlers/pessoas.rs`, removendo subconsultas custosas de etiquetas e favoritos na paginação.
- [x] **Cálculo localizado de HP psicossocial para perfil único:** introdução de `calcular_hp_pessoa` em `src/handlers/hp_psicossocial.rs` e `src/handlers/pessoas.rs`, restringindo consultas e processamento estritamente ao subgrafo relevante (até 2º grau) da pessoa consultada em vez de varrer toda a base.
- [x] **Cache e controle de recarga por foco no frontend:** introdução de cache em memória para tipos de contato e contatos no calendário, e throttling de 30 segundos em eventos de foco/visibilidade em `GraphPage.tsx` e `PersonProfilePage.tsx`.
- [x] **Cabeçalhos de cache de assets imutáveis:** injeção de `Cache-Control: public, max-age=31536000, immutable` para `/assets/*` e `no-cache` para o HTML em `src/main.rs`.
- [x] **Otimização de cache de camadas no Dockerfile:** reorganização do `Dockerfile` movendo a cópia de `migrations` para depois do build das dependências, preservando a compilação de crates quando apenas migrações mudam.

Validação desta rodada aprovada:
- 81 testes Rust executados e aprovados (`cargo test`).
- `cargo clippy --all-targets -- -D warnings`: sem avisos.
- `cargo fmt --all -- --check`: aprovado.

## Quinta rodada — Progressive Web App (PWA) e Exportação GraphML

Autorizada e concluída em 23/10/2026.

- [x] **Suporte a Progressive Web App (PWA):** criação do manifesto Web (`frontend/public/manifest.webmanifest`), meta tags mobile/PWA no HTML com preservação da assinatura FACRF, registro de Service Worker (`frontend/public/sw.js`) com estratégia Stale-While-Revalidate para o shell da aplicação e fallback offline, além de cabeçalhos `no-cache` dedicados no backend Axum para Service Worker e WebManifest.
- [x] **Exportação do grafo em formato GraphML padrão:** implementação do endpoint `/api/vinculos/grafo/graphml` e serializador XML com escape de caracteres especiais, exportando nós com metadados cadastrais, classificação de risco, toxicidade e indicadores psicossociais completos (HP, aura, vitalidade), além de arestas direcionadas com tipo e data de criação para compatibilidade com Gephi, Cytoscape Desktop, yEd e NetworkX.
- [x] **Exportação do grafo em formato GEXF nativo do Gephi:** implementação do endpoint `/api/vinculos/grafo/gexf` gerando XML GEXF 1.2 com suporte a atributos visuais (`viz:color` com cálculo RGB), classificação e métricas psicossociais completas.
- [x] **Ações de exportação no frontend:** integração de botões de download direto para GraphML e GEXF na barra de ferramentas do mapa de relacionamentos (`GraphPage.tsx`).
- [x] **Notificações PWA e clique no Service Worker:** integração de `showNotification` no Service Worker para dispositivos móveis e evento `notificationclick` com foco automático na tarefa vinculada.
- [x] **Gerenciador de instalação PWA:** componente `PwaInstallManager` nas configurações para instalação do AgendarX em desktop e mobile.
- [x] **Cobertura de testes e validação:** testes unitários para a geração de GraphML e GEXF e testes de integração HTTP cobrindo headers de anexo, formato XML e autorização de sessão.
- [x] **Sincronização remota:** push sincronizado com os servidores Git configurados.

Validação desta rodada aprovada:
- 83 testes Rust executados e aprovados (`cargo test`).
- `cargo clippy --all-targets -- -D warnings`: sem avisos.
- `cargo fmt --all -- --check`: aprovado.
- Build de produção do frontend (`tsc -b && vite build`) compilado com sucesso.
