# HP psicossocial, auras e visualização de impactos

Status: implementado na versão 0.6.5. O HP é uma simulação configurável de exposição na rede;
classificações e pesos são informados no cadastro, sem inferência automática.

## Indicadores independentes

A **aura de risco comportamental** usa a toxicidade T do próprio perfil. A
**Vitalidade psicossocial · HP** usa as penalidades recebidas dos vínculos.
Uma pessoa pode ter aura crítica e HP preservado, ou aura estável e HP impactado.
A cor da categoria permanece na moldura interna do avatar; a aura é externa.

| T próprio | Aura padrão | Cor | Pulsação |
|---|---|---|---|
| 0 ≤ T < 0,10 | Estável | `#22C55E` | Não |
| 0,10 ≤ T < 0,25 | Elevado | `#EAB308` | Não |
| 0,25 ≤ T < 0,40 | Observação | `#F97316` | Não |
| 0,40 ≤ T ≤ 1 | Crítico | `#EF4444` | Sim |

| HP | Vitalidade padrão | Cor |
|---|---|---|
| 0 ≤ HP < 0,25 | Impactada | `#EF4444` |
| 0,25 ≤ HP < 0,50 | Baixa | `#F97316` |
| 0,50 ≤ HP < 0,75 | Atenção | `#EAB308` |
| 0,75 ≤ HP ≤ 1 | Preservada | `#22C55E` |

Nomes, cores, limiares e pulsação da aura são administráveis. O limiar inferior é
inclusivo. Os selos do perfil não exibem classificações de traços; elas ficam no
cadastro. “Não classificado” utiliza T = 0, sem representar uma avaliação concluída.

## Cadastro e validação

O formulário de criação/edição contém `classificacao_risco` e `toxicidade`:

- `NAO_CLASSIFICADO` e `SEM_RISCO`: T = 0.
- `MANIPULATIVO`, `PATOLOGICO` e `MISTO`: T entre os limites administrativos,
  inicialmente 0,10 e 0,50, inclusive. O usuário informa o valor.
- Criação sem os dois campos usa `NAO_CLASSIFICADO`, T = 0.
- Edição sem os dois campos preserva os valores atuais. Enviar apenas um retorna
  400. Para remover o risco ativo, enviar classificação sem risco e T = 0 juntos.

Pesos e HP são frações na API: 50% = 0.50; a interface mostra porcentagens.
Não aceitar NaN, infinito, números fora dos limites ou valores 50/100 para pesos.
HP é somente leitura. Categoria e tipo de pessoa não determinam toxicidade.

## Função Rust e cálculo

A função pura está em `src/domain/hp_psicossocial.rs`:

```rust
pub fn calcular_hp_psicossocial(
    nos: &[NoPsicossocial],
    arestas: &[ArestaPsicossocial],
    config: &ConfigHpPsicossocial,
) -> Result<Vec<AtualizacaoHp>, ErroHp>
```

Ela retorna uma atualização por pessoa ativa, ordenada por ID. Não acessa banco,
rede, relógio ou estado global. Usa `f64` sem arredondamento intermediário e soma
as contribuições em ordem de ID das arestas; tolerância numérica dos testes: 1e-9.
IDs inválidos/repetidos, autovínculos, pontas inexistentes, tipos vazios e arestas
que violam a unicidade atual são erros de domínio.

Para cada aresta direcionada A → B:

```text
P_AB = T_A × W_AB
D_B = soma das penalidades diretas de entrada
R_C = soma das contribuições residuais elegíveis
HP_C = clamp(hp_base − D_C − R_C, hp_min, hp_base)
```

P = 0,30 reduz 30 pontos percentuais; não é 30% do HP atual. Família possui peso
1,00; Profissional, 0,50. Tipos não cadastrados usam o peso padrão 0,50.
Os pesos pertencem às arestas, sem relação com a categoria da pessoa.
Tipos/chaves recebem `trim()` e `to_lowercase()` na consulta dos pesos, mantendo
acentos; `família` e `familia` são aliases explícitos nos defaults. Exposições de
tipos distintos entre o mesmo par somam-se; chaves administrativas repetidas
após normalização são rejeitadas.

Para cada A → B de risco, C recebe `fator_segundo_grau × P_AB`, inicialmente 20%:

1. C é vizinho social de B em qualquer direção: B → C ou C → B.
2. C deve estar a distância social mínima exatamente 2 de A. Excluir A, B e todos
   os vizinhos diretos de A, inclusive aqueles conectados no sentido inverso.
