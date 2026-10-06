# Painel, busca e recuperação de trabalho

## Painel inicial

Após entrar, a página **Seu dia** reúne tarefas pendentes ou em andamento em três
listas: **Hoje**, **Atrasadas** e **Próximos sete dias**. Cada lista mostra até 20
itens e o total de pendências. Clique no título para abrir a tarefa no calendário
ou em **Concluir** para finalizar diretamente.

O dia segue o fuso horário do navegador. Tarefas de dia inteiro usam sua data de
calendário, sem deslocamento para o dia anterior por conversão de fuso. A lista
Hoje considera tarefas que começam hoje; Atrasadas considera tarefas de dias
anteriores cujo término já passou. Os próximos sete dias começam amanhã. Tarefas
concluídas ficam fora das três listas. Cada conta vê somente suas próprias tarefas.
O painel atualiza ao voltar à janela, após alterações em tarefas e a cada minuto
quando a aba está visível.

## Busca de pessoas

A lista de pessoas usa paginação no servidor, com 10, 30, 50 ou 100 itens por
página. Nome, contatos, descrição, etiquetas, notas e conteúdo dos anexos de texto
entram na busca. Arquivos ligados a vínculos e tarefas também podem localizar
as pessoas associadas; anexos de tarefas de outra conta ficam fora da pesquisa.
Categoria, tipo de pessoa e favoritos são aplicados antes de contar e paginar.
Os indicadores de pessoas e fotos correspondem aos resultados filtrados.
Favoritos aparecem primeiro, seguidos de nome e ID para ordenação estável.
O agrupamento visual organiza os resultados da página atual.

A busca ignora caixa e acentos e aceita prefixos de palavras: `reuniao` encontra
`Reunião` e `reuni` encontra `reuniões`. Várias palavras precisam estar no mesmo
documento. A busca aceita até 200 caracteres e considera as primeiras 12 palavras;
pontuação separa palavras e não executa operadores SQL ou expressões FTS.
A busca por trechos no meio de uma palavra foi substituída por busca de prefixos.

## Busca global

Use o campo **Buscar no sistema** no topo ou `Ctrl+K` (`Cmd+K` no macOS), digite
o termo e pressione Enter. A página `/busca` reúne pessoas, tarefas, vínculos e
arquivos. O filtro de tipo limita os resultados e a navegação exibe 30 por página.
Os resultados apresentam o título e um trecho próximo do termo encontrado.

Pessoas abrem o perfil; tarefas abrem a ocorrência no calendário; vínculos abrem
seus detalhes no grafo. Arquivos abrem o conteúdo autenticado em outra aba.
Nome e descrição das pessoas, meios de contato, etiquetas, títulos e descrições
de tarefas, nomes das pessoas ligadas ao vínculo, tipo/descrição do vínculo,
nomes e notas de arquivos e conteúdo de arquivos de texto compõem o índice.
PDFs, imagens, áudio e vídeo são pesquisados pelo nome e pelas notas; não há OCR
ou transcrição automática. Itens na lixeira e vínculos com pessoas excluídas não
aparecem. As tarefas e seus arquivos continuam privados por conta.

## Rascunhos de formulários

Formulários de pessoas, tarefas e vínculos salvam alterações automaticamente
neste navegador, separadas por conta e pelo ID da pessoa ou tarefa. O editor de
vínculos mantém um rascunho por conta, incluindo o ID da relação em edição.
A edição no painel de detalhes de um vínculo guarda um rascunho por relação;
fechar os detalhes preserva o trabalho e **Cancelar** descarta essa edição. Ao voltar ao
formulário, escolha **Recuperar rascunho** para preencher os campos ou **Descartar
rascunho** para usar o cadastro do servidor. A recuperação é explícita para
permitir comparar com dados que possam ter sido alterados em outra sessão.

Rascunhos duram 30 dias e são removidos após salvar o formulário com sucesso.
Os campos de eventos associados a pessoas e vínculos também são preservados.
Fotos e arquivos selecionados não são armazenados: selecione-os novamente.
Rascunhos ficam no armazenamento local do navegador, não são enviados ao servidor
nem incluídos nos backups do banco. Limpar os dados do navegador remove os
rascunhos. Se o navegador não permitir armazenamento ou a cota acabar, a tela
informa que não conseguiu salvar o rascunho.

