# Histórico de alterações

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o
versionamento pretendido é [SemVer](https://semver.org/lang/pt-BR/).

## [Não publicado]

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

[Não publicado]: https://github.com/facrf/agendarx/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/facrf/agendarx/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/facrf/agendarx/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/facrf/agendarx/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/facrf/agendarx/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/facrf/agendarx/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/facrf/agendarx/releases/tag/v0.1.0