3. Cada combinação `(aresta A → B, C)` contribui uma vez. Várias arestas B/C não
   duplicam o residual dessa exposição.
4. Arestas de origem distintas e caminhos via B distintos acumulam resíduos.
5. O residual usa P original mesmo quando B atinge o piso. Não propagar o residual,
   o HP perdido de B ou qualquer impacto para o 3º grau.
6. B só produz impactos próprios quando possui T cadastrado. Fontes também podem
   receber exposição de outras fontes; não são imunes aos resíduos da rede.

O cálculo sempre parte do HP base; repetir a consulta não repete descontos.
Remover um vínculo, reduzir T/W ou retirar uma pessoa da rede recupera HP.
Pessoas na lixeira e suas arestas são removidas antes do cálculo; restaurar uma
pessoa recompõe sua exposição. Filtros visuais não alteram os valores calculados.
As penalidades brutas são preservadas, mesmo quando a perda aplicada atinge o piso.

## Configuração administrativa

Em **Configurações → HP psicossocial**, somente administradores podem editar:

| Campo | Default | Limite |
|---|---|---|
| `ativo` | true | Booleano |
| `toxicidade_min` / `toxicidade_max` | 0,10 / 0,50 | 0 < min ≤ max ≤ 1 |
| `hp_base` / `hp_min` | 1,00 / 0,05 | 0 < piso ≤ base ≤ 1 |
| `fator_segundo_grau` | 0,20 | 0 a 1 |
| `peso_padrao` | 0,50 | 0 a 1 |
| `pesos_vinculo` | Família 1,00; Profissional 0,50 | Tipos e pesos editáveis |
| `faixas_aura` | Tabela de risco acima | Limite, nome, cor, pulsação |
| `faixas_vitalidade` | Tabela de HP acima | Limite, nome, cor |

Faixas começam em zero, têm limiares crescentes e cores `#RRGGBB`. São permitidas
1 a 32 faixas por indicador. Desativar o cálculo restaura HP base e zera descontos;
a aura continua mostrando o risco cadastrado. Fator residual zero conserva os
impactos diretos. Alterações entram em vigor sem reinício.

Configuração usa versão otimista: enviar `versao_esperada`; versão divergente
retorna 409. Novos limites que invalidam perfis, inclusive na lixeira, retornam
409 sem alteração parcial. Ajustar esses perfis antes de reduzir os limites.
As escritas de configuração e risco usam `BEGIN IMMEDIATE` no SQLite para validar
sob a mesma transação e impedir corrida entre limites e cadastro.

## API e rastreabilidade

| Método e rota | Uso |
|---|---|
| `GET /api/configuracoes/hp-psicossocial` | Qualquer sessão autenticada; configuração com versão |
| `PUT /api/configuracoes/hp-psicossocial` | Admin; parâmetros completos e `versao_esperada` |
| `POST /api/pessoas`, `PUT /api/pessoas/{id}` | Aceitam classificação e toxicidade juntos |
| `GET /api/pessoas/{id}` | Indicadores no objeto `psicossocial` |
| `GET /api/vinculos/grafo` | Indicadores em cada nó; configuração em `hp_configuracao` |

Cada conjunto de indicadores inclui HP/base/piso, somas direta/residual, aura de
risco, cor/nome de vitalidade, estado do cálculo, versão e `contribuicoes`.
Cada contribuição identifica fonte/nome, alvo direto/nome, vínculo, tipo, T,
peso, grau (1/2) e penalidade. Fonte e intermediário são explicados sem depender
de filtros do desenho. O perfil e o grafo leem snapshots consistentes, usando a
mesma função e uma transação de leitura para pessoas, arestas e parâmetros.

Erros seguem `{"erro":"mensagem"}`: 400 para entradas inválidas, 401 sem sessão,
403 sem permissão, 409 para conflitos. Dados persistidos inválidos encontrados na
leitura retornam 500; não produzir resultados parciais.

## Interface e atualização

A barra tem 20 segmentos: HP = 10% preenche exatamente dois; valores intermediários
usam preenchimento parcial do segmento, preservando a proporção exata. O número
fica visível, com semântica `progressbar`, para não depender apenas da cor.
Hover, foco ou toque abre a composição do impacto; o Dossiê possui resumo gráfico
e detalhes expansíveis das contribuições. O link **Abrir no mapa** liga o perfil
ao grafo. A legenda e os estados estão em português. As amostras da legenda acompanham
as faixas e cores salvas pelo administrador.

