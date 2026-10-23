# Grafo de relacionamentos

O mapa interpessoal mostra as pessoas visíveis e os vínculos entre elas. Ele se
organiza e se enquadra automaticamente ao abrir, trocar o layout ou mudar os
filtros, para manter relações e indicadores dentro do quadro.

## Navegação

- **Teia** distribui as pessoas em uma rede orgânica; **UML** apresenta os
  vínculos em níveis hierárquicos.
- **Organizar grafo** recalcula as posições; use após arrastar nós ou alterar
  muitos vínculos.
- **Enquadrar tudo** centraliza e ajusta o zoom sem mudar as posições atuais.
- **Tela cheia** amplia a área de análise. Use o botão de saída ou `Esc` para
  voltar à página.
- A roda do mouse ajusta o zoom e o arrasto no espaço vazio move o mapa. Arraste
  uma pessoa para reposicioná-la durante a análise atual.
- Os controles **+**, **−** e **Enquadrar mapa** ficam no canto do canvas, com o zoom
  atual em percentual. Redimensionar preserva a aproximação e o centro manuais.
- O mapa ocupa a largura disponível. **Criar ou editar vínculo** abre o formulário,
  a lista de vínculos e a lixeira; **Recolher vínculos** libera a área lateral.
- **Salvar organização** grava posições por conta e layout. **Carregar organização**
  recupera somente os nós com posições salvas. A recuperação é explícita e novos
  nós mantêm a posição atual. Consulte [o guia de produtividade](PRODUTIVIDADE.md).

## Foco e agrupamento

Use a busca para escolher uma pessoa. Um clique em um nó coloca a pessoa no
centro, ajusta o zoom para seus relacionamentos até o segundo grau e ativa
**Mostrar apenas conexões** e **Tela cheia**. As conexões diretas ficam destacadas
e as conexões de segundo grau aparecem tracejadas. Clique novamente na mesma
pessoa para recentralizar após mover o mapa. Use **Sair da tela cheia** ou `Esc`
para voltar à página; desmarque **Mostrar apenas conexões** para ver o restante
da rede, que fica suavizado enquanto houver foco.

Em **Pessoas**, cada cartão oferece **Criar vínculo**. O perfil também mantém essa
ação disponível em todas as abas. A pessoa escolhida já aparece como origem;
busque e selecione a outra pessoa, informe o tipo e confira a prévia antes de salvar.
A pessoa de origem não aparece entre as opções de destino. Os rascunhos de criação
são guardados por pessoa. Expanda **Descrição, anexos e evento (opcional)** para
complementar o vínculo; os campos principais e o botão de salvar ficam em destaque.

No grafo, **Criar vínculo com [nome]** usa a pessoa em foco e funciona também em
tela cheia. O botão direito sobre uma pessoa oferece **Criar vínculo**, **Editar
pessoa**, **Abrir perfil** e **Focar conexões**. Sobre uma linha, oferece **Editar
vínculo**, **Ver detalhes do vínculo** e **Mover vínculo para a lixeira** (com
confirmação). Use as setas para navegar pelo menu e `Esc` para fechá-lo. Os botões
visíveis permitem realizar as mesmas ações pelo teclado e em telas de toque.

No perfil de uma pessoa, **Abrir no mapa** abre esse mesmo recorte em tela cheia.
A seleção usa o ID, mesmo quando existem nomes iguais. Nessa abertura, filtros
salvos de categoria, tipo de vínculo e período são limpos para permitir a
visualização dos relacionamentos.

O foco se adapta ao tamanho da janela e reserva espaço para nomes, auras e barras
de vitalidade. Pessoas sem vínculos também ficam centralizadas, com aproximação
limitada para manter a visualização confortável.

Marque **Agrupar por categoria** para separar visualmente as pessoas por categoria.
Cada grupo recebe um título; pessoas sem categoria são reunidas em **Sem categoria**.
Essa opção funciona nos layouts Teia e UML.

## Filtros e detalhes

Os filtros de categoria, tipo de vínculo e período definem quais relações entram
no mapa. A profundidade de 1º, 2º ou 3º grau controla o recorte quando o foco está
isolado. Clique em uma linha para ver seus detalhes e dê duplo clique em uma pessoa
para abrir seu perfil.

Nos detalhes de uma linha, use **Excluir vínculo** para mover a relação para a
lixeira após confirmar. Ela sai do grafo e do cálculo psicossocial, mas os anexos
ficam preservados. Abra **Criar ou editar vínculo → Lixeira de vínculos** para restaurar a relação
com seus anexos. Um administrador pode excluí-la definitivamente nessa lista.
Se uma das pessoas também estiver na lixeira, restaure a pessoa primeiro.

O botão **Gerar PDF** usa exatamente o recorte visível do mapa e abre a janela de
impressão, onde é possível escolher **Salvar como PDF**.

O botão **Exportar GraphML** baixa o arquivo `.graphml` com todos os nós e arestas da
rede, incluindo metadados cadastrais, classificação de risco, toxicidade e indicadores
psicossociais (HP, aura, vitalidade), pronto para importação em ferramentas de análise
de redes como Gephi, Cytoscape Desktop, yEd e NetworkX.

O botão **Exportar GEXF** gera o arquivo `.gexf` (Graph Exchange XML Format) padrão do
Gephi com namespace visual (`viz:color`), atributos estruturados e mapeamento de
cores das categorias, permitindo carregar o grafo diretamente com estilização.
