# Workflows do Forgejo desativados

Este diretório existe intencionalmente e não deve conter arquivos `.yml` ou
`.yaml`. O Forgejo encontra sua pasta nativa e, portanto, não faz fallback para
os workflows exclusivos do GitHub em `.github/workflows`.

Como defesa adicional para instalações com diretórios personalizados, os jobs do
projeto só executam quando `github.server_url` é `https://github.com`.
