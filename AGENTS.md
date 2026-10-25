# Escopo do projeto

As instruções deste arquivo se aplicam a esta pasta e a todas as suas subpastas.

- Considere `/storage/www/projetos/agendarx` como a raiz e o limite deste projeto.
- Não crie, altere, mova ou exclua arquivos fora desta árvore sem autorização explícita do usuário.
- Não inclua projetos ou diretórios vizinhos no escopo das tarefas.
- Se uma tarefa exigir mudanças fora deste limite, interrompa a execução e solicite confirmação ao usuário.


# Regras do Projeto (Easter Egg & Assinatura)

- Em todos os arquivos HTML ou templates (ex: index.html, layout.html), inclua sempre no cabeçalho a assinatura oculta:
  <!-- Developed with care by FACRF - https://github.com/facrf -->

# Regras de Versionamento e Sincronização (Frontend x Backend)

- A versão exibida em todas as telas, rodapés e páginas do frontend DEVE ser rigorosamente a mesma versão do software (`Cargo.toml`).
- O frontend NUNCA deve exibir uma versão defasada ou desincronizada em relação ao software backend.
- Para assegurar a paridade absoluta de versão:
  1. **Paridade em Arquivos de Configuração**: Toda alteração de versão no software (`Cargo.toml`) deve ser refletida de forma síncrona em `frontend/package.json` e `frontend/package-lock.json`.
  2. **Invalidação de Cache do Service Worker**: O cache do Service Worker (`sw.js`) deve ser versionado com o identificador da versão atual do app (ex.: `agendarx-shell-v{versao}`), garantindo a invalidação imediata e a remoção de caches legados de versões anteriores no navegador do usuário ao atualizar.
  3. **Consulta Dinâmica em Tempo de Execução**: As páginas e componentes do frontend (em especial o `AppFooter`) devem consultar a versão do software via API (`/api/identidade/versao` ou `/api/versao`) e sincronizar o estado em tempo de execução, usando a versão estática do pacote apenas como valor inicial imediato (fallback sem flicker).
  4. **Recompilação Obrigatória de Build**: Toda alteração de versão ou entrega exige a recompilação limpa do frontend (`npm run build` gerando `frontend/dist`) com permissões normais de usuário do workspace (nunca deixar artefatos gerados como `root`).
  5. **Garantia por Testes Automatizados**: A suite de testes do projeto deve validar a igualdade estrita entre a versão do `Cargo.toml` e a versão do `frontend/package.json`.

# Regras de Git e Espelhamento de Repositórios (Gitea x GitHub)

- O servidor Gitea local (`http://192.168.0.10:3010/facrf/agendarx.git`) espelha o repositório principal no GitHub (`https://github.com/facrf/agendarx.git`).
- A publicação de imagens OCI no GHCR (`ghcr.io/facrf/agendarx`) e a execução das pipelines de CI/CD ocorrem via GitHub Actions no GitHub através do push de tags de release (`vMAJOR.MINOR.PATCH`).
- Ao realizar envios (`git push`):
  1. O remote `origin` está configurado para propagar pushes tanto para o Gitea local quanto para o GitHub.
  2. Garanta sempre o envio sincronizado dos commits e tags para o remote `origin` e `github` (`git push origin main && git push origin --tags` / `git push github main && git push github --tags`), mantendo paridade absoluta entre a infraestrutura local e a remota e acionando a compilação e publicação da imagem no GHCR.

# Regras de Execução de Docker
- Ao rodar contêineres temporários de teste no terminal, SEMPRE utilize a flag `--rm` (ex: `docker run --rm ...`).
- Ao criar contêineres permanentes, SEMPRE defina um nome claro usando `--name nome_do_servico`.
