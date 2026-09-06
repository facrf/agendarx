# Diagnóstico técnico — AgendarX

Data da auditoria: 22/08/2026.

Esta auditoria combinou revisão estática, execução dos testes e verificações dinâmicas locais. Nenhum arquivo de código foi alterado durante a análise.

## Achados de prioridade alta

1. **Possível bypass da proteção SSRF por hostname com ponto final.**
   Em `src/handlers/osint.rs:532-546` a requisição usa a URL original, enquanto em `:593-626` a validação remove o ponto final do hostname antes de fixar a resolução DNS. Um domínio controlado pode resolver para IP público na validação e para IP privado em uma segunda resolução. Corrigir normalizando o host também na URL efetivamente requisitada ou rejeitando hostnames com ponto final.

2. **Rotação de senha não é atômica.**
   O login verifica o hash antes de criar a sessão (`src/handlers/auth.rs:57-110`) e a alteração de credenciais revoga sessões em uma transação posterior (`:182-255`). Um login com a senha antiga pode concluir depois da revogação e manter acesso. A atualização deve comparar o hash esperado dentro da transação e coordenar a emissão de sessões.

3. **Enumeração e sobrecarga do login.**
   Usuário inexistente retorna antes do Argon2, enquanto senha inválida para usuário existente executa Argon2 (`src/handlers/auth.rs:46-79`). Em teste local, os tempos foram aproximadamente 1,5 ms e 440 ms, respectivamente. Não há limite de tentativas, concorrência ou tamanho específico da senha. Usar hash falso para usuários inexistentes, rate limit e limite de corpo específico para login.

4. **Credenciais administrativas públicas no fluxo de instalação.**
   `.env.example` define `admin` e `troque-esta-senha` (`.env.example:24-25`); `docs/IMPLANTACAO.md:42-49` instrui copiá-lo e iniciar o sistema; o bootstrap cria a conta em banco vazio (`src/db/mod.rs:48-78`). Exigir variáveis sem valores conhecidos ou forçar troca de senha no primeiro acesso.

5. **Artefato 0.6.2 implanta 0.6.0 por padrão.**
   A versão atual está em `Cargo.toml:3`, mas `deploy/portainer-stack.yml:3` aponta para `ghcr.io/facrf/agendarx:0.6.0`. Além de instalar funcionalidades antigas, isso pode impedir a inicialização de bancos migrados por versões posteriores. Atualizar todos os defaults de imagem junto com a release.

6. **Cadastro e edição de pessoa podem deixar dados parciais ou duplicados.**
   `frontend/src/pages/PersonFormPage.tsx:123-154` salva pessoa, contatos e foto em requisições independentes. Falha da foto após o POST deixa a tela em “nova pessoa”; uma tentativa seguinte cria duplicata. Na edição, uma falha entre exclusões e upserts deixa o conjunto de contatos parcial. Criar endpoint transacional ou tratar o ID recém-criado e operações recuperáveis.

7. **Lembretes podem ser suprimidos ou omitidos.**
   O `Set` global de IDs processados nunca é limpo (`frontend/src/components/TaskReminderWatcher.tsx:7-25`), inclusive após reagendamento ou falha. No backend, `LIMIT 200` é aplicado antes de verificar se o lembrete venceu (`src/handlers/calendario.rs:459-483`). Foi reproduzido um lembrete vencido invisível após 200 tarefas anteriores ainda não vencidas.

8. **Escape em modal aninhado fecha também o formulário externo.**
   Cada `Modal` registra listener global para Escape (`frontend/src/components/ui.tsx:137-146`). Ao fechar uma prévia de anexo dentro da edição de tarefa (`frontend/src/pages/CalendarPage.tsx:677`), ambos os modais recebem o evento e dados não salvos podem ser perdidos. Implementar uma pilha de modais ou deixar apenas o modal ativo reagir ao Escape.

9. **A release não depende das verificações de qualidade.**
   A CI não roda para tags (`.github/workflows/ci.yml:3-8`) e o workflow de release só compila o frontend, sem `fmt`, Clippy, testes ou lint (`.github/workflows/release.yml:42-77`). A formatação atual já falha em `src/handlers/configuracoes.rs`. Executar esses gates no release ou torná-los requisito do tag.

