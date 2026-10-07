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
