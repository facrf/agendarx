# Melhorias de uso, organização e confiabilidade

## Importar contatos

Em **Configurações → Importar e exportar contatos**, selecione um CSV ou vCard.
O sistema mostra uma prévia; selecionar o arquivo ainda não cadastra pessoas.

Confira os meios de contato, as coincidências na agenda e os registros repetidos
no próprio arquivo. E-mails são comparados sem diferença de maiúsculas; telefones
são comparados sem formatação, incluindo a equivalência entre números brasileiros
com DDI 55 e sua forma nacional. Nomes sozinhos não identificam duplicados.

Escolha a ação de cada registro:

- **Ignorar:** não modifica a agenda. É o padrão para registros com coincidências.
- **Criar nova pessoa:** cadastra uma pessoa, mesmo quando houver coincidência.
- **Atualizar contato existente:** escolha a pessoa indicada. Acrescenta meios
  ausentes e preenche categoria vazia, preservando nome, descrição, foto, categoria
  já preenchida e contatos anteriores.

A prévia expira em 30 minutos. Enviar outro arquivo substitui a prévia anterior da
conta. Se contatos mudarem e as coincidências deixarem de ser válidas, envie o
arquivo novamente. **Confirmar importação** aplica todas as decisões de uma vez;
uma falha mantém a prévia disponível e não aplica mudanças parciais. Após sucesso,
o token não pode ser usado novamente. **Descartar prévia** encerra a revisão.

A lista mostra até 30 registros por página. A confirmação inclui as decisões de
todas as páginas. CSV e vCard continuam disponíveis para exportação.

## Acompanhar pesquisas públicas

No perfil da pessoa, abra **Pesquisa pública**, ative os parâmetros e clique em
**Iniciar varredura**. A pesquisa entra na fila e mostra quantos parâmetros foram
concluídos. Você pode sair da página; o processamento continua no servidor e o
progresso reaparece ao voltar. A tela consulta o estado a cada dois segundos.

Cada conta acompanha e controla seus próprios trabalhos. Pessoas e achados
continuam compartilhados. Só uma pesquisa por pessoa/conta pode permanecer ativa;
o servidor processa uma pesquisa por vez, com fila limitada.

**Cancelar pesquisa** interrompe o trabalho e preserva achados já arquivados.
**Retomar pesquisa** continua do último parâmetro concluído. Um parâmetro parcial
pode ser consultado novamente, com deduplicação de URLs. As contagens de um
parâmetro interrompido antes de salvar o progresso podem não incluir achados que
já foram arquivados; eles continuam disponíveis no histórico e no dossiê.

Após reinício, trabalhos em execução aparecem como **Interrompida** e exigem
retomada explícita. Após restauração de backup, a fila também é interrompida e as
prévias de importação são descartadas. Respostas antigas de pesquisas não são
gravadas no banco restaurado. Uma pesquisa concluída com fontes indisponíveis
mantém o resumo parcial/inconclusivo e os avisos; uma nova pesquisa permite
consultar as fontes novamente.

## Agenda e teclado

Ao trocar de mês, o calendário mantém os controles disponíveis, indica a
atualização e descarta respostas de meses anteriores. Lembretes de tarefas
reagendadas podem ser apresentados novamente. Falhas ao reconhecer um lembrete
são repetidas sem repetir o aviso já exibido naquela sessão.

Ao abrir um modal, o foco entra no diálogo. Tab e Shift+Tab percorrem seus
controles, Escape fecha apenas o modal superior e o foco volta ao acionador.
Fechar uma prévia dentro do formulário mantém o formulário aberto e a rolagem da
página bloqueada enquanto ele estiver ativo.


## Histórico e conflitos de edição

No perfil da pessoa, no formulário de tarefa e no painel de detalhes do vínculo,
abra **Versões anteriores e desfazer**. A lista mostra até 100 revisões recentes,
com autor, data e os dados anteriores. **Restaurar esta versão** exige confirmação
e guarda o estado atual como outra revisão; assim, a restauração também pode ser
revertida. O histórico começa com as edições realizadas após esta atualização.