10. **Datas de dia inteiro usam UTC de forma inconsistente com o fuso local.**
    `frontend/src/pages/CalendarPage.tsx:260-296` grava meia-noite e fim de recorrência com `Z`, enquanto tarefas com horário são convertidas do fuso local. Em São Paulo, um lembrete de tarefa de dia inteiro pode vencer às 21h do dia anterior; o limite de recorrência também pode excluir a última ocorrência local.

## Achados de prioridade média

1. **Panic possível na busca INLABS com Unicode.** `src/handlers/osint/providers.rs:1022-1031` usa offset em bytes obtido da string em lowercase para fatiar o texto original em `:1061-1070`. Lowercase Unicode pode alterar o tamanho dos bytes; usar índices de caracteres ou mapeamento seguro.

2. **Concorrência permite ultrapassar 30 anexos por tarefa.** A contagem é feita fora de transação antes de consumir o multipart (`src/handlers/calendario.rs:676-692`). Com 29 anexos e dois uploads concorrentes, a reprodução terminou com 31 anexos. Garantir a regra no banco ou serializar a contagem e inserção.

3. **Rotas `/api` inexistentes retornam `200 text/html`.** O fallback da SPA também captura a API (`src/main.rs:56-68`). Uma rota desconhecida deve retornar JSON e status 404; rejeições de JSON inválido também devem seguir o mesmo contrato de erro.

4. **INLABS consulta data UTC no contêiner.** `Local::now()` em `src/handlers/osint/providers.rs:418-424` usa UTC no runtime sem `TZ` configurado. Entre 21h e meia-noite em Brasília, com lookback padrão, o DOU do dia corrente é omitido. Configurar `America/Sao_Paulo` explicitamente ou trabalhar com data de negócio definida.

5. **Risco de consumo excessivo de memória com ZIP.** Há limites por ZIP e por XML, mas não para o total descompactado retido em memória (`src/handlers/osint/providers.rs:905-930`). Limitar o total por arquivo, por execução e processar documentos em streaming.

6. **Respostas assíncronas antigas podem sobrescrever a tela atual.** O carregamento de tarefas não usa cancelamento ou contador de geração (`frontend/src/pages/CalendarPage.tsx:122-150`). Há corrida semelhante no histórico e no carregamento de anexos de vínculos (`frontend/src/pages/GraphPage.tsx:179-196`). Usar `AbortController` ou token de requisição.

7. **Tarefa que atravessa meia-noite não faz round-trip pela interface.** A API aceita intervalos em dias diferentes, mas o formulário recompõe o fim usando a data inicial (`frontend/src/pages/CalendarPage.tsx:263-270`). A tarefa volta inválida ao ser editada.

8. **Backup documentado pode ficar inconsistente.** `docs/IMPLANTACAO.md:151-153` manda copiar o SQLite em execução. Usar o comando de backup do SQLite, checkpoint coordenado ou parar o serviço antes da cópia.

9. **Stack do Portainer não é compatível com todos os modos anunciados.** A regex de healthcheck contém `$` não escapado em `deploy/portainer-stack.yml:61`, fazendo `docker stack config` falhar em Swarm. O arquivo também usa recursos específicos de Compose/standalone.

10. **Documentação e release têm inconsistências adicionais.** O README promete imagem RISC-V, mas a release publica apenas amd64, arm64 e arm/v7 (`.github/workflows/release.yml:191`); o pacote também referencia caminhos `deploy/` que não são incluídos no tarball.

## Validações executadas

- `cargo test --locked`: 37 testes aprovados.
- `cargo clippy --locked --all-targets -- -D warnings`: aprovado.
- ESLint, TypeScript e build Vite do frontend: aprovados.
- `cargo fmt --all -- --check`: reprovado por formatação em `src/handlers/configuracoes.rs`.
- Testes HTTP locais confirmaram o fallback `/api` retornando HTML 200, o cenário de anexos acima do limite e o lembrete vencido não listado.

## Estado do repositório

Ao encerrar a auditoria, nenhum artefato temporário permaneceu. A única modificação rastreada era pré-existente em `.github/workflows/release.yml` (configuração de cache da imagem).
