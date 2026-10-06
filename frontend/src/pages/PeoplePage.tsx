import {
  ArrowRight,
  Camera,
  ContactRound,
  Plus,
  Search,
  SlidersHorizontal,
  Star,
  UsersRound,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { Link } from "react-router-dom";
import { Avatar, EmptyState, PageHeader, Spinner } from "../components/ui";
import { useToast } from "../contexts/ToastContext";
import { useAuth } from "../contexts/AuthContext";
import { api, errorMessage } from "../services/api";
import type { Categoria, PessoaResumo, PessoasPagina } from "../types/api";
import { Pagination } from "../components/Pagination";
import { formatDate } from "../utils/format";

export function PeoplePage() {
  const { usuario } = useAuth();
  const preferenceKey = `agendarx:pessoas:${usuario?.id}`;
  const [pessoas, setPessoas] = useState<PessoaResumo[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [busca, setBusca] = useState(() => readPreference(preferenceKey, "busca"));
  const [categoria, setCategoria] = useState(() => readPreference(preferenceKey, "categoria"));
  const [tipo, setTipo] = useState(() => readPreference(preferenceKey, "tipo"));
  const [agrupar, setAgrupar] = useState(() => readPreference(preferenceKey, "agrupar") !== "nao");
  const [somenteFavoritos, setSomenteFavoritos] = useState(() => readPreference(preferenceKey, "favoritos") === "sim");
  const [carregando, setCarregando] = useState(true);
  const [pagina, setPagina] = useState(1);
  const [tamanho, setTamanho] = useState(30);
  const [resultado, setResultado] = useState<PessoasPagina | null>(null);
  const [erro, setErro] = useState(false);
  const [revision, setRevision] = useState(0);
  const { notify } = useToast();

  useEffect(() => {
    try {
      localStorage.setItem(preferenceKey, JSON.stringify({ busca, categoria, tipo, agrupar: agrupar ? "sim" : "nao", favoritos: somenteFavoritos ? "sim" : "nao" }));
    } catch { /* A lista também funciona com armazenamento indisponível. */ }
  }, [preferenceKey, busca, categoria, tipo, agrupar, somenteFavoritos]);

  const alternarFavorito = async (pessoa: PessoaResumo) => {
    const favorito = !pessoa.favorito;
    setPessoas((atuais) => atuais.map((item) => item.id === pessoa.id ? { ...item, favorito } : item));
    try { await api.put(`/api/produtividade/pessoas/${pessoa.id}/favorito`, { favorito }); setRevision(value => value + 1); }
    catch (error) {
      setPessoas((atuais) => atuais.map((item) => item.id === pessoa.id ? { ...item, favorito: pessoa.favorito } : item));
      notify(errorMessage(error), "erro");
    }
  };

  useEffect(() => {
    const controller = new AbortController();
    api.get<Categoria[]>("/api/configuracoes/categorias", { signal: controller.signal, cacheMs: 60_000 })
      .then(setCategorias).catch(error => { if (!controller.signal.aborted) notify(errorMessage(error), "erro"); });
    return () => controller.abort();
  }, [notify]);

  useEffect(() => {
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      setCarregando(true); setErro(false);
      const params = new URLSearchParams({ busca: busca.trim(), categoria, tipo, favoritos: String(somenteFavoritos), pagina: String(pagina), por_pagina: String(tamanho) });
      api.get<PessoasPagina>(`/api/pessoas/paginadas?${params}`, { signal: controller.signal })
        .then(data => {
          if (data.total_paginas > 0 && pagina > data.total_paginas) { setPagina(data.total_paginas); return; }
          setResultado(data); setPessoas(data.itens);
        }).catch(error => { if (!controller.signal.aborted) { setErro(true); notify(errorMessage(error), "erro"); } })
        .finally(() => { if (!controller.signal.aborted) setCarregando(false); });
    }, 250);
    return () => { controller.abort(); window.clearTimeout(timer); };
  }, [busca, categoria, tipo, somenteFavoritos, pagina, tamanho, revision, notify]);

  const filtradas = pessoas;
  const grupos = useMemo(() => {
    if (!agrupar) return [{ key: "todos", title: "Todos os contatos", pessoas: filtradas }];
    const groups = new Map<string, { key: string; title: string; pessoas: PessoaResumo[]; juridica: boolean }>();
    for (const pessoa of filtradas) {
      const key = `${pessoa.pessoa_juridica}-${pessoa.categoria_id ?? "sem"}`;
      if (!groups.has(key)) groups.set(key, { key, title: `${pessoa.pessoa_juridica ? "Pessoas jurídicas" : "Pessoas físicas"} · ${pessoa.nome_categoria || "Sem categoria"}`, pessoas: [], juridica: pessoa.pessoa_juridica });
      groups.get(key)!.pessoas.push(pessoa);
    }
    return [...groups.values()].sort((a, b) => Number(a.juridica) - Number(b.juridica) || a.title.localeCompare(b.title, "pt-BR"));
  }, [filtradas, agrupar]);

  if (carregando && !resultado) return <Spinner label="Abrindo sua agenda" />;

  return (
    <div>
      <PageHeader
        eyebrow="Sua rede"
        title="Pessoas"
        description="Encontre rapidamente cada contato e mantenha o contexto de cada relação por perto."
        action={
          <Link className="btn btn-primary" to="/pessoas/nova">
            <Plus className="size-4" /> Nova pessoa
          </Link>
        }
      />

      <section className="mb-6 grid gap-3 sm:grid-cols-3">
        <Stat icon={<UsersRound />} value={resultado?.total ?? 0} label="pessoas encontradas" />
        <Stat icon={<SlidersHorizontal />} value={categorias.length} label="categorias ativas" />
        <Stat icon={<Camera />} value={resultado?.total_com_foto ?? 0} label="perfis com foto" />
      </section>

      <section className="panel mb-6 flex flex-col gap-3 p-3 lg:flex-row lg:flex-wrap">
        <label className="relative flex-1">
          <span className="sr-only">Buscar por nome</span>
          <Search className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
          <input
            className="field border-0 bg-slate-50 pl-10 shadow-none"
            placeholder="Buscar em nomes, contatos, descrições, notas e arquivos..."
            value={busca}
            onChange={(event) => { setBusca(event.target.value); setPagina(1); }}
          />
        </label>
        <select aria-label="Categoria" className="field border-0 bg-slate-50 shadow-none lg:w-56" value={categoria} onChange={(event) => { setCategoria(event.target.value); setPagina(1); }}>
          <option value="">Todas as categorias</option>
          <option value="sem">Sem categoria</option>
          {categorias.map((item) => <option key={item.id} value={item.id}>{item.nome_categoria}</option>)}
        </select>
        <select aria-label="Tipo de pessoa" className="field border-0 bg-slate-50 shadow-none lg:w-48" value={tipo} onChange={(event) => { setTipo(event.target.value); setPagina(1); }}>
          <option value="">Todos os tipos</option><option value="fisica">Pessoas físicas</option><option value="juridica">Pessoas jurídicas</option>
        </select>
        <label className="flex items-center gap-2 px-2 text-sm"><input type="checkbox" checked={agrupar} onChange={(event) => setAgrupar(event.target.checked)} /> Agrupar por tipo e categoria</label>
        <label className="flex items-center gap-2 px-2 text-sm"><input type="checkbox" checked={somenteFavoritos} onChange={(event) => { setSomenteFavoritos(event.target.checked); setPagina(1); }} /> Somente favoritos</label>
        <button type="button" className="btn btn-ghost" onClick={() => { setBusca(""); setCategoria(""); setTipo(""); setSomenteFavoritos(false); setPagina(1); }}>Limpar filtros</button>
      </section>

      {erro && <div className="mb-4 rounded-xl bg-amber-50 p-3" role="alert">Não foi possível carregar a lista. <button type="button" className="underline" onClick={() => setRevision(value => value + 1)}>Tentar novamente</button></div>}
      {carregando && <p className="mb-3 text-sm text-slate-500" role="status">Atualizando resultados…</p>}
      {!erro && resultado?.total === 0 && !busca && !categoria && !tipo && !somenteFavoritos ? (
        <EmptyState
          icon={<ContactRound className="size-7" />}
          title="Sua agenda está pronta para começar"
          description="Cadastre a primeira pessoa e adicione seus meios de contato, foto e categoria."
          action={<Link className="btn btn-primary" to="/pessoas/nova"><Plus className="size-4" /> Cadastrar pessoa</Link>}
        />
      ) : filtradas.length === 0 ? (
        <EmptyState
          icon={<Search className="size-7" />}
          title="Nenhuma pessoa encontrada"
          description="Tente outro nome ou remova o filtro de categoria."
        />
      ) : (
        <div className="space-y-8">
          {grupos.map((grupo) => <section key={grupo.key}>
            <h2 className="mb-3 text-lg font-semibold text-slate-700">{grupo.title} <span className="chip">{grupo.pessoas.length}</span></h2>
            <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">{grupo.pessoas.map((pessoa) => <PersonCard key={pessoa.id} pessoa={pessoa} onFavorite={() => void alternarFavorito(pessoa)} />)}</div>
          </section>)}
        </div>
      )}
      {resultado && <Pagination page={pagina} pages={resultado.total_paginas} total={resultado.total} size={tamanho} busy={carregando} onPage={setPagina} onSize={size => { setTamanho(size); setPagina(1); }} />}
    </div>
  );
}

