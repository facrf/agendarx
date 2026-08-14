import {
  ArrowLeft,
  Camera,
  ImageUp,
  Plus,
  Save,
  Trash2,
  UserRoundPlus,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { ChangeEvent, DragEvent, FormEvent } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Button, PageHeader, Spinner, cn } from "../components/ui";
import { useToast } from "../contexts/ToastContext";
import { api, apiUrl, errorMessage } from "../services/api";
import type {
  Categoria,
  ContatoPayload,
  PessoaDetalhe,
  TipoMeioContato,
} from "../types/api";
import { dataTransferHasFiles, droppedFiles } from "../utils/dropFiles";

export function PersonFormPage() {
  const { id } = useParams();
  const pessoaId = id ? Number(id) : null;
  const editando = pessoaId !== null;
  const navigate = useNavigate();
  const { notify } = useToast();
  const [nome, setNome] = useState("");
  const [descricao, setDescricao] = useState("");
  const [categoriaId, setCategoriaId] = useState<number | null>(null);
  const [contatos, setContatos] = useState<ContatoPayload[]>([]);
  const [contatosOriginais, setContatosOriginais] = useState<number[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [tipos, setTipos] = useState<TipoMeioContato[]>([]);
  const [temFoto, setTemFoto] = useState(false);
  const [removerFoto, setRemoverFoto] = useState(false);
  const [foto, setFoto] = useState<File | null>(null);
  const [arrastandoFoto, setArrastandoFoto] = useState(false);
  const [carregando, setCarregando] = useState(true);
  const [salvando, setSalvando] = useState(false);

  useEffect(() => {
    const requests: [Promise<Categoria[]>, Promise<TipoMeioContato[]>, Promise<PessoaDetalhe> | null] = [
      api.get("/api/configuracoes/categorias"),
      api.get("/api/configuracoes/tipos-contato"),
      pessoaId ? api.get(`/api/pessoas/${pessoaId}`) : null,
    ];
    Promise.all([requests[0], requests[1], requests[2]])
      .then(([categoriasData, tiposData, pessoa]) => {
        setCategorias(categoriasData);
        setTipos(tiposData);
        if (pessoa) {
          setNome(pessoa.nome);
          setDescricao(pessoa.descricao || "");
          setCategoriaId(pessoa.categoria_id);
          setContatos(pessoa.contatos.map((contato) => ({ ...contato })));
          setContatosOriginais(pessoa.contatos.map((contato) => contato.id));
          setTemFoto(pessoa.tem_foto);
        }
      })
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setCarregando(false));
  }, [pessoaId, notify]);

  const preview = useMemo(() => (foto ? URL.createObjectURL(foto) : null), [foto]);
  useEffect(() => () => { if (preview) URL.revokeObjectURL(preview); }, [preview]);

  const adicionarContato = () => {
    setContatos((atuais) => [...atuais, { tipo_contato_id: tipos[0]?.id || 0, valor: "" }]);
  };

  const alterarContato = (index: number, patch: Partial<ContatoPayload>) => {
    setContatos((atuais) => atuais.map((contato, i) => (i === index ? { ...contato, ...patch } : contato)));
  };

  const selecionarFoto = (file: File | null) => {
    if (file?.size === 0) {
      notify("A imagem selecionada está vazia", "erro");
      return;
    }
    if (file && !arquivoPareceImagem(file)) {
      notify("Solte ou selecione um arquivo de imagem", "erro");
      return;
    }
    setFoto(file);
    if (file) setRemoverFoto(false);
  };

  const escolherFoto = (event: ChangeEvent<HTMLInputElement>) => {
    selecionarFoto(event.target.files?.[0] || null);
    event.target.value = "";
  };

  const soltarFoto = async (event: DragEvent<HTMLLabelElement>) => {
    if (!dataTransferHasFiles(event.dataTransfer)) return;
    event.preventDefault();
    event.stopPropagation();
    setArrastandoFoto(false);
    const dropped = await droppedFiles(event.dataTransfer);
    if (dropped.ignoredDirectories > 0) {
      notify("Pastas não podem ser usadas como foto", "erro");
    }
    if (dropped.files.length > 1) {
      notify("Somente a primeira imagem foi selecionada");
    }
    selecionarFoto(dropped.files[0] || null);
  };

  const salvar = async (event: FormEvent) => {
    event.preventDefault();
    if (!nome.trim()) return notify("Informe o nome da pessoa", "erro");
    if (contatos.some((contato) => !contato.tipo_contato_id || !contato.valor.trim())) {
      return notify("Preencha ou remova os meios de contato incompletos", "erro");
    }

    setSalvando(true);
    try {
      let destinoId = pessoaId;
      if (!destinoId) {
        const criada = await api.post<PessoaDetalhe>("/api/pessoas", {
          nome: nome.trim(),
          categoria_id: categoriaId,
          descricao: descricao.trim() || null,
          contatos: contatos.map(({ tipo_contato_id, valor }) => ({ tipo_contato_id, valor: valor.trim() })),
        });
        destinoId = criada.id;
      } else {
        await api.put(`/api/pessoas/${destinoId}`, {
          nome: nome.trim(),
          categoria_id: categoriaId,
          descricao: descricao.trim() || null,
        });
        const idsAtuais = new Set(contatos.flatMap((contato) => (contato.id ? [contato.id] : [])));
        await Promise.all(
          contatosOriginais.filter((contatoId) => !idsAtuais.has(contatoId)).map((contatoId) => api.delete(`/api/pessoas/contatos/${contatoId}`)),
        );
        await Promise.all(
          contatos.map((contato) => {
            const payload = { tipo_contato_id: contato.tipo_contato_id, valor: contato.valor.trim() };
            return contato.id
              ? api.put(`/api/pessoas/contatos/${contato.id}`, payload)
              : api.post(`/api/pessoas/${destinoId}/contatos`, payload);
          }),
        );
      }

      if (foto) await api.put(`/api/dossie/pessoas/${destinoId}/foto`, foto, foto.type);
      else if (removerFoto && temFoto) await api.delete(`/api/dossie/pessoas/${destinoId}/foto`);

      notify(editando ? "Pessoa atualizada" : "Pessoa cadastrada");
      navigate(`/pessoas/${destinoId}`, { replace: true });
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  if (carregando) return <Spinner label={editando ? "Carregando perfil" : "Preparando formulário"} />;

  return (
    <div className="mx-auto max-w-5xl">
      <PageHeader
        eyebrow={editando ? "Atualizar perfil" : "Novo contato"}
        title={editando ? "Editar pessoa" : "Cadastrar pessoa"}
        description="Organize os dados básicos e adicione quantos meios de contato forem necessários."
        action={<Link className="btn btn-secondary" to={pessoaId ? `/pessoas/${pessoaId}` : "/pessoas"}><ArrowLeft className="size-4" /> Voltar</Link>}
      />

      <form onSubmit={salvar} className="space-y-5">
        <section className="panel p-5 sm:p-7">
          <div className="mb-6 flex items-center gap-3">
            <div className="grid size-11 place-items-center rounded-2xl bg-teal-50 text-teal-700"><UserRoundPlus className="size-5" /></div>
            <div><h2 className="font-display text-xl font-semibold">Dados principais</h2><p className="text-sm text-slate-500">Nome, categoria e retrato principal.</p></div>
          </div>
          <div className="grid gap-6 lg:grid-cols-[1fr_15rem]">
            <div className="space-y-5">
              <div><label className="field-label" htmlFor="nome">Nome completo</label><input id="nome" className="field" required value={nome} onChange={(event) => setNome(event.target.value)} placeholder="Como esta pessoa é conhecida?" /></div>
              <div>
                <label className="field-label" htmlFor="categoria">Categoria</label>
                <select id="categoria" className="field" value={categoriaId ?? ""} onChange={(event) => setCategoriaId(event.target.value ? Number(event.target.value) : null)}>
                  <option value="">Sem categoria</option>
                  {categorias.map((categoria) => <option key={categoria.id} value={categoria.id}>{categoria.nome_categoria}</option>)}
                </select>
              </div>
            </div>
            <div>
              <label className="field-label">Foto principal</label>
              <label
                className={cn(
                  "group relative flex aspect-square cursor-pointer items-center justify-center overflow-hidden rounded-3xl border-2 border-dashed bg-slate-50 transition hover:border-teal-400 hover:bg-teal-50",
                  arrastandoFoto ? "border-teal-500 bg-teal-50 ring-4 ring-teal-100" : "border-slate-200",
                )}
                onDragEnter={(event) => { if (dataTransferHasFiles(event.dataTransfer)) { event.preventDefault(); event.stopPropagation(); setArrastandoFoto(true); } }}
                onDragOver={(event) => { if (dataTransferHasFiles(event.dataTransfer)) { event.preventDefault(); event.stopPropagation(); event.dataTransfer.dropEffect = "copy"; } }}
                onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setArrastandoFoto(false); }}
                onDrop={(event) => void soltarFoto(event)}
              >
                {preview ? <img className="h-full w-full object-cover" src={preview} alt="Prévia da foto" /> : temFoto && !removerFoto && pessoaId ? <img className="h-full w-full object-cover" src={apiUrl(`/api/dossie/pessoas/${pessoaId}/foto`)} alt="Foto atual" /> : <div className="text-center text-slate-400"><ImageUp className="mx-auto size-7" /><span className="mt-2 block text-xs">Escolher imagem</span></div>}
                <input className="sr-only" type="file" accept="image/*" onChange={escolherFoto} />
                <span className={cn("absolute inset-x-3 bottom-3 rounded-xl bg-slate-950/65 px-3 py-2 text-center text-xs font-medium text-white backdrop-blur transition", arrastandoFoto ? "opacity-100" : "opacity-0 group-hover:opacity-100")}><Camera className="mr-1 inline size-3.5" /> {arrastandoFoto ? "Solte a imagem aqui" : "Clique ou arraste uma foto"}</span>
              </label>
              {temFoto && !foto && <button type="button" className="mt-2 w-full text-xs font-medium text-rose-600 hover:underline" onClick={() => setRemoverFoto((value) => !value)}>{removerFoto ? "Manter foto atual" : "Remover foto atual"}</button>}
            </div>
          </div>
        </section>

        <section className="panel p-5 sm:p-7">
          <div className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div><h2 className="font-display text-xl font-semibold">Meios de contato</h2><p className="text-sm text-slate-500">WhatsApp, e-mail, Nostr, sites e outros canais.</p></div>
            <Button type="button" variant="secondary" onClick={adicionarContato} disabled={tipos.length === 0}><Plus className="size-4" /> Adicionar meio</Button>
          </div>
          {tipos.length === 0 ? (
            <div className="rounded-2xl bg-amber-50 p-4 text-sm text-amber-800">Cadastre ao menos um meio de contato em <Link className="font-semibold underline" to="/configuracoes">Configurações</Link>.</div>
          ) : contatos.length === 0 ? (
            <button type="button" onClick={adicionarContato} className="w-full rounded-2xl border border-dashed border-slate-300 p-8 text-sm text-slate-500 hover:border-teal-400 hover:bg-teal-50">+ Adicionar o primeiro meio de contato</button>
          ) : (
            <div className="space-y-3">
              {contatos.map((contato, index) => (
                <div key={contato.id ?? `novo-${index}`} className="grid gap-3 rounded-2xl bg-slate-50 p-3 sm:grid-cols-[13rem_1fr_auto]">
                  <select className="field" aria-label="Tipo do contato" value={contato.tipo_contato_id} onChange={(event) => alterarContato(index, { tipo_contato_id: Number(event.target.value) })}>{tipos.map((tipo) => <option key={tipo.id} value={tipo.id}>{tipo.nome_tipo}</option>)}</select>
                  <input className="field" aria-label="Valor do contato" value={contato.valor} onChange={(event) => alterarContato(index, { valor: event.target.value })} placeholder="Número, endereço, usuário ou URL" />
                  <button type="button" className="icon-button text-rose-600" onClick={() => setContatos((atuais) => atuais.filter((_, i) => i !== index))} aria-label="Remover contato"><Trash2 className="size-4" /></button>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="panel p-5 sm:p-7">
          <label className="field-label" htmlFor="descricao">Descrição</label>
          <p className="mb-3 text-sm text-slate-500">Registre contexto, características ou outras informações relevantes sobre esta pessoa.</p>
          <textarea
            id="descricao"
            className="field min-h-36 resize-y"
            maxLength={5000}
            value={descricao}
            onChange={(event) => setDescricao(event.target.value)}
            placeholder="Descreva esta pessoa…"
          />
          <p className="mt-2 text-right text-xs text-slate-400">{descricao.length}/5000</p>
        </section>

        <div className="flex justify-end gap-3">
          <Link className="btn btn-ghost" to={pessoaId ? `/pessoas/${pessoaId}` : "/pessoas"}>Cancelar</Link>
          <Button type="submit" loading={salvando}><Save className="size-4" /> {editando ? "Salvar alterações" : "Cadastrar pessoa"}</Button>
        </div>
      </form>
    </div>
  );
}

function arquivoPareceImagem(file: File) {
  return file.type.startsWith("image/") || /\.(avif|bmp|gif|ico|jpe?g|png|webp)$/i.test(file.name);
}
