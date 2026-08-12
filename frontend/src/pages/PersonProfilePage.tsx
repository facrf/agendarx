import {
  ArrowLeft,
  AtSign,
  CalendarDays,
  Edit3,
  Files,
  GitFork,
  Mail,
  Phone,
  Radar,
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
import type { PessoaDetalhe, TipoMeioContato } from "../types/api";
import { formatDate } from "../utils/format";

export function PersonProfilePage() {
  const id = Number(useParams().id);
  const [params, setParams] = useSearchParams();
  const abaParam = params.get("aba");
  const aba = abaParam === "dossie" || abaParam === "osint" ? abaParam : "perfil";
  const [pessoa, setPessoa] = useState<PessoaDetalhe | null>(null);
  const [tipos, setTipos] = useState<TipoMeioContato[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [excluindo, setExcluindo] = useState(false);
  const navigate = useNavigate();
  const { notify } = useToast();

  useEffect(() => {
    Promise.all([
      api.get<PessoaDetalhe>(`/api/pessoas/${id}`),
      api.get<TipoMeioContato[]>("/api/configuracoes/tipos-contato"),
    ])
      .then(([pessoaData, tiposData]) => { setPessoa(pessoaData); setTipos(tiposData); })
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
          <Avatar pessoaId={pessoa.id} nome={pessoa.nome} temFoto={pessoa.tem_foto} cor={cor} size="xl" />
          <div className="min-w-0 flex-1">
            <span className="chip"><span className="size-2 rounded-full bg-[var(--person-color)]" />{pessoa.nome_categoria || "Sem categoria"}</span>
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
        <button className={cn("flex flex-1 items-center justify-center gap-2 rounded-xl px-5 py-2.5 text-sm font-semibold transition sm:flex-none", aba === "dossie" ? "bg-ink text-white" : "text-slate-500 hover:bg-slate-50")} onClick={() => setParams({ aba: "dossie" })}><Files className="size-4" /> Dossiê</button>
        <button className={cn("flex flex-1 items-center justify-center gap-2 rounded-xl px-5 py-2.5 text-sm font-semibold transition sm:flex-none", aba === "osint" ? "bg-ink text-white" : "text-slate-500 hover:bg-slate-50")} onClick={() => setParams({ aba: "osint" })}><Radar className="size-4" /> Pesquisa pública</button>
      </div>

      {aba === "dossie" ? <section className="panel p-5 sm:p-7"><DossierPanel pessoaId={id} /></section> : aba === "osint" ? <section className="panel p-5 sm:p-7"><OSINTTab pessoaId={id} /></section> : (
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

function ContactIcon({ tipo }: { tipo: string }) {
  const normalized = tipo.toLowerCase();
  const Icon = normalized.includes("mail") ? Mail : normalized.includes("tel") || normalized.includes("whats") ? Phone : AtSign;
  return <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-white text-teal-700 shadow-sm"><Icon className="size-4" /></div>;
}
