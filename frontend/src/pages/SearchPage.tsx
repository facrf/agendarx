/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { Search } from "lucide-react";
import { PageHeader, EmptyState } from "../components/ui";
import { Pagination } from "../components/Pagination";
import { api, apiUrl, errorMessage } from "../services/api";
import type { Pagina, BuscaResultado } from "../types/api";
const labels: Record<string, string> = { pessoa: "Pessoa", tarefa: "Tarefa", vinculo: "Vínculo", anexo_dossie: "Arquivo do dossiê", anexo_vinculo: "Arquivo do vínculo", anexo_tarefa: "Arquivo da tarefa" };
export function SearchPage() {
  const [params, setParams] = useSearchParams();
  const query = params.get("busca") ?? "";
  const type = params.get("tipo") ?? "";
  const page = Math.max(1, Number(params.get("pagina")) || 1);
  const [result, setResult] = useState<Pagina<BuscaResultado> | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  const change = (values: Record<string, string>) => { const next = new URLSearchParams(params); Object.entries(values).forEach(([key, value]) => value ? next.set(key, value) : next.delete(key)); setParams(next, { replace: true }); };
  useEffect(() => {
    const controller = new AbortController();
    if (!query.trim()) { setResult(null); setLoading(false); setError(""); return; }
    setLoading(true); setError("");
    const timer = window.setTimeout(() => {
      const args = new URLSearchParams({ busca: query, pagina: String(page), por_pagina: "30" });
      if (type) args.set("tipo", type);
      api.get<Pagina<BuscaResultado>>(`/api/busca?${args}`, { signal: controller.signal }).then(setResult)
        .catch(e => { if (!controller.signal.aborted) setError(errorMessage(e)); })
        .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    }, 250);
    return () => { controller.abort(); window.clearTimeout(timer); };
  }, [query, type, page, revision]);
  return <div>
    <PageHeader title="Busca global" eyebrow="Encontre no sistema" description="Pesquise nomes, contatos, etiquetas, descrições, notas e conteúdo de arquivos de texto." />
    <div className="panel mb-5 flex flex-wrap gap-3 p-4">
      <input className="field flex-1" type="search" maxLength={200} aria-label="Termo da busca global" placeholder="Digite uma ou mais palavras…" value={query} onChange={e => change({ busca: e.target.value, pagina: "1" })} />
      <select className="field w-auto" aria-label="Tipo de resultado" value={type} onChange={e => change({ tipo: e.target.value, pagina: "1" })}><option value="">Todos os resultados</option><option value="pessoa">Pessoas</option><option value="tarefa">Tarefas</option><option value="vinculo">Vínculos</option><option value="anexo">Arquivos</option></select>
    </div>
    {error && <div className="rounded-xl bg-amber-50 p-4" role="alert">{error} <button className="underline" onClick={() => setRevision(v => v + 1)}>Tentar novamente</button></div>}
    {loading && <p className="mb-3 text-sm text-slate-500" role="status">Buscando…</p>}
    {!query.trim() ? <EmptyState icon={<Search />} title="O que deseja encontrar?" description="A busca reúne pessoas, suas tarefas, vínculos e arquivos disponíveis." />
      : !loading && !error && result?.total === 0 ? <EmptyState icon={<Search />} title="Nenhum resultado encontrado" description="Tente outras palavras ou remova o filtro." /> : null}
    {!error && result && <div className="space-y-3">{result.itens.map(item => <article key={`${item.tipo}-${item.recurso_id}`} className="panel p-4">
      <span className="text-xs font-semibold text-teal-700">{labels[item.tipo] ?? item.tipo}</span>
      <h2 className="mt-1 font-semibold">{item.tipo.startsWith("anexo_") ? <a className="text-teal-800 underline" href={apiUrl(item.url)} target="_blank" rel="noopener noreferrer">{item.titulo}</a> : <Link className="text-teal-800 underline" to={item.url}>{item.titulo}</Link>}</h2>
      <p className="mt-2 break-words text-sm text-slate-500">{item.resumo}</p>
    </article>)}</div>}
    {result && !error && <Pagination page={page} pages={result.total_paginas} total={result.total} size={30} busy={loading} onPage={p => change({ pagina: String(p) })} />}
  </div>;
}
