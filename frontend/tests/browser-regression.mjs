import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const distRoot = fileURLToPath(new URL('../dist/', import.meta.url));
const cacheRoot = fileURLToPath(new URL('../../.cache/', import.meta.url));
await mkdir(cacheRoot, { recursive: true });
const server = createServer(async (req, res) => {
  const path = req.url.split('?')[0];
  const file = join(distRoot, path.startsWith('/assets/') ? path : 'index.html');
  try {
    res.setHeader('content-type', file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : 'text/html');
    res.end(await readFile(file));
  } catch { res.statusCode = 404; res.end(); }
});
await new Promise(resolve => server.listen(4179, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium-browser', args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(30_000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));
const hpConfig = { versao: 1, ativo: true, toxicidade_min: 0.1, toxicidade_max: 0.5, hp_base: 1, hp_min: 0.05, fator_segundo_grau: 0.2, peso_padrao: 0.5, pesos_vinculo: { família: 1, profissional: 0.5 }, faixas_aura: [{ min: 0, nome: 'Estável', cor_hex: '#22C55E', pulsante: false }, { min: 0.1, nome: 'Elevado', cor_hex: '#EAB308', pulsante: false }, { min: 0.25, nome: 'Observação', cor_hex: '#F97316', pulsante: false }, { min: 0.4, nome: 'Crítico', cor_hex: '#EF4444', pulsante: true }], faixas_vitalidade: [{ min: 0, nome: 'Impactada', cor_hex: '#EF4444', pulsante: false }, { min: 0.75, nome: 'Preservada', cor_hex: '#22C55E', pulsante: false }] };
const indicators = { hp: 1, hp_percentual: 100, hp_base: 1, hp_min: 0.05, penalidade_propria: 0, penalidade_direta: 0, penalidade_residual: 0, aura_nome: 'Crítico', aura_cor_hex: '#EF4444', aura_pulsante: true, vitalidade_nome: 'Preservada', vitalidade_cor_hex: '#22C55E', calculo_ativo: true, configuracao_versao: 1, contribuicoes: [] };
let currentHp = 1;
const metrics = () => ({ ...indicators, hp: currentHp, hp_percentual: currentHp * 100, penalidade_direta: 1 - currentHp, vitalidade_nome: currentHp < 0.25 ? 'Impactada' : 'Preservada', vitalidade_cor_hex: currentHp < 0.25 ? '#EF4444' : '#22C55E' });
const base = { classificacao_risco: 'MANIPULATIVO', toxicidade: 0.4, psicossocial: indicators, categoria_id: 1, nome_categoria: 'Amigos', cor_hex: '#112233', tem_foto: false, pessoa_juridica: false, data_cadastro: '2026-09-14', descricao: '# Perfil\n\n**Importante**', contatos: [], etiquetas: '', favorito: false };
const people = [{ ...base, id: 1, nome: 'Ana' }, { ...base, id: 2, nome: 'Zeca' }, { ...base, id: 3, nome: 'Empresa', pessoa_juridica: true }];
const attachments = [1, 2].map(id => ({ id, pessoa_id: 1, nome_arquivo: `foto${id}.png`, mime_type: 'image/png', tamanho_bytes: 100, data_upload: '2026-09-14', url_stream: `/api/dossie/anexos/${id}/stream`, url_download: `/api/dossie/anexos/${id}/download` }));
const notes = {};
const events = [];
const parameters = [];
let failEvent = false;
let attachmentPosts = 0;
let photoPosts = 0, personPosts = 0, failPhoto = false;
let savedSnapshot = null;
let nextSavedSnapshot = null;
const riskHistory = {};
let failPreview = false;
let previewPosts = 0;
let logoutPosts = 0;
let groupFixture = false;
const deletedRelationshipIds = new Set();
const riskRecord = person => ({ classificacao_risco: person.classificacao_risco, toxicidade: person.toxicidade, justificativa: person.risco_justificativa ?? '', revisado_em: person.risco_revisado_em || null });
await page.context().route('**/api/**', async route => {
  const req = route.request(), path = new URL(req.url()).pathname;
  let data = [];
  if (path === '/api/auth/logout') { logoutPosts++; data = {}; }
  else if (path.endsWith('/sessao')) data = { usuario: { id: 1, login: 'teste', perfil: 'admin' } };
  else if (path.endsWith('/categorias')) data = [{ id: 1, nome_categoria: 'Amigos', cor_hex: '#112233' }];
  else if (path === '/api/configuracoes/backups/configuracao') data = { ativo: false, horario: '03:00', manter_diarios: 7, manter_semanais: 4, manter_mensais: 12, max_upload_bytes: 1000000 };
  else if (path.endsWith('/admin/diagnostico-armazenamento')) data = { banco_bytes: 0, dossie_bytes: 0, vinculos_bytes: 0, tarefas_bytes: 0, midia_total_bytes: 0, anexos_total: 0, pessoas_total: people.length, limite_usuario_tarefas_bytes: 1, max_arquivo_bytes: 1, usuarios: [] };
  else if (path === '/api/configuracoes/hp-psicossocial') { if (req.method() === 'PUT') { Object.assign(hpConfig, req.postDataJSON(), { versao: hpConfig.versao + 1 }); } data = hpConfig; }
  else if (path.startsWith('/api/produtividade/grafo/posicoes/')) data = people.map(p => ({ pessoa_id: p.id, x: p.id * 10000, y: -p.id * 10000 }));
  else if (path === '/api/vinculos/grafo') data = { hp_configuracao: hpConfig, nodes: [...people.map(p => ({ ...metrics(), ...(savedSnapshot?.[p.id] ?? {}), classificacao_risco: p.classificacao_risco, toxicidade: p.toxicidade, id: p.id, label: p.nome, color: p.cor_hex, categoria: groupFixture && p.id === 3 ? 'Trabalho' : p.nome_categoria, pessoa_juridica: p.pessoa_juridica, contatos: [] })), ...(groupFixture ? [{ ...metrics(), id: 99, label: 'Sem vínculos', color: '#86A6A3', categoria: null, pessoa_juridica: false, contatos: [] }] : [])], edges: [{ id: 1, source: 1, target: 2, label: 'Amizade', descricao: '# Contexto', data_criacao: '2026-09-14' }, { id: 2, source: 2, target: 3, label: 'Profissional', descricao: null, data_criacao: '2026-09-14' }].filter(edge => !deletedRelationshipIds.has(edge.id)) };
  else if (path === '/api/vinculos/lixeira') data = [...deletedRelationshipIds].map(id => ({ id, tipo_vinculo: 'Amizade', pessoa_origem_id: 1, pessoa_destino_id: 2, origem_nome: 'Ana', destino_nome: 'Zeca', excluido_em: '2026-09-23 21:00:00' }));
  else if (/^\/api\/vinculos\/lixeira\/\d+\/restaurar$/.test(path) && req.method() === 'POST') { deletedRelationshipIds.delete(Number(path.split('/')[4])); return route.fulfill({ status: 204 }); }
  else if (/^\/api\/vinculos\/lixeira\/\d+$/.test(path) && req.method() === 'DELETE') { deletedRelationshipIds.delete(Number(path.split('/')[4])); return route.fulfill({ status: 204 }); }
  else if (path === '/api/vinculos') data = [{ id: 1, pessoa_origem_id: 1, pessoa_destino_id: 2, tipo_vinculo: 'Amizade', descricao: '# Contexto' }].filter(relationship => !deletedRelationshipIds.has(relationship.id));
  else if (path === '/api/pessoas/risco/previa') {
    previewPosts++;
    if (failPreview) return route.fulfill({ status: 503, json: { erro: 'Prévia indisponível' } });
    const proposed = req.postDataJSON();
    const hp = 1 - proposed.toxicidade;
    data = { pessoa: { ...indicators, hp, hp_percentual: hp * 100, penalidade_propria: proposed.toxicidade }, versao_configuracao: 1, alteracoes: [{ pessoa_id: proposed.pessoa_id ?? 4, nome: proposed.nome || 'Nova pessoa', hp_antes: proposed.pessoa_id ? 1 : null, hp_depois: hp, aura_antes: 'Crítico', aura_depois: 'Observação' }, { pessoa_id: 3, nome: 'Empresa', hp_antes: 1, hp_depois: 0.85, aura_antes: 'Estável', aura_depois: 'Estável' }] };
  }
  else if (/^\/api\/pessoas\/\d+\/risco\/historico$/.test(path)) data = riskHistory[Number(path.split('/')[3])] ?? [];
  else if (path === '/api/pessoas') {
    if (req.method() === 'POST') { personPosts++; data = { ...base, ...req.postDataJSON(), id: 4 }; }
    else data = people;
  } else if (/^\/api\/pessoas\/\d+$/.test(path)) {
    const person = people.find(p => p.id === Number(path.split('/').at(-1))) ?? { ...people[0] };
    if (req.method() === 'PUT') {
      const anterior = riskRecord(person);
      Object.assign(person, req.postDataJSON());
      const novo = riskRecord(person);
      if (JSON.stringify(anterior) !== JSON.stringify(novo)) (riskHistory[person.id] ??= []).unshift({ id: (riskHistory[person.id]?.length ?? 0) + 1, autor_login: 'teste', registrado_em: '2026-09-17 12:00:00', anterior, novo });
      if (nextSavedSnapshot) savedSnapshot = nextSavedSnapshot;
    }
    data = { ...person, risco_registro: riskRecord(person), psicossocial: { ...metrics(), ...(savedSnapshot?.[person.id] ?? {}) } };
  }
  else if (path === '/api/calendario/tarefas' && req.method() === 'POST') {
    if (failEvent) return route.fulfill({ status: 503, json: { erro: 'Agenda indisponível' } });
    const event = { ...req.postDataJSON(), id: events.length + 1, pessoas: [], anexos: [] }; events.push(event); data = event;
  }
  else if (/^\/api\/vinculos\/\d+$/.test(path) && req.method() === 'PUT') data = { ...req.postDataJSON(), id: 1 };
  else if (/^\/api\/vinculos\/\d+$/.test(path) && req.method() === 'DELETE') {
    deletedRelationshipIds.add(Number(path.split('/').at(-1)));
    return route.fulfill({ status: 204 });
  }
  else if (path.includes('/osint/parametros/')) {
    if (req.method() === 'POST') parameters.push({ ...req.postDataJSON(), id: parameters.length + 1, pessoa_id: 1 });
    data = req.method() === 'POST' ? parameters.at(-1) : parameters;
  }
  else if (path.includes('/osint/historico/')) data = { itens: [], total: 0, pagina: 1, por_pagina: 10, total_paginas: 0 };
  else if (path.endsWith('/anexos')) {
    if (req.method() === 'POST') { attachmentPosts++; const id = 10 + attachmentPosts; data = { ...attachments[0], id, nome_arquivo: 'novo.txt', url_stream: `/api/dossie/anexos/${id}/stream`, url_download: `/api/dossie/anexos/${id}/download` }; }
    else data = attachments;
  }
  else if (path.endsWith('/notas')) {
    if (req.method() === 'PUT') notes[path] = req.postDataJSON().notas;
    data = { notas: notes[path] || '', pessoas_ids: [1] };
  } else if (path.endsWith('/foto') && req.method() === 'POST') {
    photoPosts++;
    assert.ok(req.headers()['content-type'].startsWith('multipart/form-data;'));
    if (failPhoto) return route.abort('failed');
    data = { mensagem: 'foto atualizada' };
  } else if (path.endsWith('/stream')) return route.fulfill({ contentType: 'image/png', body: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aJ1sAAAAASUVORK5CYII=', 'base64') });
  await route.fulfill({ json: data });
});
const expectGraphFramed = async () => {
  await page.waitForFunction(() => {
    const container = document.querySelector('[role=application]');
    const cy = container?._cyreg?.cy;
    if (!cy || cy.nodes().empty()) return false;
    const bounds = cy.elements().renderedBoundingBox();
    const rect = container.getBoundingClientRect();
    return bounds.x1 >= 0 && bounds.y1 >= 0 && bounds.x2 <= cy.width() && bounds.y2 <= cy.height()
      && [...document.querySelectorAll('[data-person-id], [data-graph-category]')].every(bar => {
        const box = bar.getBoundingClientRect();
        return box.left >= rect.left && box.top >= rect.top && box.right <= rect.right && box.bottom <= rect.bottom;
      });
  });
};
try {
  await page.goto('http://127.0.0.1:4179/pessoas');
  await page.getByRole('heading', { name: /Pessoas físicas · Amigos/ }).waitFor();
  await page.getByLabel('Tipo de pessoa').selectOption('juridica');
  await page.reload();
  await page.getByRole('heading', { name: 'Empresa', exact: true }).waitFor();
  assert.equal(await page.getByLabel('Tipo de pessoa').inputValue(), 'juridica');
  assert.equal(await page.getByRole('heading', { name: 'Ana', exact: true }).count(), 0);
  await page.getByRole('button', { name: 'Limpar filtros' }).click();
  await page.goto('http://127.0.0.1:4179/pessoas/1?aba=dossie');
  await page.getByAltText('foto1.png').click();
  await page.getByRole('button', { name: 'Próxima imagem' }).waitFor();
  for (const viewport of [{ width: 1440, height: 1000 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport);
    await page.waitForFunction(() => {
      const rect = document.querySelector('[role="dialog"]')?.getBoundingClientRect();
      return rect && rect.top > 40 && rect.height <= innerHeight * 0.85 + 1
        && Math.abs(rect.top + rect.height / 2 - innerHeight / 2) < 1
        && Math.abs(rect.left + rect.width / 2 - innerWidth / 2) < 1;
    });
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.keyboard.press('ArrowRight');
  await page.getByRole('heading', { name: 'foto2.png', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Próxima imagem' }).click();
  await page.getByRole('heading', { name: 'foto1.png', exact: true }).waitFor();
  await page.locator('summary').filter({ hasText: 'Notas do arquivo' }).click();
  await page.getByRole('button', { name: 'Editar notas' }).click();
  await page.getByLabel('Notas do arquivo', { exact: true }).fill('**Nota de teste**');
  await page.keyboard.press('ArrowRight');
  await page.getByRole('heading', { name: 'foto1.png', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Próxima imagem' }).click();
  await page.getByRole('button', { name: 'Imagem anterior' }).click();
  await page.locator('summary').filter({ hasText: 'Notas do arquivo' }).click();
  assert.equal(await page.getByLabel('Notas do arquivo', { exact: true }).inputValue(), '**Nota de teste**');
  await page.getByRole('button', { name: 'Salvar notas' }).click();
  await page.getByText('Notas do arquivo salvas', { exact: true }).waitFor();
  assert.equal(notes['/api/dossie/anexos/1/notas'], '**Nota de teste**');
  const noteDialog = page.getByRole('dialog', { name: 'foto1.png', exact: true });
  assert.equal(events.length, 0);
  await noteDialog.getByLabel('Criar evento na agenda (opcional)').check();
  await noteDialog.getByLabel('Dia inteiro', { exact: true }).check();
  await noteDialog.getByLabel('Início', { exact: true }).fill('2026-10-20');
  await noteDialog.getByRole('button', { name: 'Salvar notas e criar evento' }).click();
  await noteDialog.getByRole('link', { name: 'Abrir evento criado' }).waitFor();
  assert.deepEqual(events[0].pessoas_ids, [1]);
  assert.ok(events[0].descricao.includes('/api/dossie/anexos/1/stream'));
  assert.equal(events[0].inicio_em, '2026-10-20T00:00:00.000Z');
  await page.keyboard.press('Escape');
  await page.goto('http://127.0.0.1:4179/pessoas/nova');
  await page.getByLabel('Nome completo').fill('Nova pessoa');
  await page.getByLabel('Classificação de risco', { exact: true }).selectOption('MANIPULATIVO');
  await page.getByLabel('Intensidade do risco cadastrado', { exact: true }).fill('0.4');
  await page.getByLabel('Descrição', { exact: true }).fill('# Título\n\n**Texto**\n\n| A | B |\n| - | - |\n| 1 | 2 |');
  await page.getByRole('button', { name: 'Visualizar formatação' }).click();
  await page.getByRole('heading', { name: 'Título', exact: true }).waitFor();
  assert.equal(await page.locator('.markdown-text table').count(), 1);
  const image = await page.evaluate(() => { const c = document.createElement('canvas'); c.width = 2400; c.height = 1800; c.getContext('2d').fillRect(0, 0, 2400, 1800); return c.toDataURL('image/png').split(',')[1]; });
  await page.locator('input[type=file]').setInputFiles({ name: 'foto.png', mimeType: 'image/png', buffer: Buffer.from(image, 'base64') });
  await page.getByLabel('Criar evento na agenda (opcional)').check();
  await page.getByLabel('Início', { exact: true }).fill('2026-10-21T09:30');
  failPhoto = true;
  await page.getByRole('button', { name: 'Cadastrar pessoa', exact: true }).click();
  await page.getByText(/Dados da pessoa salvos, mas a foto/).waitFor();
  failPhoto = false;
  await page.getByRole('button', { name: 'Cadastrar pessoa', exact: true }).click();
  await page.waitForURL('**/pessoas/4');
  assert.equal(personPosts, 1);
  assert.equal(photoPosts, 2);
  assert.equal(events.length, 2);
  assert.deepEqual(events[1].pessoas_ids, [4]);
  assert.ok(events[1].descricao.includes('/pessoas/4'));
  await page.goto('http://127.0.0.1:4179/grafo?busca=Ana');
  await page.waitForFunction(() => document.querySelector('[role=application]')?._cyreg?.cy?.nodes(':selected').length === 1);
  await expectGraphFramed();
  await page.evaluate(() => {
    const cy = document.querySelector('[role=application]')._cyreg.cy;
    cy.nodes().positions(() => ({ x: 10000, y: -10000 }));
    cy.zoom(3); cy.pan({ x: 10000, y: -10000 });
  });
  await page.getByRole('button', { name: 'Organizar grafo', exact: true }).click();
  await expectGraphFramed();
  assert.ok(await page.evaluate(() => {
    const nodes = document.querySelector('[role=application]')._cyreg.cy.nodes();
    const a = nodes[0].position(), b = nodes[1].position();
    return Math.hypot(a.x - b.x, a.y - b.y) > 78;
  }));
  await page.reload();
  await page.waitForFunction(() => document.querySelector('[role=application]')?._cyreg?.cy?.nodes(':selected').length === 1);
  await expectGraphFramed();
  assert.equal(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.nodes(':selected').style('border-color')), 'rgb(17,34,51)');
  await page.getByLabel('Legenda psicossocial', { exact: true }).waitFor();
  assert.equal(await page.getByLabel('Perfil selecionado').getByRole('progressbar').getAttribute('aria-valuenow'), '100');
  await page.evaluate(() => { const cy = document.querySelector('[role=application]')._cyreg.cy; cy.zoom(1.2); cy.pan({ x: 120, y: 80 }); window.__hpCy = cy; window.__hpViewport = { zoom: cy.zoom(), pan: cy.pan(), position: cy.nodes(':selected').position() }; });
  currentHp = 0.1;
  await page.evaluate(() => window.dispatchEvent(new Event('agendarx:psychosocial-updated')));
  await page.waitForFunction(() => document.querySelector('[aria-label="Perfil selecionado"] [role="progressbar"]')?.getAttribute('aria-valuenow') === '10');
  assert.equal(await page.getByLabel('Perfil selecionado').locator('.vitality-segment > span').evaluateAll(items => items.filter(item => item.style.width === '100%').length), 2);
  assert.equal(await page.getByLabel('Perfil selecionado').locator('.vitality-segment').count(), 20);
  await page.waitForFunction(() => document.querySelector('[data-person-id="1"] [role="progressbar"]')?.getAttribute('aria-valuenow') === '10');
  for (const [hp, color] of [[1, 'rgb(34, 197, 94)'], [0.7, 'rgb(34, 197, 94)'], [0.699, 'rgb(234, 179, 8)'], [0.4, 'rgb(234, 179, 8)'], [0.399, 'rgb(239, 68, 68)'], [0.1, 'rgb(239, 68, 68)']]) {
    currentHp = hp;
    await page.evaluate(() => window.dispatchEvent(new Event('agendarx:psychosocial-updated')));
    await page.waitForFunction(value => document.querySelector('[data-person-id="1"] [role="progressbar"]')?.getAttribute('aria-valuenow') === String(value * 100), hp);
    const bar = page.locator('[data-person-id="1"] [role="progressbar"]');
    assert.equal(await bar.evaluate(el => getComputedStyle(el).width), '56px');
    assert.equal(await bar.locator('div').evaluate(el => el.style.backgroundColor), color);
    assert.ok(Math.abs(await bar.locator('div').evaluate(el => parseFloat(el.style.width)) - hp * 100) < 1e-9);
  }
  await page.evaluate(() => { const cy = document.querySelector('[role=application]')._cyreg.cy; if (cy !== window.__hpCy || cy.zoom() !== window.__hpViewport.zoom || JSON.stringify(cy.pan()) !== JSON.stringify(window.__hpViewport.pan) || JSON.stringify(cy.nodes(':selected').position()) !== JSON.stringify(window.__hpViewport.position)) throw new Error('Atualização de HP alterou o mapa'); });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.nodes().first().style('underlay-opacity') === '0.16');
  assert.equal(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.nodes().first().style('underlay-opacity')), '0.16');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.evaluate(() => { document.querySelector('[role=application]')._cyreg.cy.$id('node-1').emit('tap'); });
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.nodes().length === 3);
  assert.equal(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.$id('node-3').hasClass('is-secondary')), true);
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.$id('edge-1').style('line-color') === 'rgb(15,118,110)');
  assert.equal(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.$id('edge-2').style('line-style')), 'dashed');
  await page.getByText('3 pessoas · 2 vínculos visíveis', { exact: true }).waitFor();
  await expectGraphFramed();
  await page.getByRole('button', { name: 'UML', exact: true }).click();
  await expectGraphFramed();
  await page.evaluate(() => {
    const cy = document.querySelector('[role=application]')._cyreg.cy;
    cy.nodes().positions((node, index) => ({ x: index * 10000, y: index * 10000 }));
    window.__graphPositions = cy.nodes().map(node => ({ ...node.position() }));
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await expectGraphFramed();
  assert.ok(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.zoom() < 0.2));
  assert.deepEqual(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.nodes().map(node => node.position())), await page.evaluate(() => window.__graphPositions));
  await page.getByRole('button', { name: 'Organizar grafo', exact: true }).click();
  await expectGraphFramed();
  await page.getByRole('button', { name: 'Teia', exact: true }).click();
  await expectGraphFramed();
  await page.setViewportSize({ width: 1440, height: 1000 });
  await expectGraphFramed();
  await page.evaluate(() => {
    const cy = document.querySelector('[role=application]')._cyreg.cy;
    cy.$id('node-2').position({ x: 4500, y: -3500 });
    window.__fitPositions = cy.nodes().map(node => ({ ...node.position() }));
    cy.zoom(3); cy.pan({ x: 10000, y: 10000 });
  });
  await page.getByRole('button', { name: 'Enquadrar tudo', exact: true }).click();
  await expectGraphFramed();
  assert.deepEqual(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.nodes().map(node => node.position())), await page.evaluate(() => window.__fitPositions));
  groupFixture = true;
  await page.evaluate(() => window.dispatchEvent(new Event('agendarx:psychosocial-updated')));
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.nodes().length === 4);
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.$id('node-99').style('opacity') === '0.2');
  await page.waitForFunction(() => document.querySelector('[data-person-id="99"]')?.style.opacity === '0.2');
  await page.getByRole('button', { name: 'Limpar foco', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.nodes(':selected').empty()
    && document.querySelector('[role=application]')._cyreg.cy.elements('.is-dimmed').empty());
  await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.$id('node-1').emit('tap'));
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.$id('node-99').hasClass('is-dimmed'));
  await page.getByLabel('Mostrar apenas conexões', { exact: true }).check();
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.nodes().length === 3);
  await page.getByLabel('Mostrar apenas conexões', { exact: true }).uncheck();
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.nodes().length === 4);
  await page.getByLabel('Agrupar por categoria', { exact: true }).check();
  await page.waitForFunction(() => document.querySelectorAll('[data-graph-category]').length === 3);
  assert.deepEqual(await page.locator('[data-graph-category]').allTextContents(), ['Amigos', 'Sem categoria', 'Trabalho']);
  await expectGraphFramed();
  assert.ok(await page.evaluate(() => {
    const cy = document.querySelector('[role=application]')._cyreg.cy;
    const groups = [cy.nodes('#node-1, #node-2'), cy.$id('node-3'), cy.$id('node-99')].map(nodes => nodes.boundingBox());
    return groups.every((a, index) => groups.slice(index + 1).every(b => a.x2 < b.x1 || b.x2 < a.x1 || a.y2 < b.y1 || b.y2 < a.y1));
  }));
  for (const viewport of [{ width: 1440, height: 1000 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport);
    await page.getByRole('button', { name: 'Tela cheia', exact: true }).click();
    await page.waitForFunction(() => {
      const rect = document.querySelector('[aria-label="Grafo em tela cheia"]')?.getBoundingClientRect();
      return rect && rect.top === 0 && rect.left === 0 && Math.abs(rect.width - innerWidth) < 1 && Math.abs(rect.height - innerHeight) < 1;
    });
    await expectGraphFramed();
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Tela cheia', exact: true }).waitFor();
    assert.equal(await page.evaluate(() => document.documentElement.style.overflow), '');
    await expectGraphFramed();
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.getByRole('button', { name: 'UML', exact: true }).click();
  await expectGraphFramed();
  await page.getByRole('button', { name: 'Tela cheia', exact: true }).click();
  await page.getByRole('button', { name: 'Sair da tela cheia', exact: true }).click();
  await page.getByLabel('Agrupar por categoria', { exact: true }).uncheck();
  await page.waitForFunction(() => document.querySelectorAll('[data-graph-category]').length === 0);
  groupFixture = false;
  await page.evaluate(() => window.dispatchEvent(new Event('agendarx:psychosocial-updated')));
  await page.waitForFunction(() => document.querySelector('[role=application]')._cyreg.cy.nodes().length === 3);
  await page.getByRole('button', { name: 'Teia', exact: true }).click();
  await expectGraphFramed();
  if (process.env.BROWSER_SCREENSHOTS === '1') await page.screenshot({ path: join(cacheRoot, 'hp-grafo.png'), fullPage: false, timeout: 10000 });

  await page.evaluate(() => { document.querySelector('[role=application]')._cyreg.cy.edges().first().emit('tap'); });
  await page.getByRole('dialog', { name: 'Detalhes do vínculo' }).waitFor();
  await page.getByAltText('foto1.png').click();
  await page.getByRole('button', { name: 'Próxima imagem' }).waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('dialog', { name: 'Detalhes do vínculo' }).waitFor();
  assert.equal(await page.getByRole('dialog', { name: 'foto1.png', exact: true }).count(), 0);
  const drawer = page.getByRole('dialog', { name: 'Detalhes do vínculo' });
  await drawer.getByRole('button', { name: 'Editar', exact: true }).click();
  await drawer.getByLabel('Criar evento na agenda (opcional)').check();
  await drawer.getByLabel('Início', { exact: true }).fill('2026-10-22T10:00');
  failEvent = true;
  await drawer.getByRole('button', { name: 'Salvar relação' }).click();
  await page.getByText(/Dados salvos, mas o evento não foi criado/).waitFor();
  assert.equal(events.length, 2);
  failEvent = false;
  await drawer.getByRole('button', { name: 'Salvar relação' }).click();
  await drawer.getByRole('button', { name: 'Editar', exact: true }).waitFor();
  assert.equal(events.length, 3);
  assert.deepEqual(events[2].pessoas_ids, [1, 2]);
  await page.keyboard.press('Escape');
  await page.getByRole('dialog', { name: 'Detalhes do vínculo' }).waitFor({ state: 'hidden' });
  await page.goto('http://127.0.0.1:4179/calendario');
  await page.getByRole('heading', { name: 'Calendário', exact: true }).waitFor();
  await page.goto('http://127.0.0.1:4179/pessoas/1?aba=osint');
  await page.getByLabel('Fonte de pesquisa').selectOption('DATAJUD');
  assert.equal(await page.getByLabel('Tipo', { exact: true }).inputValue(), 'PROCESSO');
  assert.equal(await page.getByLabel('Tipo', { exact: true }).locator('option').count(), 1);
  await page.getByLabel('Valor pesquisado').fill('0000832-35.2018.4.01.3202');
  await page.getByRole('button', { name: 'Adicionar parâmetro' }).click();
  await page.getByText('0000832-35.2018.4.01.3202', { exact: true }).waitFor();
  assert.equal(parameters[0].provider, 'DATAJUD');
  assert.equal(parameters[0].tipo, 'PROCESSO');
  await page.goto('http://127.0.0.1:4179/pessoas/1?aba=dossie');
  await page.getByRole('heading', { name: 'Adicionar ao dossiê' }).waitFor();
  await page.getByLabel('Criar evento na agenda (opcional)').check();
  await page.locator('input[type=file]').setInputFiles({ name: 'novo.txt', mimeType: 'text/plain', buffer: Buffer.from('Arquivo de teste') });
  await page.getByRole('button', { name: 'Reenviar falhas (1)' }).waitFor();
  assert.equal(attachmentPosts, 0);
  await page.getByLabel('Início', { exact: true }).fill('2026-10-23T10:00');
  failEvent = true;
  await page.getByRole('button', { name: 'Reenviar falhas (1)' }).click();
  await page.getByRole('button', { name: 'Tentar criar evento dos arquivos salvos' }).waitFor();
  assert.equal(attachmentPosts, 1);
  assert.equal(events.length, 3);
  failEvent = false;
  await page.getByRole('button', { name: 'Tentar criar evento dos arquivos salvos' }).click();
  await page.getByText('Evento vinculado aos arquivos criado', { exact: true }).waitFor();
  assert.equal(attachmentPosts, 1);
  assert.equal(events.length, 4);
  assert.deepEqual(events[3].pessoas_ids, [1]);
  assert.ok(events[3].descricao.includes('/api/dossie/anexos/11/stream'));
  await page.goto('http://127.0.0.1:4179/configuracoes');
  try {
    await page.getByRole('heading', { name: 'Configurações', exact: true }).waitFor({ timeout: 10_000 });
  } catch (error) {
    const body = (await page.locator('body').innerText()).slice(0, 1_000);
    throw new Error(`Configurações não carregou em ${page.url()}. Erros: ${errors.join(' | ') || 'nenhum'}. Conteúdo: ${body}`, { cause: error });
  }
  await page.goto('http://127.0.0.1:4179/pessoas/1?aba=dossie');
  await page.getByLabel('Resumo psicossocial', { exact: true }).waitFor();
  if (process.env.BROWSER_SCREENSHOTS === '1') await page.screenshot({ path: join(cacheRoot, 'hp-perfil.png'), fullPage: false, timeout: 10000 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  assert.equal(await page.locator('.risk-aura').evaluate(element => getComputedStyle(element, '::before').animationName), 'none');
  await page.getByRole('button', { name: /Vitalidade psicossocial/ }).click();
  await page.getByRole('region', { name: 'Composição do impacto psicossocial' }).waitFor();
  await page.getByRole('region', { name: 'Composição do impacto psicossocial' }).getByLabel('Cálculo do HP').waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('region', { name: 'Composição do impacto psicossocial' }).waitFor({ state: 'hidden' });
  await page.goto('http://127.0.0.1:4179/configuracoes');
  await page.getByRole('heading', { name: 'HP psicossocial', exact: true }).waitFor();
  await page.getByLabel('Fator de 2º grau', { exact: true }).fill('0.4');
  await page.getByRole('button', { name: 'Salvar parâmetros psicossociais', exact: true }).click();
  await page.getByText('Parâmetros psicossociais atualizados', { exact: true }).waitFor();
  assert.equal(hpConfig.fator_segundo_grau, 0.4);
  // Saving different profiles in another tab must refresh each graph node independently.
  await page.goto('http://127.0.0.1:4179/grafo?busca=Ana');
  await page.waitForFunction(() => document.querySelector('[role=application]')?._cyreg?.cy?.nodes().length === 3);
  const editor = await page.context().newPage();
  for (const [id, t, values] of [[1, '0.3', [0.7, 0.45, 0.37]], [2, '0.2', [0.7, 0.65, 0.47]]]) {
    nextSavedSnapshot = Object.fromEntries(values.map((hp, index) => {
      const propria = index + 1 === id ? Number(t) : people[index].toxicidade;
      const color = propria >= 0.4 ? '#EF4444' : propria >= 0.25 ? '#F97316' : '#EAB308';
      const residual = index === 2 ? 0.03 : 0;
      return [index + 1, { hp, hp_percentual: hp * 100, penalidade_propria: propria, penalidade_direta: Math.max(0, 1 - propria - hp - residual), penalidade_residual: residual, aura_nome: propria >= 0.4 ? 'Crítico' : propria >= 0.25 ? 'Observação' : 'Elevado', aura_cor_hex: color, aura_pulsante: propria >= 0.4 }];
    }));
    await editor.goto(`http://127.0.0.1:4179/pessoas/${id}/editar`);
    await editor.getByLabel('Classificação de risco', { exact: true }).selectOption('MANIPULATIVO');
    await editor.getByLabel('Intensidade do risco cadastrado', { exact: true }).fill(t);
    await editor.getByLabel('Justificativa da classificação', { exact: true }).fill(`Observações da pessoa ${id}`);
    await editor.getByLabel('Data da revisão', { exact: true }).fill('2026-09-17');
    await editor.getByRole('progressbar', { name: 'HP previsto desta pessoa', exact: true }).waitFor();
    await editor.waitForFunction(hp => document.querySelector('[aria-label="HP previsto desta pessoa"]')?.getAttribute('aria-valuenow') === String(hp * 100), 1 - Number(t));
    await editor.getByLabel('Prévia dos impactos').getByRole('cell', { name: 'Empresa', exact: true }).waitFor();
    if (id === 1 && process.env.BROWSER_SCREENSHOTS === '1') await editor.screenshot({ path: join(cacheRoot, 'risco-previa.png'), fullPage: true, timeout: 10000 });
    assert.equal(people.find(p => p.id === id).toxicidade, 0.4);
    await editor.getByRole('button', { name: 'Salvar alterações', exact: true }).click();
    await editor.waitForURL(`**/pessoas/${id}`);
    assert.equal(people.find(p => p.id === id).toxicidade, Number(t));
    await editor.locator('summary').filter({ hasText: 'Histórico de risco e revisões' }).click();
    await editor.getByText(`Observações da pessoa ${id}`, { exact: true }).last().waitFor();
    assert.equal(riskHistory[id][0].autor_login, 'teste');
    assert.equal(riskHistory[id][0].anterior.toxicidade, 0.4);
    assert.equal(riskHistory[id][0].novo.toxicidade, Number(t));
    await page.waitForFunction(({ id, t, values }) => {
      const cy = document.querySelector('[role=application]')?._cyreg?.cy;
      return cy?.$id(`node-${id}`).data('profile').toxicidade === Number(t)
        && values.every((hp, i) => document.querySelector(`[data-person-id="${i + 1}"] [role="progressbar"]`)?.getAttribute('aria-valuenow') === String(hp * 100));
    }, { id, t, values });
    await editor.goto(`http://127.0.0.1:4179/pessoas/${id}/editar`);
    await editor.getByLabel('Intensidade do risco cadastrado', { exact: true }).waitFor();
    assert.equal(await editor.getByLabel('Intensidade do risco cadastrado', { exact: true }).inputValue(), t);
    assert.equal(await editor.getByLabel('Justificativa da classificação', { exact: true }).inputValue(), `Observações da pessoa ${id}`);
    assert.equal(await editor.getByLabel('Data da revisão', { exact: true }).inputValue(), '2026-09-17');
  }
  if (process.env.BROWSER_SCREENSHOTS === '1') await page.screenshot({ path: join(cacheRoot, 'hp-grafo.png'), fullPage: false, timeout: 10000 });
  failPreview = true;
  await editor.getByLabel('Intensidade do risco cadastrado', { exact: true }).fill('0.35');
  await editor.getByRole('alert').filter({ hasText: 'Não foi possível calcular a prévia' }).waitFor();
  failPreview = false;
  await editor.getByRole('button', { name: 'Tentar calcular novamente' }).click();
  await editor.waitForFunction(() => document.querySelector('[aria-label="HP previsto desta pessoa"]')?.getAttribute('aria-valuenow') === '65');
  assert.equal(people.find(p => p.id === 2).toxicidade, 0.2);
  assert.ok(previewPosts > 0);
  await editor.route('**/api/configuracoes/hp-psicossocial', route => route.fulfill({ status: 503, json: { erro: 'Indisponível' } }));
  await editor.goto('http://127.0.0.1:4179/pessoas/2/editar');
  await editor.getByRole('alert').filter({ hasText: 'Não foi possível carregar o cadastro completo' }).waitFor();
  assert.equal(await editor.getByRole('button', { name: 'Salvar alterações', exact: true }).isDisabled(), true);
  await editor.close();
  await page.goto('http://127.0.0.1:4179/pessoas');
  let panel = page.getByRole('link', { name: 'Abrir configurações' }).locator('..');
  await panel.click({ position: { x: 3, y: 30 } });
  await page.waitForURL('**/configuracoes');
  await page.getByRole('heading', { name: 'Configurações', exact: true }).waitFor();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('http://127.0.0.1:4179/pessoas');
  await page.getByRole('button', { name: 'Abrir menu' }).click();
  panel = page.getByRole('link', { name: 'Abrir configurações' }).filter({ visible: true }).locator('..');
  await panel.click({ position: { x: 3, y: 30 } });
  await page.waitForURL('**/configuracoes');
  assert.equal(await page.getByRole('button', { name: 'Fechar menu' }).count(), 0);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('http://127.0.0.1:4179/grafo');
  await page.waitForFunction(() => document.querySelector('[role=application]')?._cyreg?.cy?.edges().length === 2);
  await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.$id('edge-1').emit('tap'));
  const relationshipDrawer = page.getByRole('dialog', { name: 'Detalhes do vínculo' });
  await relationshipDrawer.waitFor();
  page.once('dialog', dialog => dialog.dismiss());
  await relationshipDrawer.getByRole('button', { name: 'Excluir vínculo' }).click();
  assert.equal(deletedRelationshipIds.size, 0);
  await relationshipDrawer.waitFor();
  page.once('dialog', dialog => dialog.accept());
  await relationshipDrawer.getByRole('button', { name: 'Excluir vínculo' }).click();
  await relationshipDrawer.waitFor({ state: 'hidden' });
  await page.waitForFunction(() => document.querySelector('[role=application]')?._cyreg?.cy?.edges().length === 1);
  assert.deepEqual([...deletedRelationshipIds], [1]);
  assert.equal(await page.getByRole('heading', { name: 'Vínculos cadastrados' }).count(), 0);
  await page.getByText('Lixeira de vínculos (1)').click();
  await page.getByRole('button', { name: 'Restaurar', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('[role=application]')?._cyreg?.cy?.edges().length === 2);
  assert.equal(deletedRelationshipIds.size, 0);
  await page.getByRole('button', { name: 'Sair', exact: true }).click();
  await page.waitForURL('**/login');
  assert.equal(logoutPosts, 1);
  assert.deepEqual(errors, []);
  console.log('PASS: regressão de navegador, lixeira e restauração de vínculo pelo grafo, organização automática do grafo e reajuste manual, enquadramento de Teia/UML e barras no celular, composição do HP, termos neutros, justificativa e data persistidas, histórico com autor e valores, prévia sem gravação, repetição após falha da prévia, barras de 56px e limites de cores, atualização entre abas preservando o mapa.');
} catch (error) { console.error('Falha original:', error); console.error('Erros de página:', errors); try { console.error('Interface:', (await page.locator('body').innerText({ timeout: 3000 })).slice(0, 5000)); await page.screenshot({ path: join(cacheRoot, 'hp-browser-error.png'), fullPage: false, timeout: 5000 }); } catch { /* Preserve the original failure when Chromium cannot capture the page. */ } throw error; } finally { await browser.close(); server.close(); }