No calendário, abra **Nova tarefa** ou a edição da ocorrência para recuperar o
respectivo rascunho. No grafo, abra **Gerenciar vínculos** para acessar o formulário.

## Organização do grafo

Arraste as pessoas e clique em **Salvar organização**. As posições ficam salvas
para sua conta e separadas entre os layouts Teia e UML. **Carregar organização**
aplica as posições salvas às pessoas visíveis e preserva as posições dos nós que
não estavam no salvamento. O mapa se enquadra ao concluir a recuperação.

A recuperação é manual: abrir o mapa continua usando organização automática.
Salvar um recorte atualiza somente as pessoas desse recorte; posições das demais
pessoas permanecem salvas. O agrupamento não possui um conjunto de posições
separado: cada layout tem uma organização por conta. **Organizar grafo** recalcula
o desenho atual e não altera a cópia salva até clicar em **Salvar organização**.

## Auditoria

Administradores encontram a auditoria em **Configurações → Lixeira e auditoria**.
Os filtros permitem combinar usuário, trecho do caminho do recurso, ação,
período e código HTTP. Clique em **Filtrar auditoria** para aplicar. Datas são
interpretadas em UTC e o dia final é incluído. A lista é paginada e consulta todo
o histórico, ultrapassando o limite de 500 registros da listagem antiga.

Abra uma operação para consultar ID, método HTTP, duração do atendimento em
milissegundos e resultado. Quando possível, **Abrir recurso** leva ao perfil,
à tarefa ou ao vínculo. Registros anteriores à atualização preservam seus campos
originais e não possuem método/duração. A duração mede o atendimento do handler,
antes da gravação do próprio registro e sem medir transmissão da resposta.
O registro não armazena corpos de requisições, senhas ou conteúdo de anexos.

## Carregamento e atualização

O grafo abre carregando apenas o snapshot e as categorias. A lista de vínculos
é derivada das arestas do snapshot; pessoas para o formulário e a lixeira são
consultadas ao abrir o gerenciamento. Atualizações de HP carregam somente o
snapshot e preservam posições, seleção e zoom quando a estrutura não mudou.
Eventos de foco e visibilidade são agrupados por 150 ms. Requisições GET
simultâneas para o mesmo caminho compartilham a resposta quando não têm um sinal
de cancelamento. Categorias usam cache em memória de 60 segundos; seletores de
pessoas no grafo usam 30 segundos. Escritas, uploads e encerramento de sessão
invalidam esse cache; respostas antigas não repovoam um cache já invalidado.
Busca e paginação cancelam consultas anteriores quando seus parâmetros mudam.

## Banco e implantação

As migrações `0020_busca_indexada.sql` e `0021_auditoria_detalhes.sql` são aplicadas
automaticamente na inicialização. A primeira cria os documentos pesquisáveis,
o índice FTS5 e triggers para manter inserções, edições e exclusões sincronizadas.
Ela também indexa os registros existentes. A segunda acrescenta os detalhes da
auditoria. A validação de backups passa a reconhecer o esquema 21 e continua
aceitando backups de versões anteriores conforme o fluxo de restauração.

O índice aumenta o tamanho do SQLite porque mantém uma cópia pesquisável do texto.
A primeira inicialização pode demorar mais conforme o volume de arquivos de texto.
Os backups completos incluem as novas tabelas e o índice.

## Validação

`cargo test --locked` cobre paginação, favoritos por conta, acentos, atualização
do índice, lixeira, privacidade de tarefas, painel, auditoria e posições do grafo,
além das regressões existentes de backups, calendário, autenticação e anexos.
No frontend, execute `npm run lint`, `npm run build` e `npm run test:browser`.
O teste de navegador usa respostas controladas para exercitar os fluxos visuais;
os testes Rust verificam o comportamento da API e do SQLite reais.

Validação desta implementação em 06/10/2026: 59 testes Rust aprovados,
build de produção e lint aprovados, além da regressão completa em Chromium.
O navegador também verifica enquadramentos repetidos de um nó isolado sem
alterar os limites internos do Cytoscape e recuperação de rascunhos no painel
de detalhes dos vínculos. A validação não altera o banco de produção.
