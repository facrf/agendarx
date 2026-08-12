# Política de segurança

## Versões suportadas

Enquanto o projeto estiver na série `0.x`, somente a versão mais recente publicada
recebe correções de segurança.

## Relato de vulnerabilidades

Não publique vulnerabilidades, credenciais ou dados pessoais em uma issue aberta.
Use [Security > Report a vulnerability](https://github.com/facrf/agendarx/security/advisories/new)
no GitHub. Caso o relato privado não esteja habilitado, contate o mantenedor por um
canal privado informado em seu perfil.

Inclua, quando possível:

- versão ou commit afetado;
- cenário e impacto;
- passos mínimos para reprodução;
- mitigação conhecida, sem dados pessoais reais.

## Operação segura

- Defina um `JWT_SECRET` aleatório com pelo menos 32 caracteres.
- Use HTTPS e `COOKIE_SECURE=true` quando houver proxy reverso TLS.
- Restrinja o acesso à porta da aplicação e proteja backups do SQLite.
- Mantenha a imagem e as dependências atualizadas.
- Use o módulo OSINT apenas para fontes públicas e finalidade legítima.
- Não exponha o banco, anexos ou variáveis de ambiente em logs e tickets.
