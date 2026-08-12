# Workflows do Gitea desativados

Este diretório existe intencionalmente e não deve conter arquivos `.yml` ou
`.yaml`. O Gitea usa o primeiro diretório de workflows existente; assim, ele não
faz fallback para `.github/workflows` e não executa as automações destinadas ao
espelho no GitHub.

Se o administrador da instância tiver personalizado `WORKFLOW_DIRS` para ler
diretamente `.github/workflows`, os jobs ainda possuem uma condição que permite
execução somente quando `github.server_url` é `https://github.com`.
