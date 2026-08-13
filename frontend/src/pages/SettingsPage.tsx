import {
  Check,
  Edit3,
  Palette,
  Plus,
  Save,
  Settings2,
  Tags,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { Button, EmptyState, PageHeader, Spinner } from "../components/ui";
import { AdminCredentialsManager, BrandingManager, ContactTransferManager, TaskNotificationManager } from "../components/SettingsTools";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type { Categoria, TipoMeioContato } from "../types/api";

const HEX_PATTERN = /^#[0-9A-Fa-f]{6}$/;

export function SettingsPage() {
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [tipos, setTipos] = useState<TipoMeioContato[]>([]);
  const [carregando, setCarregando] = useState(true);
  const { notify } = useToast();

  useEffect(() => {
    Promise.all([
      api.get<Categoria[]>("/api/configuracoes/categorias"),
      api.get<TipoMeioContato[]>("/api/configuracoes/tipos-contato"),
    ])
      .then(([categoriasData, tiposData]) => { setCategorias(categoriasData); setTipos(tiposData); })
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setCarregando(false));
  }, [notify]);

  if (carregando) return <Spinner label="Carregando configurações" />;

  return (
    <div>
      <PageHeader eyebrow="Personalização" title="Configurações" description="Defina a identidade visual, organize categorias e transfira sua agenda." />
      <div className="grid gap-6 xl:grid-cols-2">
        <BrandingManager />
        <AdminCredentialsManager />
        <TaskNotificationManager />
        <CategoryManager categorias={categorias} setCategorias={setCategorias} />
        <ContactTypeManager tipos={tipos} setTipos={setTipos} />
        <ContactTransferManager />
      </div>
    </div>
  );
}

