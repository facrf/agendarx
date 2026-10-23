# Auditoria e Endurecimento de Segurança — AgendarX

Data da auditoria: 23/10/2026  
Versão auditada: 0.6.9  
Status: Implementado e verificado

Este documento consolida a análise de segurança do código-fonte do **AgendarX**, detalhando as fragilidades identificadas, as medidas corretivas aplicadas e as diretrizes operacionais de proteção.

---

## 1. Vulnerabilidades Identificadas e Corrigidas

### 1.1 Injeção CRLF / Injeção de Propriedades na Exportação vCard (.vcf)
- **Classificação**: CWE-93 (Improper Neutralization of CRLF Sequences), Severidade Média.
- **Arquivo afetado**: `src/handlers/intercambio.rs` (`escapar_vcard`).
- **Problema**: O mecanismo anterior substituía `\n`, `\`, `,` e `;`, mas não removia nem tratava o caractere de retorno de carro (`\r` / `0x0D`). Entradas contendo sequências CRLF ou `\r` isoladas podiam quebrar a linha nos leitores de vCard (RFC 6350), possibilitando a injeção de propriedades arbitrárias ou cartões falsos no arquivo `.vcf` baixado pelo usuário.
- **Correção**: `escapar_vcard` foi atualizado para purgar explicitamente qualquer caractere `\r`, garantindo que nenhuma quebra de linha seja introduzida inadvertidamente na saída. Teste unitário adicionado cobrindo sequências maliciosas com `\r\n` e `\r`.

### 1.2 Ausência de Limites de Comprimento em Campos Textuais (DoS / Esgotamento de Memória)
- **Classificação**: CWE-400 (Uncontrolled Resource Consumption), Severidade Média.
- **Arquivos afetados**:
  - `src/handlers/pessoas.rs`: `validar_nome` e `validar_contato`.
  - `src/handlers/vinculos.rs`: `validar_vinculo`.
  - `src/handlers/configuracoes.rs`: `validar_texto`.
- **Problema**: Embora campos de texto longo como `descricao` possuíssem o teto de 50.000 caracteres, os campos de identificação primária (`nome`, `valor` do contato, `tipo_vinculo`, `nome_categoria` e `nome_tipo`) apenas verificavam se o valor não estava vazio. Um usuário autenticado podia enviar payloads de vários megabytes (até o teto da requisição), inflando tabelas e índices B-Tree e FTS5, além de causar sobrecarga de memória na serialização de listagens.
- **Correção**:
  - `nome` da pessoa limitado a **255 caracteres** (alinhado com o teto de importação).
  - `valor` de meios de contato limitado a **500 caracteres**.
  - `tipo_vinculo` limitado a **100 caracteres**.
  - `nome_categoria` e `nome_tipo_contato` limitados a **100 caracteres**.
  - Testes unitários adicionados para cada validação.

### 1.3 Ausência de Cabeçalhos Globais de Segurança HTTP
- **Classificação**: CWE-1021 (Improper Restriction of Rendered UI Layers or Frames), CWE-693.
- **Arquivo afetado**: `src/main.rs` (`construir_app`).
- **Problema**: Embora endpoints de download de blobs e fotos utilizassem `sandbox; default-src 'none'`, o servidor Axum não aplicava cabeçalhos defensivos na aplicação principal e no fallback da SPA, expondo a aplicação a riscos de Clickjacking/UI Redressing e MIME-sniffing.
- **Correção**: Implementado middleware global via `cabecalhos_seguranca` no Axum injetando:
  - `X-Frame-Options: SAMEORIGIN`
  - `X-Content-Type-Options: nosniff`
  - `Referrer-Policy: strict-origin-when-cross-origin`

### 1.4 Endurecimento de Configuração Padrão de Implantação
- **Classificação**: CWE-614 (Sensitive Cookie Without 'Secure' Attribute).
- **Arquivos afetados**: `deploy/portainer.env.example` e `.env.example`.
- **Problema**: `COOKIE_SECURE` constava como `false` nos exemplos, gerando risco de operadores subirem o sistema em HTTPS mantendo cookies sem a flag `Secure`.
- **Correção**: Configurado `COOKIE_SECURE=true` no arquivo de exemplo de produção (`portainer.env.example`) e documentada a obrigatoriedade da flag em ambientes protegidos por TLS.

---

## 2. Análise de Defesas Existentes no AgendarX

Durante a auditoria, foram confirmados os seguintes controles defensivos já consolidados:

1. **Injeção SQL**: Totalmente mitigada pelo uso do driver tipado `sqlx` com binds preparados (`?`). Concatenações de queries existentes utilizam apenas literais hardcoded ou constantes imutáveis validadas.
2. **SSRF em OSINT**: `resolver_destino_publico` valida hostnames e endereços IP, bloqueia redes privadas, loopback, documentação, CGNAT e benchmark, resolve o DNS antes da chamada e fixa os IPs na requisição (`resolve_to_addrs`), com política de redirecionamento desativada.
3. **Zip Slip / Path Traversal**: Extração de backups no `backup.rs` restrita pontualmente aos arquivos `agendarx.db` e `manifest.json`, gravados em caminhos temporários isolados com permissões restritas (`0600` em Unix).
4. **Resistência a Timing Attacks e Força Bruta**: Autenticação com Argon2id inclui verificação com hash de custo equivalente (`HASH_USUARIO_AUSENTE`) para usuários inexistentes, além de semáforo com concorrência máxima de 4 verificações simultâneas e limite de tentativas por login e globais.
5. **Injeção de Fórmulas CSV**: O método `escapar_csv` prefixa tabulação (`\t`) em campos iniciados com `=`, `+`, `-` ou `@`.

---

## 3. Checklist de Implantação Segura

Para operar o AgendarX em ambiente produtivo:

- [ ] Gerar segredo criptográfico forte para `JWT_SECRET` (pelo menos 32 caracteres hexadecimais aleatórios).
- [ ] Ativar `COOKIE_SECURE=true` ao publicar atrás de proxy reverso com TLS/HTTPS.
- [ ] Configurar senhas exclusivas e complexas para `ADMIN_PASSWORD` no primeiro bootstrap.
- [ ] Restringir a rede do contêiner SearXNG para que apenas o backend AgendarX tenha comunicação direta.
- [ ] Manter o volume `/app/data` em armazenamento com permissões estritas para o usuário `agendarx` (UID 1000/system).
