import {
  ArrowLeft,
  AlignLeft,
  AtSign,
  CalendarDays,
  CheckCircle2,
  CircleAlert,
  Clock3,
  Edit3,
  Files,
  GitFork,
  Mail,
  Phone,
  Radar,
  Paperclip,
  Repeat2,
  Trash2,
  UserRound,
} from "lucide-react";
import { useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { DossierPanel } from "../components/DossierPanel";
import { OSINTTab } from "../components/OSINTTab";
import { Avatar, Button, EmptyState, Spinner, cn } from "../components/ui";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type { PessoaDetalhe, TarefaCalendario, TipoMeioContato } from "../types/api";
import { formatDate } from "../utils/format";

export function PersonProfilePage() {
  const id = Number(useParams().id);
  const [params, setParams] = useSearchParams();
  const abaParam = params.get("aba");
  const aba = abaParam === "tarefas" || abaParam === "dossie" || abaParam === "osint" ? abaParam : "perfil";
  const [pessoa, setPessoa] = useState<PessoaDetalhe | null>(null);
  const [tipos, setTipos] = useState<TipoMeioContato[]>([]);
  const [tarefas, setTarefas] = useState<TarefaCalendario[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [excluindo, setExcluindo] = useState(false);
  const navigate = useNavigate();
  const { notify } = useToast();

  useEffect(() => {
    Promise.all([
      api.get<PessoaDetalhe>(`/api/pessoas/${id}`),
      api.get<TipoMeioContato[]>("/api/configuracoes/tipos-contato"),
      api.get<TarefaCalendario[]>(`/api/calendario/pessoas/${id}/tarefas`),
    ])
      .then(([pessoaData, tiposData, tarefasData]) => {
        setPessoa(pessoaData);
        setTipos(tiposData);
        setTarefas(tarefasData);
      })
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setCarregando(false));
  }, [id, notify]);

  const excluir = async () => {
    if (!pessoa || !window.confirm(`Excluir ${pessoa.nome} e todo o dossiê associado?`)) return;
    setExcluindo(true);
    try {
      await api.delete(`/api/pessoas/${id}`);
      notify("Pessoa excluída");
      navigate("/pessoas", { replace: true });
    } catch (error) {
      notify(errorMessage(error), "erro");
      setExcluindo(false);
    }
  };

  if (carregando) return <Spinner label="Carregando perfil" />;
  if (!pessoa) return <EmptyState icon={<UserRound className="size-7" />} title="Pessoa não encontrada" description="Este perfil pode ter sido removido." action={<Link className="btn btn-primary" to="/pessoas">Voltar à agenda</Link>} />;

  const cor = pessoa.cor_hex || "#86A6A3";
  return (
    <div style={{ "--person-color": cor } as CSSProperties}>
      <Link className="mb-5 inline-flex items-center gap-2 text-sm font-medium text-slate-500 hover:text-teal-800" to="/pessoas"><ArrowLeft className="size-4" /> Voltar para pessoas</Link>
      <section className="panel relative mb-6 overflow-hidden p-5 sm:p-7">
        <div className="absolute inset-x-0 top-0 h-1.5 bg-[var(--person-color)]" />
        <div className="flex flex-col gap-5 sm:flex-row sm:items-center">
          <Avatar pessoaId={pessoa.id} nome={pessoa.nome} temFoto={pessoa.tem_foto} pessoaJuridica={pessoa.pessoa_juridica} cor={cor} size="xl" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap gap-2"><span className="chip"><span className="size-2 rounded-full bg-[var(--person-color)]" />{pessoa.nome_categoria || "Sem categoria"}</span>{pessoa.pessoa_juridica && <span className="chip bg-slate-50 font-semibold">Pessoa jurídica</span>}</div>
            <h1 className="mt-3 font-display text-3xl font-semibold tracking-tight sm:text-4xl" style={{ color: cor }}>{pessoa.nome}</h1>
            <p className="mt-2 flex items-center gap-2 text-sm text-slate-400"><CalendarDays className="size-4" /> Cadastrado em {formatDate(pessoa.data_cadastro)}</p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Link className="btn btn-secondary" to={`/pessoas/${id}/editar`}><Edit3 className="size-4" /> Editar</Link>
            <Button variant="danger" loading={excluindo} onClick={() => void excluir()}><Trash2 className="size-4" /> Excluir</Button>
          </div>
        </div>
      </section>

      <div className="mb-6 flex flex-wrap gap-1 rounded-2xl border border-slate-200 bg-white p-1.5 shadow-sm sm:w-fit">
        <button className={cn("flex flex-1 items-center justify-center gap-2 rounded-xl px-5 py-2.5 text-sm font-semibold transition sm:flex-none", aba === "perfil" ? "bg-ink text-white" : "text-slate-500 hover:bg-slate-50")} onClick={() => setParams({})}><UserRound className="size-4" /> Perfil</button>
        <button className={cn("flex flex-1 items-center justify-center gap-2 rounded-xl px-5 py-2.5 text-sm font-semibold transition sm:flex-none", aba === "tarefas" ? "bg-ink text-white" : "text-slate-500 hover:bg-slate-50")} onClick={() => setParams({ aba: "tarefas" })}><CalendarDays className="size-4" /> Tarefas <span className={cn("rounded-full px-1.5 py-0.5 text-[10px]", aba === "tarefas" ? "bg-white/15 text-white" : "bg-slate-100 text-slate-500")}>{tarefas.length}</span></button>
        <button className={cn("flex flex-1 items-center justify-center gap-2 rounded-xl px-5 py-2.5 text-sm font-semibold transition sm:flex-none", aba === "dossie" ? "bg-ink text-white" : "text-slate-500 hover:bg-slate-50")} onClick={() => setParams({ aba: "dossie" })}><Files className="size-4" /> Dossiê</button>
        <button className={cn("flex flex-1 items-center justify-center gap-2 rounded-xl px-5 py-2.5 text-sm font-semibold transition sm:flex-none", aba === "osint" ? "bg-ink text-white" : "text-slate-500 hover:bg-slate-50")} onClick={() => setParams({ aba: "osint" })}><Radar className="size-4" /> Pesquisa pública</button>
      </div>

      {aba === "tarefas" ? <TasksPanel tarefas={tarefas} /> : aba === "dossie" ? <section className="panel p-5 sm:p-7"><DossierPanel pessoaId={id} /></section> : aba === "osint" ? <section className="panel p-5 sm:p-7"><OSINTTab pessoaId={id} /></section> : (
        <div className="grid gap-5 xl:grid-cols-[1fr_20rem]">
          <section className="panel p-5 sm:p-7">
            <h2 className="font-display text-xl font-semibold">Meios de contato</h2>
            <p className="mt-1 text-sm text-slate-500">Canais associados a esta pessoa.</p>
            {pessoa.contatos.length === 0 ? <div className="mt-5 rounded-2xl border border-dashed border-slate-200 p-7 text-center text-sm text-slate-400">Nenhum meio de contato cadastrado.</div> : (
              <div className="mt-5 grid gap-3 sm:grid-cols-2">
                {pessoa.contatos.map((contato) => {
                  const nomeTipo = tipos.find((tipo) => tipo.id === contato.tipo_contato_id)?.nome_tipo || "Contato";
                  return <article key={contato.id} className="flex items-center gap-3 rounded-2xl border border-slate-100 bg-slate-50 p-4"><ContactIcon tipo={nomeTipo} /><div className="min-w-0"><p className="text-xs font-semibold uppercase tracking-wide text-slate-400">{nomeTipo}</p><p className="truncate text-sm font-medium text-slate-800" title={contato.valor}>{contato.valor}</p></div></article>;
                })}
              </div>
            )}

            <div className="mt-7 border-t border-slate-100 pt-6">
              <div className="flex items-center gap-2">
                <AlignLeft className="size-5 text-teal-700" />
                <h2 className="font-display text-xl font-semibold">Descrição</h2>
              </div>
              <div className="mt-3 whitespace-pre-wrap rounded-2xl border border-slate-100 bg-slate-50 p-4 text-sm leading-7 text-slate-600">
                {pessoa.descricao || "Nenhuma descrição cadastrada para esta pessoa."}
              </div>
            </div>
          </section>
          <aside className="panel p-5">
            <div className="grid size-11 place-items-center rounded-2xl bg-coral/10 text-coral"><GitFork className="size-5" /></div>
            <h2 className="mt-4 font-display text-lg font-semibold">Conexões</h2>
            <p className="mt-2 text-sm leading-6 text-slate-500">Veja esta pessoa no mapa e compreenda seus vínculos na rede.</p>
            <Link className="btn btn-secondary mt-5 w-full" to={`/grafo?busca=${encodeURIComponent(pessoa.nome)}`}>Abrir no mapa</Link>
          </aside>
        </div>
      )}
    </div>
  );
}

function TasksPanel({ tarefas }: { tarefas: TarefaCalendario[] }) {
  const pendentes = tarefas.filter((tarefa) => tarefa.status !== "CONCLUIDA").length;
  return (
    <section className="panel p-5 sm:p-7">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="font-display text-xl font-semibold">Tarefas agendadas</h2>
          <p className="mt-1 text-sm text-slate-500">{pendentes} pendente(s) · {tarefas.length - pendentes} concluída(s)</p>
        </div>
        <Link className="btn btn-secondary" to="/calendario"><CalendarDays className="size-4" /> Abrir calendário</Link>
      </div>

      {tarefas.length === 0 ? (
        <div className="mt-5 rounded-2xl border border-dashed border-slate-200 p-8 text-center">
          <CalendarDays className="mx-auto size-7 text-slate-300" />
          <p className="mt-3 text-sm font-medium text-slate-600">Nenhuma tarefa vinculada a esta pessoa.</p>
          <Link className="btn btn-primary mt-4" to="/calendario">Agendar no calendário</Link>
        </div>
      ) : (
        <div className="mt-5 divide-y divide-slate-100 overflow-hidden rounded-2xl border border-slate-100">
          {tarefas.map((tarefa) => (
            <article key={tarefa.id} className="flex items-start gap-3 bg-white p-4 transition hover:bg-slate-50">
              <span className="mt-1 size-3 shrink-0 rounded-full" style={{ backgroundColor: tarefa.cor_hex }} />
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <Link className={cn("font-semibold text-slate-800 hover:text-teal-700 hover:underline", tarefa.status === "CONCLUIDA" && "text-slate-400 line-through")} to={`/calendario?tarefa=${tarefa.id}`}>{tarefa.titulo}</Link>
                  <TaskStatus tarefa={tarefa} />
                  {tarefa.prioridade === "ALTA" && <span className="inline-flex items-center gap-1 text-xs font-semibold text-rose-600"><CircleAlert className="size-3.5" /> Alta</span>}
                  {tarefa.recorrencia !== "NENHUMA" && <span className="inline-flex items-center gap-1 text-xs font-semibold text-violet-600"><Repeat2 className="size-3.5" /> Recorrente</span>}
                </div>
                {tarefa.descricao && <p className="mt-1 line-clamp-2 text-sm text-slate-500">{tarefa.descricao}</p>}
                <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-slate-400">
                  <span className="inline-flex items-center gap-1.5"><Clock3 className="size-3.5" /> {formatTaskPeriod(tarefa)}</span>
                  {tarefa.anexos.length > 0 && <span className="inline-flex items-center gap-1.5"><Paperclip className="size-3.5" /> {tarefa.anexos.length} anexo(s)</span>}
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function TaskStatus({ tarefa }: { tarefa: TarefaCalendario }) {
  if (tarefa.status === "CONCLUIDA") return <span className="chip bg-emerald-50 text-emerald-700"><CheckCircle2 className="size-3" /> Concluída</span>;
  if (tarefa.status === "EM_ANDAMENTO") return <span className="chip bg-blue-50 text-blue-700"><Clock3 className="size-3" /> Em andamento</span>;
  return <span className="chip bg-amber-50 text-amber-700"><Clock3 className="size-3" /> Pendente</span>;
}

function formatTaskPeriod(tarefa: TarefaCalendario) {
  if (tarefa.dia_inteiro) {
    const [ano, mes, dia] = tarefa.inicio_em.slice(0, 10).split("-").map(Number);
    return new Date(ano, mes - 1, dia).toLocaleDateString("pt-BR", { day: "2-digit", month: "long", year: "numeric" });
  }
  const inicio = new Date(tarefa.inicio_em);
  const data = inicio.toLocaleDateString("pt-BR", { day: "2-digit", month: "long", year: "numeric" });
  const hora = inicio.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  return `${data}, ${hora}`;
}

function ContactIcon({ tipo }: { tipo: string }) {
  const normalized = tipo.toLowerCase();
  const Icon = normalized.includes("mail") ? Mail : normalized.includes("tel") || normalized.includes("whats") ? Phone : AtSign;
  return <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-white text-teal-700 shadow-sm"><Icon className="size-4" /></div>;
}
