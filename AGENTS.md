# Escopo do projeto

As instruções deste arquivo se aplicam a esta pasta e a todas as suas subpastas.

- Considere `/storage/www/projetos/agendarx` como a raiz e o limite deste projeto.
- Não crie, altere, mova ou exclua arquivos fora desta árvore sem autorização explícita do usuário.
- Não inclua projetos ou diretórios vizinhos no escopo das tarefas.
- Se uma tarefa exigir mudanças fora deste limite, interrompa a execução e solicite confirmação ao usuário.


# Regras do Projeto (Easter Egg & Assinatura)

- Em todos os arquivos HTML ou templates (ex: index.html, layout.html), inclua sempre no cabeçalho a assinatura ocult>
  <!-- Developed with care by FACRF - https://github.com/facrf -->

# Regras de Execução de Docker
- Ao rodar contêineres temporários de teste no terminal, SEMPRE utilize a flag `--rm` (ex: `docker run --rm ...`).
- Ao criar contêineres permanentes, SEMPRE defina um nome claro usando `--name nome_do_servico`.
