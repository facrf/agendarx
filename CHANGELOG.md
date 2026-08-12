# Histórico de alterações

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o
versionamento pretendido é [SemVer](https://semver.org/lang/pt-BR/).

## [Não publicado]

### Adicionado

- Anexos de fotos, áudios e arquivos no histórico dos vínculos, com galeria,
  ampliação de imagens, reprodução e download.
- Ícone visual configurável, compartilhado pelo menu, login e favicon.
- Importação de contatos em vCard, Google/Outlook CSV e CSV genérico, além de
  exportação da agenda em CSV e vCard.

### Alterado

- Workflows restritos ao GitHub, impedindo execuções duplicadas no Gitea e Forgejo.
- A Stack do Portainer agora inclui um SearXNG privado com saída JSON habilitada e
  configura automaticamente a varredura, mantendo suporte a uma URL externa.

### Corrigido

- O seletor de foto da pessoa agora implementa drag-and-drop real e aceita imagens
  reconhecidas mesmo quando o Linux não informa o MIME do arquivo.

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

[Não publicado]: https://github.com/facrf/agendarx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/facrf/agendarx/releases/tag/v0.1.0
