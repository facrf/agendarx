import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const server = createServer(async (req, res) => {
  const path = req.url.split('?')[0];
  const file = '/workspace/frontend/dist' + (path.startsWith('/assets/') ? path : '/index.html');
  try {
    res.setHeader('content-type', file.endsWith('.js') ? 'text/javascript' : file.endsWith('.css') ? 'text/css' : 'text/html');
    res.end(await readFile(file));
  } catch { res.statusCode = 404; res.end(); }
});
await new Promise(resolve => server.listen(4179, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium-browser', args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
const base = { categoria_id: 1, nome_categoria: 'Amigos', cor_hex: '#112233', tem_foto: false, pessoa_juridica: false, data_cadastro: '2026-09-14', descricao: '# Perfil\n\n**Importante**', contatos: [], etiquetas: '', favorito: false };
const people = [{ ...base, id: 1, nome: 'Ana' }, { ...base, id: 2, nome: 'Zeca' }, { ...base, id: 3, nome: 'Empresa', pessoa_juridica: true }];
const attachments = [1, 2].map(id => ({ id, pessoa_id: 1, nome_arquivo: `foto${id}.png`, mime_type: 'image/png', tamanho_bytes: 100, data_upload: '2026-09-14', url_stream: `/api/dossie/anexos/${id}/stream`, url_download: `/api/dossie/anexos/${id}/download` }));
const notes = {};
let photoPosts = 0, personPosts = 0, failPhoto = false;
await page.route('**/api/**', async route => {
  const req = route.request(), path = new URL(req.url()).pathname;
  let data = [];
  if (path.endsWith('/sessao')) data = { usuario: { id: 1, login: 'teste', perfil: 'admin' } };
  else if (path.endsWith('/categorias')) data = [{ id: 1, nome_categoria: 'Amigos', cor_hex: '#112233' }];
  else if (path.endsWith('/admin/diagnostico-armazenamento')) data = { banco_bytes: 0, dossie_bytes: 0, vinculos_bytes: 0, tarefas_bytes: 0, midia_total_bytes: 0, anexos_total: 0, pessoas_total: people.length, limite_usuario_tarefas_bytes: 1, max_arquivo_bytes: 1, usuarios: [] };
  else if (path === '/api/vinculos/grafo') data = { nodes: people.map(p => ({ id: p.id, label: p.nome, color: p.cor_hex, categoria: p.nome_categoria, pessoa_juridica: p.pessoa_juridica, contatos: [] })), edges: [{ id: 1, source: 1, target: 2, label: 'Amizade', descricao: '# Contexto', data_criacao: '2026-09-14' }] };
  else if (path === '/api/vinculos') data = [{ id: 1, pessoa_origem_id: 1, pessoa_destino_id: 2, tipo_vinculo: 'Amizade', descricao: '# Contexto' }];
  else if (path === '/api/pessoas') {
    if (req.method() === 'POST') { personPosts++; data = { ...base, ...req.postDataJSON(), id: 4 }; }
    else data = people;
  } else if (/^\/api\/pessoas\/\d+$/.test(path)) data = { ...people[0], ...(req.method() === 'PUT' ? req.postDataJSON() : {}) };
  else if (path.endsWith('/anexos')) data = attachments;
  else if (path.endsWith('/notas')) {
    if (req.method() === 'PUT') notes[path] = req.postDataJSON().notas;
    data = { notas: notes[path] || '' };
  } else if (path.endsWith('/foto') && req.method() === 'POST') {
    photoPosts++;
    assert.ok(req.headers()['content-type'].startsWith('multipart/form-data;'));
    if (failPhoto) return route.abort('failed');
    data = { mensagem: 'foto atualizada' };
  } else if (path.endsWith('/stream')) return route.fulfill({ contentType: 'image/png', body: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aJ1sAAAAASUVORK5CYII=', 'base64') });
  await route.fulfill({ json: data });
});
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
  await page.keyboard.press('Escape');
  await page.goto('http://127.0.0.1:4179/pessoas/nova');
  await page.getByLabel('Nome completo').fill('Nova pessoa');
  await page.getByLabel('Descrição', { exact: true }).fill('# Título\n\n**Texto**\n\n| A | B |\n| - | - |\n| 1 | 2 |');
  await page.getByRole('button', { name: 'Visualizar formatação' }).click();
  await page.getByRole('heading', { name: 'Título', exact: true }).waitFor();
  assert.equal(await page.locator('.markdown-text table').count(), 1);
  const image = await page.evaluate(() => { const c = document.createElement('canvas'); c.width = 2400; c.height = 1800; c.getContext('2d').fillRect(0, 0, 2400, 1800); return c.toDataURL('image/png').split(',')[1]; });
  await page.locator('input[type=file]').setInputFiles({ name: 'foto.png', mimeType: 'image/png', buffer: Buffer.from(image, 'base64') });
  failPhoto = true;
  await page.getByRole('button', { name: 'Cadastrar pessoa', exact: true }).click();
  await page.getByText(/Dados da pessoa salvos, mas a foto/).waitFor();
  failPhoto = false;
  await page.getByRole('button', { name: 'Cadastrar pessoa', exact: true }).click();
  await page.waitForURL('**/pessoas/4');
  assert.equal(personPosts, 1);
  assert.equal(photoPosts, 2);
  await page.goto('http://127.0.0.1:4179/grafo?busca=Ana');
  await page.waitForFunction(() => document.querySelector('[role=application]')?._cyreg?.cy?.nodes(':selected').length === 1);
  assert.equal(await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.nodes(':selected').style('border-color')), 'rgb(17,34,51)');
  await page.evaluate(() => document.querySelector('[role=application]')._cyreg.cy.edges().first().emit('tap'));
  await page.getByRole('dialog', { name: 'Detalhes do vínculo' }).waitFor();
  await page.getByAltText('foto1.png').click();
  await page.getByRole('button', { name: 'Próxima imagem' }).waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('dialog', { name: 'Detalhes do vínculo' }).waitFor();
  assert.equal(await page.getByRole('dialog', { name: 'foto1.png', exact: true }).count(), 0);
  await page.keyboard.press('Escape');
  await page.getByRole('dialog', { name: 'Detalhes do vínculo' }).waitFor({ state: 'hidden' });
  await page.goto('http://127.0.0.1:4179/calendario');
  await page.getByRole('heading', { name: 'Calendário', exact: true }).waitFor();
  await page.goto('http://127.0.0.1:4179/configuracoes');
  try {
    await page.getByRole('heading', { name: 'Configurações', exact: true }).waitFor({ timeout: 10_000 });
  } catch (error) {
    const body = (await page.locator('body').innerText()).slice(0, 1_000);
    throw new Error(`Configurações não carregou em ${page.url()}. Erros: ${errors.join(' | ') || 'nenhum'}. Conteúdo: ${body}`, { cause: error });
  }
  assert.deepEqual(errors, []);
  console.log('PASS: rotas sob demanda, agrupamento, filtros após recarregar, galeria teclado/mouse, notas, Markdown, foto multipart, repetição sem duplicar pessoa e grafo.');
} finally { await browser.close(); server.close(); }
