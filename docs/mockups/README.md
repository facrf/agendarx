# Mockups de HP psicossocial

Duas imagens de referência para o layout do AgendarX, geradas com a ferramenta
integrada `image_gen`. São mockups estáticos; o halo representa visualmente a
pulsação proposta, e os dados e identidades são fictícios.

- [image_0.png](image_0.png): perfil com aura escarlate externa, HP em 10% e resumo
  de métricas no Dossiê.
- [image_1.png](image_1.png): mapa de vínculos com auras, barras de vitalidade e
  legenda dos indicadores.

A paleta acompanha o frontend existente: fundo `#F5F3ED`, navegação `#123B3A`,
cartões brancos e detalhes coral. A aura externa permanece separada da moldura
interna do avatar. O HP de 10% corresponde a 2 de 20 segmentos preenchidos.

Os requisitos de cálculo e configuração estão em
[HP_PSICOSSOCIAL.md](../HP_PSICOSSOCIAL.md).

## Prompt final: perfil

```text
Use case: ui-mockup.
Asset type: high-fidelity desktop web app screenshot for AgendarX, landscape 16:9, approximately 1920x1080, crisp readable typography.
Visual system grounded in the existing AgendarX interface: warm ivory canvas #F5F3ED, deep teal #123B3A full-height left navigation about 240 pixels wide, ivory/white cards with 24px rounded corners, subtle shadows, Inter-like sans serif body typography and Avenir-like headings. Thin line icons, small coral #E7654F accents, restrained professional Portuguese investigator workspace. Brand upper left reads exactly "AgendarX", with small subtitle "RELAÇÕES VIVAS". Left navigation reads "Pessoas", "Calendário", "Mapa de vínculos", "Configurações". Sidebar bottom has a small administrator avatar and "admin". Generous but efficient spacing, balanced polished product design. Use neutral fictional sample identities only.
Critical status is visualized by a slightly transparent scarlet #EF4444 organic glowing outer ring with 2 soft flowing concentric arcs, suggesting a pulse in a single still frame. Keep an independent existing thin teal inner circular photo border; the red aura must not replace the inner border. No personality diagnoses or insulting/judgmental labels anywhere.
Vitality bars are refined RPG-inspired segmented UI, not arcade graphics: exactly 20 equal rectangular segments with narrow gaps. 10% means precisely the first TWO of 20 segments filled red; the remaining 18 segments are empty light-gray with subtle reddish outlines. Visible numeric "10%" must agree with the bar.
No browser chrome, no device frame, no perspective tilt, no external margins, no watermark, no illustration of a computer, no sci-fi HUD, no decorative circuitry, no neon overload, no illegible tiny text. The requested result is a clean flat screenshot of the application.
Primary request: create the PROFILE AND DOSSIER screen, a beautiful advanced investigator dashboard with the new multilayer behavioral aura and vitality indicator integrated into the existing AgendarX layout.
Composition: sidebar at left with "Pessoas" active. Main workspace top has breadcrumb "Pessoas / Perfil", a small search field and subdued utilities. A large wide white profile card dominates the upper main area.
Profile card: at left a large realistic circular headshot of a fictional Brazilian male professional aged about 38, dark hair, neutral expression, off-white photographic background. Preserve thin teal inner border, add clearly visible translucent flowing scarlet outer halo. To the right display the fictional name exactly "Rafael Costa" in large dark teal text, with a neutral small "Profissional" category chip above it. DIRECTLY BELOW THE NAME show the exact label "Psychosocial Vitality HP", a wide 20-segment horizontal vitality bar filled with only TWO red segments, and the large numeric "10%" aligned at its right end. A discreet graph-link button on the card right reads exactly "Abrir no mapa" with a node-link icon, plus a small "Editar" button. Do not put judgmental status text beneath the name.
Below: a segmented tab navigation containing "Perfil", "Tarefas", "Dossiê", "Pesquisa pública", with "Dossiê" selected in deep teal.
Dossiê section: large polished white card headed "Dossiê" and subtitle "Resumo psicossocial". Include a clear horizontal summary chart with three accurately proportional bars on one common 0%-100% axis: "Exposição direta" 75% in red, "Estresse secundário" 15% in orange, "Vitalidade" 10% in muted red. Small clear tick labels 0%, 25%, 50%, 75%, 100%. Adjacent three compact metric tiles repeat 75%, 15%, 10% with their matching labels. Use restrained visual semantics, no diagnosis chart. Below this summary show a compact attachment row headed "Arquivos e registros", two attractive document cards labeled "Notas de contexto" and "Histórico de vínculos", and a subtle "Adicionar arquivo" button.
Right auxiliary column: a card headed "Conexões" with a compact tasteful mini-network diagram and the link "Ver grafo completo →". Beneath it show a small card headed "Atualizações recentes" with 3 concise entries "Vínculo atualizado", "Registro adicionado", "Métricas recalculadas", and an understated "Parâmetros configuráveis" line with settings icon. All key UI labels must be legible. This is the primary person-profile screenshot, not a montage or split-screen.
```

