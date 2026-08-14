import {
  ArrowRight,
  Camera,
  ContactRound,
  Plus,
  Search,
  SlidersHorizontal,
  UsersRound,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { Link } from "react-router-dom";
import { Avatar, EmptyState, PageHeader, Spinner } from "../components/ui";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type { Categoria, PessoaResumo } from "../types/api";
import { formatDate } from "../utils/format";

export function PeoplePage() {
  const [pessoas, setPessoas] = useState<PessoaResumo[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [busca, setBusca] = useState("");
  const [categoria, setCategoria] = useState("");
  const [carregando, setCarregando] = useState(true);
  const { notify } = useToast();

  useEffect(() => {
    Promise.all([
      api.get<PessoaResumo[]>("/api/pessoas"),
      api.get<Categoria[]>("/api/configuracoes/categorias"),
    ])
      .then(([pessoasData, categoriasData]) => {
        setPessoas(pessoasData);
        setCategorias(categoriasData);
      })
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setCarregando(false));
  }, [notify]);

  const filtradas = useMemo(() => {
    const termo = busca.trim().toLocaleLowerCase("pt-BR");
    return pessoas.filter(
      (pessoa) =>
        (!termo || pessoa.nome.toLocaleLowerCase("pt-BR").includes(termo)) &&
        (!categoria || pessoa.categoria_id === Number(categoria)),
    );
  }, [pessoas, busca, categoria]);

  if (carregando) return <Spinner label="Abrindo sua agenda" />;

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
        <Stat icon={<UsersRound />} value={pessoas.length} label="pessoas cadastradas" />
        <Stat icon={<SlidersHorizontal />} value={categorias.length} label="categorias ativas" />
        <Stat icon={<Camera />} value={pessoas.filter((p) => p.tem_foto).length} label="perfis com foto" />
      </section>

      <section className="panel mb-6 flex flex-col gap-3 p-3 sm:flex-row">
        <label className="relative flex-1">
          <span className="sr-only">Buscar por nome</span>
          <Search className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
          <input
            className="field border-0 bg-slate-50 pl-10 shadow-none"
            placeholder="Buscar por nome..."
            value={busca}
            onChange={(event) => setBusca(event.target.value)}
          />
        </label>
        <select className="field border-0 bg-slate-50 shadow-none sm:w-64" value={categoria} onChange={(event) => setCategoria(event.target.value)}>
          <option value="">Todas as categorias</option>
          {categorias.map((item) => <option key={item.id} value={item.id}>{item.nome_categoria}</option>)}
        </select>
      </section>

      {pessoas.length === 0 ? (
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
        <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
          {filtradas.map((pessoa) => <PersonCard key={pessoa.id} pessoa={pessoa} />)}
        </section>
      )}
    </div>
  );
}

function Stat({ icon, value, label }: { icon: React.ReactNode; value: number; label: string }) {
  return (
    <div className="panel flex items-center gap-4 px-5 py-4">
      <div className="grid size-11 place-items-center rounded-2xl bg-teal-50 text-teal-700 [&>svg]:size-5">{icon}</div>
      <div><p className="font-display text-2xl font-semibold text-slate-950">{value}</p><p className="text-xs text-slate-500">{label}</p></div>
    </div>
  );
}

function PersonCard({ pessoa }: { pessoa: PessoaResumo }) {
  const cor = pessoa.cor_hex || "#86A6A3";
  const style = { "--category-color": cor } as CSSProperties;
  return (
    <Link
      to={`/pessoas/${pessoa.id}`}
      className="group panel relative overflow-hidden p-5 transition duration-300 hover:-translate-y-1 hover:border-teal-200 hover:shadow-xl"
      style={style}
    >
      <div className="absolute inset-x-0 top-0 h-1 bg-[var(--category-color)]" />
      <div className="flex items-start justify-between gap-4">
        <Avatar pessoaId={pessoa.id} nome={pessoa.nome} temFoto={pessoa.tem_foto} pessoaJuridica={pessoa.pessoa_juridica} cor={cor} size="lg" />
        <div className="grid size-9 place-items-center rounded-full bg-slate-50 text-slate-400 transition group-hover:bg-teal-50 group-hover:text-teal-700">
          <ArrowRight className="size-4 transition group-hover:translate-x-0.5" />
        </div>
      </div>
      <h2 className="mt-5 font-display text-xl font-semibold" style={{ color: cor }}>{pessoa.nome}</h2>
      {pessoa.pessoa_juridica && <span className="mt-2 inline-flex rounded-lg bg-slate-100 px-2 py-1 text-[10px] font-bold uppercase tracking-wide text-slate-500">Pessoa jurídica</span>}
      <div className="mt-2 flex items-center gap-2 text-xs text-slate-500">
        <span className="size-2 rounded-full" style={{ backgroundColor: cor }} />
        <span>{pessoa.nome_categoria || "Sem categoria"}</span>
      </div>
      <p className="mt-5 border-t border-slate-100 pt-3 text-xs text-slate-400">Na agenda desde {formatDate(pessoa.data_cadastro)}</p>
    </Link>
  );
}