function readPreference(storageKey: string, key: string): string {
  try {
    const value = JSON.parse(localStorage.getItem(storageKey) || "{}")[key];
    return typeof value === "string" ? value : "";
  } catch { return ""; }
}

function Stat({ icon, value, label }: { icon: React.ReactNode; value: number; label: string }) {
  return (
    <div className="panel flex items-center gap-4 px-5 py-4">
      <div className="grid size-11 place-items-center rounded-2xl bg-teal-50 text-teal-700 [&>svg]:size-5">{icon}</div>
      <div><p className="font-display text-2xl font-semibold text-slate-950">{value}</p><p className="text-xs text-slate-500">{label}</p></div>
    </div>
  );
}

function PersonCard({ pessoa, onFavorite }: { pessoa: PessoaResumo; onFavorite: () => void }) {
  const cor = pessoa.cor_hex || "#86A6A3";
  const style = { "--category-color": cor } as CSSProperties;
  return (
    <article className="group panel relative overflow-hidden transition duration-300 hover:-translate-y-1 hover:border-teal-200 hover:shadow-xl" style={style}>
      <Link to={`/pessoas/${pessoa.id}`} className="block p-5">
      <div className="absolute inset-x-0 top-0 h-1 bg-[var(--category-color)]" />
      <div className="flex items-start justify-between gap-4">
        <Avatar pessoaId={pessoa.id} nome={pessoa.nome} temFoto={pessoa.tem_foto} pessoaJuridica={pessoa.pessoa_juridica} cor={cor} size="lg" />
        <div className="flex gap-2">
          <span className="grid size-9 place-items-center rounded-full bg-slate-50 text-slate-400 transition group-hover:bg-teal-50 group-hover:text-teal-700"><ArrowRight className="size-4 transition group-hover:translate-x-0.5" /></span>
        </div>
      </div>
      <h2 className="mt-5 font-display text-xl font-semibold" style={{ color: cor }}>{pessoa.nome}</h2>
      {pessoa.pessoa_juridica && <span className="mt-2 inline-flex rounded-lg bg-slate-100 px-2 py-1 text-[10px] font-bold uppercase tracking-wide text-slate-500">Pessoa jurídica</span>}
      <div className="mt-2 flex items-center gap-2 text-xs text-slate-500">
        <span className="size-2 rounded-full" style={{ backgroundColor: cor }} />
        <span>{pessoa.nome_categoria || "Sem categoria"}</span>
      </div>
      {pessoa.etiquetas && <div className="mt-3 flex flex-wrap gap-1">{pessoa.etiquetas.split("\u001f").filter(Boolean).map((etiqueta) => <span key={etiqueta} className="chip">#{etiqueta}</span>)}</div>}
      <p className="mt-5 border-t border-slate-100 pt-3 text-xs text-slate-400">Na agenda desde {formatDate(pessoa.data_cadastro)}</p>
      </Link>
      <button type="button" className="absolute right-16 top-5 grid size-9 place-items-center rounded-full bg-white/90 shadow-sm" onClick={onFavorite} aria-label={pessoa.favorito ? `Remover ${pessoa.nome} dos favoritos` : `Favoritar ${pessoa.nome}`} title={pessoa.favorito ? "Remover dos favoritos" : "Favoritar"}><Star className={`size-4 ${pessoa.favorito ? "fill-amber-400 text-amber-500" : "text-slate-400"}`} /></button>
    </article>
  );
}