function CategoryManager({ categorias, setCategorias }: {
  categorias: Categoria[];
  setCategorias: React.Dispatch<React.SetStateAction<Categoria[]>>;
}) {
  const [nome, setNome] = useState("");
  const [cor, setCor] = useState("#13716D");
  const [editando, setEditando] = useState<number | null>(null);
  const [salvando, setSalvando] = useState(false);
  const { notify } = useToast();

  const reset = () => { setNome(""); setCor("#13716D"); setEditando(null); };
  const iniciarEdicao = (categoria: Categoria) => { setNome(categoria.nome_categoria); setCor(categoria.cor_hex); setEditando(categoria.id); };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!nome.trim() || !HEX_PATTERN.test(cor)) return notify("Informe um nome e uma cor hexadecimal válida", "erro");
    setSalvando(true);
    try {
      const payload = { nome_categoria: nome.trim(), cor_hex: cor.toUpperCase() };
      if (editando) {
        const atualizada = await api.put<Categoria>(`/api/configuracoes/categorias/${editando}`, payload);
        setCategorias((items) => items.map((item) => item.id === editando ? atualizada : item).sort(sortCategoria));
        notify("Categoria atualizada");
      } else {
        const criada = await api.post<Categoria>("/api/configuracoes/categorias", payload);
        setCategorias((items) => [...items, criada].sort(sortCategoria));
        notify("Categoria criada");
      }
      reset();
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const excluir = async (categoria: Categoria) => {
    if (!window.confirm(`Excluir a categoria “${categoria.nome_categoria}”? Pessoas associadas ficarão sem categoria.`)) return;
    try {
      await api.delete(`/api/configuracoes/categorias/${categoria.id}`);
      setCategorias((items) => items.filter((item) => item.id !== categoria.id));
      if (editando === categoria.id) reset();
      notify("Categoria excluída");
    } catch (error) { notify(errorMessage(error), "erro"); }
  };

  return (
    <section className="panel overflow-hidden">
      <header className="flex items-center gap-3 border-b border-slate-100 p-5 sm:p-6"><div className="grid size-11 place-items-center rounded-2xl bg-fuchsia-50 text-fuchsia-700"><Palette className="size-5" /></div><div><h2 className="font-display text-xl font-semibold">Categorias</h2><p className="text-sm text-slate-500">Nome e identidade de cor.</p></div></header>
      <form className="border-b border-slate-100 bg-slate-50/70 p-4 sm:p-5" onSubmit={submit}>
        <div className="grid gap-3 sm:grid-cols-[1fr_9rem_auto]">
          <input className="field" value={nome} onChange={(event) => setNome(event.target.value)} placeholder="Ex.: Família" aria-label="Nome da categoria" required />
          <div className="flex items-center gap-2 rounded-xl border border-slate-200 bg-white p-1.5 shadow-sm">
            <input className="size-8 cursor-pointer rounded-lg border-0 bg-transparent" type="color" value={HEX_PATTERN.test(cor) ? cor : "#13716D"} onChange={(event) => setCor(event.target.value.toUpperCase())} aria-label="Selecionar cor" />
            <input className="min-w-0 flex-1 bg-transparent text-xs font-semibold uppercase outline-none" value={cor} onChange={(event) => setCor(event.target.value)} maxLength={7} aria-label="Cor hexadecimal" />
          </div>
          <Button type="submit" loading={salvando}>{editando ? <Save className="size-4" /> : <Plus className="size-4" />}{editando ? "Salvar" : "Criar"}</Button>
        </div>
        {editando && <button type="button" onClick={reset} className="mt-2 inline-flex items-center gap-1 text-xs font-medium text-slate-500 hover:text-slate-800"><X className="size-3" /> Cancelar edição</button>}
      </form>
      <div className="p-4 sm:p-5">
        {categorias.length === 0 ? <EmptyState icon={<Palette className="size-6" />} title="Nenhuma categoria" description="Crie uma categoria para organizar e colorir os perfis." /> : (
          <div className="space-y-2">
            {categorias.map((categoria) => (
              <article key={categoria.id} className="flex items-center gap-3 rounded-2xl border border-slate-100 p-3 transition hover:bg-slate-50">
                <div className="grid size-10 place-items-center rounded-xl text-white shadow-sm" style={{ backgroundColor: categoria.cor_hex }}><Check className="size-4" /></div>
                <div className="min-w-0 flex-1"><p className="truncate text-sm font-semibold text-slate-800">{categoria.nome_categoria}</p><p className="text-xs font-medium uppercase text-slate-400">{categoria.cor_hex}</p></div>
                <button className="icon-button" onClick={() => iniciarEdicao(categoria)} title="Editar"><Edit3 className="size-4" /></button>
                <button className="icon-button text-rose-600" onClick={() => void excluir(categoria)} title="Excluir"><Trash2 className="size-4" /></button>
              </article>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

function ContactTypeManager({ tipos, setTipos }: {
  tipos: TipoMeioContato[];
  setTipos: React.Dispatch<React.SetStateAction<TipoMeioContato[]>>;
}) {
  const [nome, setNome] = useState("");
  const [editando, setEditando] = useState<number | null>(null);
  const [salvando, setSalvando] = useState(false);
  const { notify } = useToast();

  const reset = () => { setNome(""); setEditando(null); };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!nome.trim()) return;
    setSalvando(true);
    try {
      const payload = { nome_tipo: nome.trim() };
      if (editando) {
        const atualizado = await api.put<TipoMeioContato>(`/api/configuracoes/tipos-contato/${editando}`, payload);
        setTipos((items) => items.map((item) => item.id === editando ? atualizado : item).sort(sortTipo));
        notify("Meio de contato atualizado");
      } else {
        const criado = await api.post<TipoMeioContato>("/api/configuracoes/tipos-contato", payload);
        setTipos((items) => [...items, criado].sort(sortTipo));
        notify("Meio de contato criado");
      }
      reset();
    } catch (error) { notify(errorMessage(error), "erro"); }
    finally { setSalvando(false); }
  };

  const excluir = async (tipo: TipoMeioContato) => {
    if (!window.confirm(`Excluir o meio “${tipo.nome_tipo}”?`)) return;
    try {
      await api.delete(`/api/configuracoes/tipos-contato/${tipo.id}`);
      setTipos((items) => items.filter((item) => item.id !== tipo.id));
      if (editando === tipo.id) reset();
      notify("Meio de contato excluído");
    } catch (error) { notify(errorMessage(error), "erro"); }
  };

  return (
    <section className="panel overflow-hidden">
      <header className="flex items-center gap-3 border-b border-slate-100 p-5 sm:p-6"><div className="grid size-11 place-items-center rounded-2xl bg-sky-50 text-sky-700"><Tags className="size-5" /></div><div><h2 className="font-display text-xl font-semibold">Meios de contato</h2><p className="text-sm text-slate-500">Canais disponíveis nos perfis.</p></div></header>
      <form className="border-b border-slate-100 bg-slate-50/70 p-4 sm:p-5" onSubmit={submit}>
        <div className="flex gap-3"><input className="field" value={nome} onChange={(event) => setNome(event.target.value)} placeholder="Ex.: Telegram, Site, Skype" required /><Button type="submit" loading={salvando}>{editando ? <Save className="size-4" /> : <Plus className="size-4" />}{editando ? "Salvar" : "Criar"}</Button></div>
        {editando && <button type="button" onClick={reset} className="mt-2 inline-flex items-center gap-1 text-xs font-medium text-slate-500 hover:text-slate-800"><X className="size-3" /> Cancelar edição</button>}
      </form>
      <div className="p-4 sm:p-5">
        {tipos.length === 0 ? <EmptyState icon={<Settings2 className="size-6" />} title="Nenhum meio cadastrado" description="Crie os canais que poderão ser usados nos perfis." /> : (
          <div className="flex flex-wrap gap-2">
            {tipos.map((tipo) => (
              <div key={tipo.id} className="flex items-center gap-2 rounded-xl border border-slate-200 bg-white py-1.5 pl-3 pr-1.5 text-sm font-medium text-slate-700 shadow-sm"><Tags className="size-3.5 text-teal-600" /><span>{tipo.nome_tipo}</span><button className="rounded-lg p-1.5 text-slate-400 hover:bg-teal-50 hover:text-teal-700" onClick={() => { setNome(tipo.nome_tipo); setEditando(tipo.id); }} title="Editar"><Edit3 className="size-3.5" /></button><button className="rounded-lg p-1.5 text-slate-400 hover:bg-rose-50 hover:text-rose-600" onClick={() => void excluir(tipo)} title="Excluir"><Trash2 className="size-3.5" /></button></div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

const sortCategoria = (a: Categoria, b: Categoria) => a.nome_categoria.localeCompare(b.nome_categoria, "pt-BR");
const sortTipo = (a: TipoMeioContato, b: TipoMeioContato) => a.nome_tipo.localeCompare(b.nome_tipo, "pt-BR");