## Prompt final: grafo

```text
Use case: ui-mockup.
Asset type: high-fidelity desktop web app screenshot for AgendarX, landscape 16:9, approximately 1920x1080, crisp readable typography.
Visual system grounded in the existing AgendarX interface: warm ivory canvas #F5F3ED, deep teal #123B3A full-height left navigation about 240 pixels wide, ivory/white cards with 24px rounded corners, subtle shadows, Inter-like sans serif body typography and Avenir-like headings. Thin line icons, small coral #E7654F accents, restrained professional Portuguese investigator workspace. Brand upper left reads exactly "AgendarX", with small subtitle "RELAÇÕES VIVAS". Left navigation reads "Pessoas", "Calendário", "Mapa de vínculos", "Configurações". Sidebar bottom has a small administrator avatar and "admin". Generous but efficient spacing, balanced polished product design. Use neutral fictional sample identities only.
Critical status is visualized by a slightly transparent scarlet #EF4444 organic glowing outer ring with 2 soft flowing concentric arcs, suggesting a pulse in a single still frame. Keep an independent existing thin teal inner circular photo border; the red aura must not replace the inner border. No personality diagnoses or insulting/judgmental labels anywhere.
Vitality bars are refined RPG-inspired segmented UI, not arcade graphics: exactly 20 equal rectangular segments with narrow gaps. 10% means precisely the first TWO of 20 segments filled red; the remaining 18 segments are empty light-gray with subtle reddish outlines. Visible numeric "10%" must agree with the bar.
No browser chrome, no device frame, no perspective tilt, no external margins, no watermark, no illustration of a computer, no sci-fi HUD, no decorative circuitry, no neon overload, no illegible tiny text. The requested result is a clean flat screenshot of the application.
Primary request: create the matching GRAPH VIEW screenshot, the second screen of the same AgendarX dashboard, presenting an interpersonal network and the new aura/vitality legend with an elegant composition.
Composition: same full-height deep teal sidebar and warm ivory app canvas; "Mapa de vínculos" is active. Workspace headline exactly "Mapa de vínculos", subtitle "Explore conexões e impactos na rede." Top action row contains a search field, small tabs "Rede orgânica" (selected) and "Hierárquico", and restrained buttons "Filtros" and "Exportar".
The main body is an expansive white softly rounded network-canvas card, next to a well-spaced white legend/detail column to the right. On the canvas show a legible, organically arranged seven-person network with fine curved gray/teal directed edges, small clear link labels "Família" or "Profissional", and generous whitespace. A visibly selected central profile node is the same fictional "Rafael Costa", realistic male headshot age 38, dark hair and neutral expression, thin teal inner circular border plus slightly transparent flowing scarlet glowing outer halo suggesting a pulse. Below his name show a compact segmented vitality bar with 2 of 20 segments filled red and the text "10%". Other fictional nodes labeled "Marina Alves", "Lucas Lima", "Camila Rocha", "Pedro Santos", "Ana Souza", "Bruno Melo" have restrained orange, yellow or green translucent outer auras and vitality bars that visually reflect varied vitality levels. Two nodes near Rafael have family/professional links, and their neighbors illustrate second-degree social connections. Maintain readable labels without overlap. On-canvas bottom-left compact zoom controls and a tiny "7 pessoas · 8 vínculos" counter. Display exactly 7 nodes and 8 edges if using that counter.
Right column upper legend card: heading exactly "Behavioral Risk Auras". Four spacious rows, each with a clear ring visual sample and exact text: a softly pulsing-looking scarlet ring labeled "Critical"; orange ring "Watch"; yellow ring "Elevated"; green ring "Stable". Optional tiny text "Pulsing Red" beside the first ring, without replacing "Critical". The colors describe network status, not character labels.
Immediately below a thin divider, heading exactly "Vitality Bar". Show a fully green filled 20-segment sample with the exact caption "Full Green = Healthy", and a 20-segment completely empty sample with pale/red outlines, no filled segments, caption exactly "Empty Red = Impacted". These two legend samples must be visually distinct from the 10%-red selected-node bar.
Right column lower card: "Perfil selecionado", "Rafael Costa", clearly labeled "Psychosocial Vitality HP", a 20-segment bar with exactly two red segments, "10%", and a button exactly "Abrir perfil". Include a small settings-link row "Ajustar parâmetros" with gear icon and an unobtrusive admin lock icon. Treat the legend as essential and make its exact text readable; avoid a giant legend that crowds the graph. This is one cohesive screenshot, not multiple images within one image.
```
