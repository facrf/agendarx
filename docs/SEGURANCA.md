# Auditoria e Endurecimento de Segurança — AgendarX

Data da auditoria: 25/10/2026  
Versão auditada: 0.7.2  
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

### 1.5 Cabeçalhos Globais Content-Security-Policy (CSP) e Permissions-Policy
- **Classificação**: CWE-79 (Cross-Site Scripting), CWE-1021 (Improper Restriction of Rendered UI Layers).
- **Arquivo afetado**: `src/main.rs` (`cabecalhos_seguranca`).
- **Problema**: O frontend SPA e as rotas gerais da aplicação não declaravam uma política explícita de Content-Security-Policy nem Permissions-Policy, embora os endpoints de entrega de anexos já contassem com sandbox isolada.
- **Correção**: Injetados cabeçalhos restritivos por padrão no middleware global:
  - `Content-Security-Policy`: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; media-src 'self' blob:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'self';`
  - `Permissions-Policy`: `camera=(), microphone=(), geolocation=()`
  - O cabeçalho preserva regras específicas mais estritas caso a resposta já possua CSP definido (ex: `sandbox; default-src 'none'` para arquivos do dossiê).

### 1.6 Auditoria Completa de Tentativas de Autenticação (Login)
- **Classificação**: CWE-778 (Insufficient Logging).
- **Arquivo afetado**: `src/handlers/auth.rs` (`login`).
- **Problema**: As rotas públicas de autenticação não eram interceptadas pelo middleware de auditoria de rotas autenticadas. Dessa forma, tentativas de invasão, ataques de força bruta ou falhas por senha incorreta não eram visíveis na tabela `auditoria` do painel administrativo.
- **Correção**: Implementado registro automático em `auditoria` para todos os eventos de autenticação:
  - Logins bem-sucedidos (`status_http: 200`, ação `EXECUTAR`, recurso `auth/login`).
  - Falhas por senha ou login inválido (`status_http: 401`, ação `EXECUTAR`, recurso `auth/login`).
  - Bloqueios preventivos de taxa/concorrência (`status_http: 429`, ação `EXECUTAR`, recurso `auth/login`).
  - Todos os registros gravam duração e resumo explicativo, permitindo monitoramento forense direto na aba **Configurações → Lixeira e auditoria**.

### 1.7 Correção de CVEs e Atualização de Dependências (Ciclo Outubro/2026)
- **Classificação**: Múltiplas (RCE / DoS / Parsing XML & ZIP / Memory Safety / ReDoS / Prototype Pollution).
- **Escopo**: Ecossistemas Rust (Cargo) e Node.js/TypeScript (npm).
- **Problemas Mitigados**:
  - `zip` (RUSTSEC-2026-0369 [CVSS 9.1 Crítico], RUSTSEC-2026-0368, RUSTSEC-2026-0367): vulnerabilidades graves de heap overflow e negação de serviço na extração e descompressão de arquivos ZIP no módulo de restauração de backups (`backup.rs`). Atualizado para `zip = "9.1.0"`.
  - `quick-xml` (RUSTSEC-2026-0195, RUSTSEC-2026-0194): vulnerabilidades de panics descontrolados e DoS por alocação excessiva em parsing de tags e CDATA no provedor OSINT (`src/handlers/osint/providers.rs`). Atualizado para `quick-xml = "0.42.0"`.
  - Atualização do lockfile Cargo (`cargo update`): resolução de 17 advisories de segurança em dependências transitivas, incluindo `rustls 0.23.46`, `rustls-webpki 0.103.16`, `webpki-root-certs 1.0.11`, `tokio 1.54.1`, `quinn-proto 0.11.19` e `h2 0.4.20`.
  - Frontend (`npm audit fix`): correção de 4 vulnerabilidades de severidade High identificadas em outubro/2026, atualizando `vite` para `8.3.5`, `zod` para `4.6.7`, `brace-expansion` para `5.0.12` e `source-map-js` para `1.2.2`. Relatório final com 0 vulnerabilidades no `npm audit`.

---

## 2. Análise de Defesas Existentes no AgendarX

### 2.1 Modelo Operacional e Premissas de Rede (LAN / VPN)
O **AgendarX foi projetado prioritariamente para operação em redes locais (LAN) e redes privadas virtuais (VPN)**, atendendo equipes em ambientes controlados e isolados. Por essa razão:
- O sistema não depende de CDNs externas ou scripts de terceiros (toda a interface, fontes, ícones e assets são estáticos e servidos localmente).
- Adota-se o princípio de **defesa em profundidade**: mesmo dentro de uma rede corporativa ou VPN privada, onde outros dispositivos podem estar comprometidos, a aplicação isola privilégios, audita tentativas de autenticação, restringe origens e valida rigorosamente o tráfego.

### 2.2 Controles Defensivos Consolidados
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