Pessoas incluem dados do cadastro, avaliação de risco e meios de contato. Tarefas
incluem dados, status, lembrete e pessoas vinculadas. Vínculos incluem as duas
pessoas, tipo e descrição. Fotos, arquivos, notas de arquivos e etiquetas não são
substituídos por esse comando. Histórico de tarefa só fica disponível para sua
conta. Atualizações de contatos realizadas na importação também criam revisões.

Se alguém alterar o registro desde a abertura do formulário, salvar retorna um
aviso de conflito. Seus campos e o rascunho permanecem disponíveis. Reabra o
registro e compare os dados antes de reaplicar sua edição. Restaurar uma revisão
também exige que o registro não tenha mudado desde a consulta ao histórico.

## Mesclar pessoas duplicadas

Como administrador, abra **Configurações → Mesclar contatos duplicados**. Escolha
a origem e o destino; o destino será a pessoa que permanece na agenda. Clique em
**Revisar mesclagem** para conferir ambos os registros e selecionar os campos que
devem usar os valores da origem. Os campos não marcados mantêm o destino.

A confirmação transfere contatos distintos, anexos do dossiê, etiquetas, favoritos,
associações de tarefas, parâmetros e achados de pesquisas. Contatos idênticos do
mesmo tipo são reunidos; valores com formatação diferente permanecem disponíveis.
Achados com a mesma URL já existente no destino não substituem os existentes; os
registros originais continuam preservados no cadastro arquivado da origem.

As duas fotos são preservadas como anexos. Vínculos que ficariam duplicados ou
apontariam para a própria pessoa são arquivados como documentos no dossiê, com
seus arquivos e notas. Um relatório guarda os campos anteriores de ambos os
cadastros. Trabalhos de pesquisa ativos das duas pessoas são interrompidos para
impedir gravação de respostas recebidas durante a mesclagem.

A origem fica arquivada e sai da agenda e da lixeira comum. A mesclagem não pode
ser desfeita pelo histórico: revisões anteriores do destino ficam disponíveis
apenas para consulta, evitando reintroduzir dados separados depois da união.
Se qualquer cadastro mudar após a prévia, a confirmação será recusada; revise
novamente antes de confirmar. Tarefas privadas continuam pertencendo às mesmas
contas após a transferência das associações.

## Horário silencioso e adiar lembretes

Em **Configurações → Horário silencioso**, informe início e fim. O intervalo pode
atravessar a meia-noite, como 22:00–07:00. Os horários seguem o relógio local de
cada dispositivo e a preferência é salva por conta. Deixe os dois campos vazios
para desativar. Durante o intervalo, os avisos internos e do navegador ficam
pendentes; não são dispensados e poderão aparecer ao fim do período silencioso.

O aviso interno oferece **Adiar por 15 minutos**. O formulário da tarefa também
permite esse adiamento. A API aceita 5, 15, 30, 60 minutos ou um dia. O adiamento é
persistido no servidor e sobrevive ao recarregamento. Uma confirmação antiga do
aviso não pode cancelar um adiamento mais recente. Editar ou mover a tarefa limpa
o adiamento anterior e recalcula seu lembrete.

## Saúde do sistema

Como administrador, abra **Configurações → Saúde do sistema**. O painel reúne o
consumo do banco e das mídias, os estados das pesquisas e as últimas falhas de
trabalhos, além do último backup validado ainda disponível em arquivo e do erro
mais recente do agendamento de backup.

**Atualizar saúde do sistema** renova os dados. **Verificar último backup** confere
o checksum registrado e a integridade SQLite do arquivo mais recente. Backups
corrompidos ou ausentes deixam de ser apresentados como válidos. A data exibida
no resumo é a criação do backup; a mensagem da verificação descreve o resultado
obtido naquele momento.
