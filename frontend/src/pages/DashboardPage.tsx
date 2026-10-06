/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { CalendarDays, RefreshCw } from "lucide-react";
import { Button, PageHeader } from "../components/ui";
import { api, errorMessage } from "../services/api";
import type { PainelResumo, TarefaPainel } from "../types/api";
export function DashboardPage() {
  const [data, setData] = useState<PainelResumo | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [revision, setRevision] = useState(0);
  const [changing, setChanging] = useState<number | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    const start = new Date(); start.setHours(0, 0, 0, 0);
    const end = new Date(start); end.setDate(end.getDate() + 1);
    setLoading(true); setError("");
    api.get<PainelResumo>(`/api/painel?${new URLSearchParams({ inicio: start.toISOString(), fim: end.toISOString(), dia: `${start.getFullYear()}-${String(start.getMonth() + 1).padStart(2, "0")}-${String(start.getDate()).padStart(2, "0")}` })}`, { signal: controller.signal })
      .then(setData).catch(e => { if (!controller.signal.aborted) setError(errorMessage(e)); })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [revision]);
  useEffect(() => {
    const refresh = () => { if (!document.hidden) setRevision(v => v + 1); };
    const updated = (event: Event) => { if ((event as CustomEvent<{ path: string }>).detail?.path.startsWith("/api/calendario/")) refresh(); };
    const timer = window.setInterval(refresh, 60_000);
    window.addEventListener("focus", refresh); window.addEventListener("agendarx:data-updated", updated);
    document.addEventListener("visibilitychange", refresh);
    return () => { window.clearInterval(timer); window.removeEventListener("focus", refresh); window.removeEventListener("agendarx:data-updated", updated); document.removeEventListener("visibilitychange", refresh); };
  }, []);
  const complete = async (task: TarefaPainel) => {
    setChanging(task.id);
    try { await api.patch(`/api/calendario/tarefas/${task.id}/status`, { status: "CONCLUIDA" }); }
    catch (e) { setError(errorMessage(e)); }
    finally { setChanging(null); }
  };
  return <div>
    <PageHeader title="Seu dia" eyebrow="Painel inicial" description="Acompanhe suas pendências e os compromissos dos próximos sete dias." action={<Button variant="secondary" loading={loading} onClick={() => setRevision(v => v + 1)}><RefreshCw className="size-4" /> Atualizar</Button>} />
    {error && <p className="mb-4 rounded-xl bg-amber-50 p-3" role="alert">{error}</p>}
    {loading && !data && <p role="status">Carregando pendências…</p>}
    {data && <div className="grid items-start gap-5 xl:grid-cols-3">{[
      { title: "Hoje", tasks: data.hoje, total: data.total_hoje, color: "text-teal-800" },
      { title: "Atrasadas", tasks: data.atrasadas, total: data.total_atrasadas, color: "text-amber-800" },
      { title: "Próximos sete dias", tasks: data.proximas, total: data.total_proximas, color: "text-slate-700" },
    ].map(group => <section key={group.title} className="panel p-5">
      <h2 className={`mb-4 flex items-center justify-between text-lg font-semibold ${group.color}`}>{group.title}<span className="chip">{group.total}</span></h2>
      {!group.tasks.length && <p className="text-sm text-slate-500">Nenhuma tarefa pendente neste período.</p>}
      <div className="space-y-3">{group.tasks.map(task => <article key={task.id} className="rounded-xl border border-slate-100 p-3">
        <Link className="font-semibold text-teal-800 hover:underline" to={`/calendario?tarefa=${task.id}`}>{task.titulo}</Link>
        <p className="mt-1 text-xs text-slate-500">{task.dia_inteiro ? `${new Date(task.inicio_em.slice(0, 10) + "T12:00:00").toLocaleDateString("pt-BR")} · dia inteiro` : new Date(task.inicio_em).toLocaleString("pt-BR")} · {task.prioridade === "ALTA" ? "Prioridade alta" : task.prioridade === "BAIXA" ? "Prioridade baixa" : "Prioridade normal"}</p>
        <Button className="mt-2" variant="ghost" disabled={changing !== null} loading={changing === task.id} onClick={() => void complete(task)}>Concluir</Button>
      </article>)}</div>
      {group.total > group.tasks.length && <p className="mt-3 text-xs text-slate-500">Exibindo as primeiras {group.tasks.length} tarefas.</p>}
    </section>)}</div>}
    <Link className="btn btn-secondary mt-5" to="/calendario"><CalendarDays className="size-4" /> Abrir calendário</Link>
  </div>;
}