Selecionar um nó destaca vizinhos de 1º e 2º grau e identifica fontes pelas
contribuições. O 2º grau usa traços no contorno e nas arestas correspondentes.
O contador indica apenas nós/arestas visíveis. A barra de cada nó tem a mesma
proporção do perfil; o PDF inclui HP numérico, aura e somas de penalidade.

A pulsação é suave, com período aproximado de 3,6 segundos. A explicação também
pode ser fechada por Escape. CSS e Cytoscape
respeitam `prefers-reduced-motion`; no grafo, a pulsação fica suspensa quando a aba
está oculta. As categorias conservam suas cores internas.

Após uma gravação de pessoa, vínculo, configuração ou lixeira, o cliente invalida
os indicadores e consulta o snapshot atualizado. No formulário de vínculo, a
consulta ocorre antes de aguardar anexos e eventos opcionais. Abas do mesmo
navegador recebem aviso por `storage`; retomar/focar o perfil ou grafo também
atualiza. Não existe push para outros dispositivos: eles atualizam ao retomar a
página. Respostas antigas do grafo são descartadas por sequência de consulta.
Falha na atualização do grafo mantém os últimos valores e oferece nova tentativa.

Cytoscape permanece na mesma instância. Atualizações apenas de dados/HP/aura
preservam posição, zoom e seleção. Mudanças de topologia ou layout executam layout;
editar pontas de uma aresta recria a aresta correspondente.

## Banco, backup e exemplos

A migração `0017_hp_psicossocial.sql` adiciona classificação/T à pessoa e cria
`hp_psicossocial_configuracao`, registro único com parâmetros JSON, versão,
autor e data. Bancos anteriores/importações sem risco usam T = 0. HP/aura não
são persistidos como descontos. Backup/restauração mantém pesos, faixas e perfis;
a validação de backup reconhece o schema 17.

| Fontes diretas sobre B, com C ligado somente a B | HP B | HP C |
|---|---|---|
| 0,30 × Família 1,00 | 70% | 94% |
| 0,30 × Profissional 0,50 | 85% | 97% |
| 0,30 × 1,00 + 0,40 × 0,50 | 50% | 90% |
| 0,50 × 1,00 + 0,50 × 1,00 | 5% | 80% |

A aura de B depende de T_B, independentemente desses resultados de HP.

## Verificação e referências

Testes unitários cobrem soma, piso, direção, ciclos, duplicação de residual,
múltiplos caminhos, reversibilidade, independência aura/HP, parâmetros e validação.
Teste HTTP cobre cadastro, omissão de campos, permissões, conflitos de versão,
limites incompatíveis, consistência perfil/grafo e lixeira/restauração.
O teste de backup verifica a sobrevivência dos campos e da configuração.
A regressão de navegador cobre 20 segmentos/10%, atualização preservando o mapa,
legenda, destaque de 2º grau, contagem de arestas visíveis, redução de movimento,
resumo no Dossiê e edição administrativa.

```bash
cargo fmt --check
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cd frontend
npm run build
npm run lint
npm run test:browser
```

O frontend foi validado com Node 24. A regressão usa Chromium local, em
`/usr/bin/chromium-browser` por padrão; `CHROMIUM_PATH` permite outro executável.
Os dados da regressão são simulados. `BROWSER_SCREENSHOTS=1` habilita capturas de
perfil e grafo em `.cache/`, sem incluir esses dados no banco.

Validação da entrega isolada em 17/09/2026: 52 testes Rust aprovados; formatação, Clippy
com `-D warnings`, build TypeScript/Vite, ESLint e regressão completa de navegador
aprovados. A regressão também verifica as cores e os traços das arestas e o
fechamento da explicação por Escape.

O conjunto completo, incluindo DataJud e eventos relacionados à agenda, também
foi validado nesta data: 56 testes Rust aprovados, além dos mesmos checks de
formatação, Clippy, build, ESLint e navegador.

Referências das APIs utilizadas: [SQLx Pool::begin_with](https://docs.rs/sqlx/0.8.6/sqlx/struct.Pool.html#method.begin_with),
[Cytoscape: atualizações em lote](https://js.cytoscape.org/#cy.batch) e
[Cytoscape: underlay](https://js.cytoscape.org/#style/underlay).
Os [mockups](mockups/README.md) são referências conceituais anteriores à
implementação; a separação entre risco e vitalidade e os textos em português
estão definidos neste documento.
