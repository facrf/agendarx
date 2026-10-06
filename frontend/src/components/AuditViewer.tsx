/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, errorMessage } from "../services/api";
import type { AuditoriaDetalhe, Pagina } from "../types/api";
import { Pagination } from "./Pagination";
import { Button } from "./ui";
import { formatDate } from "../utils/format";
const empty = { usuario: "", recurso: "", acao: "", desde: "", ate: "", status: "" };
function resourceLink(path: string) {
  const person = path.match(/^\/api\/pessoas\/(\d+)(?:\/|$)/);
  if (person) return `/pessoas/${person[1]}`;
  const task = path.match(/^\/api\/calendario\/tarefas\/(\d+)(?:\/|$)/);
  if (task) return `/calendario?tarefa=${task[1]}`;
  const edge = path.match(/^\/api\/vinculos\/(\d+)(?:\/|$)/);
  return edge ? `/grafo?vinculo=${edge[1]}` : null;
}
export function AuditViewer() {
  const [filters, setFilters] = useState(empty);
  const [applied, setApplied] = useState(empty);
  const [page, setPage] = useState(1);
  const [size, setSize] = useState(10);
  const [result, setResult] = useState<Pagina<AuditoriaDetalhe> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true); setError("");
    const args = new URLSearchParams({ pagina: String(page), por_pagina: String(size) });
    Object.entries(applied).forEach(([key, value]) => { if (value) args.set(key, value); });
    api.get<Pagina<AuditoriaDetalhe>>(`/api/produtividade/auditoria/paginada?${args}`, { signal: controller.signal })
      .then(setResult).catch(e => { if (!controller.signal.aborted) setError(errorMessage(e)); })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [applied, page, size, revision]);
  return <div>
    <h3 className="mb-3 font-semibold">Auditoria de operações</h3>
    <form className="mb-4 grid gap-2 sm:grid-cols-2" onSubmit={e => { e.preventDefault(); setApplied({ ...filters }); setPage(1); }}>
      <input className="field" aria-label="Usuário da auditoria" maxLength={200} placeholder="Usuário" value={filters.usuario} onChange={e => setFilters({ ...filters, usuario: e.target.value })} />
      <input className="field" aria-label="Recurso da auditoria" maxLength={200} placeholder="Recurso ou caminho" value={filters.recurso} onChange={e => setFilters({ ...filters, recurso: e.target.value })} />
      <select className="field" aria-label="Ação da auditoria" value={filters.acao} onChange={e => setFilters({ ...filters, acao: e.target.value })}><option value="">Todas as ações</option>{["CRIAR", "ALTERAR", "EXCLUIR", "EXECUTAR"].map(action => <option key={action}>{action}</option>)}</select>
      <input className="field" type="number" min={100} max={599} aria-label="Status HTTP da auditoria" placeholder="Status HTTP" value={filters.status} onChange={e => setFilters({ ...filters, status: e.target.value })} />
      <label className="text-xs text-slate-500">Desde (UTC)<input className="field" type="date" value={filters.desde} onChange={e => setFilters({ ...filters, desde: e.target.value })} /></label>
      <label className="text-xs text-slate-500">Até (UTC)<input className="field" type="date" min={filters.desde || undefined} value={filters.ate} onChange={e => setFilters({ ...filters, ate: e.target.value })} /></label>
      <div className="flex flex-wrap gap-2 sm:col-span-2"><Button type="submit" loading={loading}>Filtrar auditoria</Button><Button type="button" variant="ghost" onClick={() => { setFilters(empty); setApplied(empty); setPage(1); setRevision(v => v + 1); }}>Limpar filtros</Button><Button type="button" variant="secondary" disabled={loading} onClick={() => setRevision(v => v + 1)}>Atualizar</Button></div>
    </form>
    {error && <p className="rounded-xl bg-amber-50 p-3" role="alert">{error}</p>}
    {loading && <p className="mb-2 text-sm text-slate-500" role="status">Carregando operações…</p>}
    {!error && result && <div className="max-h-[32rem] space-y-2 overflow-y-auto">{result.itens.map(item => {
      const link = resourceLink(item.recurso);
      return <details key={item.id} className="rounded-xl border border-slate-200 p-3 text-sm">
        <summary className="cursor-pointer"><strong>{item.usuario_login}</strong> · {item.acao} · <span className={item.status_http >= 400 ? "text-rose-700" : "text-teal-700"}>HTTP {item.status_http}</span><span className="mt-1 block break-all text-xs text-slate-500">{formatDate(item.data_evento, true)} · {item.recurso}</span></summary>
        <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-3 gap-y-2 text-xs"><dt>Registro</dt><dd>#{item.id}</dd><dt>Método</dt><dd>{item.metodo || "Não registrado nesta versão"}</dd><dt>Duração</dt><dd>{item.duracao_ms === null ? "Não registrada" : `${item.duracao_ms} ms`}</dd><dt>Resultado</dt><dd>{item.resumo || (item.status_http < 400 ? "Operação concluída" : "Operação recusada ou falhou")}</dd></dl>
        {link && <Link className="mt-3 inline-block text-xs font-semibold text-teal-800 underline" to={link}>Abrir recurso</Link>}
      </details>;
    })}{result.total === 0 && <p className="text-sm text-slate-500">Nenhuma operação corresponde aos filtros.</p>}</div>}
    {result && !error && <Pagination page={page} pages={result.total_paginas} total={result.total} size={size} busy={loading} onPage={setPage} onSize={n => { setSize(n); setPage(1); }} />}
  </div>;
}
