import {
  ArrowLeft,
  Camera,
  ImageUp,
  Plus,
  Save,
  Tags,
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
  Etiqueta,
} from "../types/api";
import { dataTransferHasFiles, droppedFiles } from "../utils/dropFiles";
import { MarkdownText } from "../components/MarkdownText";
import { prepareProfilePhoto } from "../utils/profilePhoto";

export function PersonFormPage() {
  const { id } = useParams();
  const pessoaId = id ? Number(id) : null;
  const editando = pessoaId !== null;
  const navigate = useNavigate();
  const { notify } = useToast();
  const [nome, setNome] = useState("");
  const [descricao, setDescricao] = useState("");
  const [previewDescricao, setPreviewDescricao] = useState(false);
  const [pessoaJuridica, setPessoaJuridica] = useState(false);
  const [categoriaId, setCategoriaId] = useState<number | null>(null);
  const [contatos, setContatos] = useState<ContatoPayload[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [tipos, setTipos] = useState<TipoMeioContato[]>([]);
  const [etiquetas, setEtiquetas] = useState<Etiqueta[]>([]);
  const [etiquetasIds, setEtiquetasIds] = useState<number[]>([]);
  const [temFoto, setTemFoto] = useState(false);
  const [removerFoto, setRemoverFoto] = useState(false);
  const [foto, setFoto] = useState<File | null>(null);
  const [arrastandoFoto, setArrastandoFoto] = useState(false);
  const [carregando, setCarregando] = useState(true);
  const [salvando, setSalvando] = useState(false);
  const [progressoFoto, setProgressoFoto] = useState<number | null>(null);
  const [pessoaSalvaId, setPessoaSalvaId] = useState<number | null>(null);

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
          setPessoaJuridica(pessoa.pessoa_juridica);
          setCategoriaId(pessoa.categoria_id);
          setContatos(pessoa.contatos.map((contato) => ({ ...contato })));
          setTemFoto(pessoa.tem_foto);
        }
      })
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setCarregando(false));
  }, [pessoaId, notify]);

  useEffect(() => {
    Promise.all([
      api.get<Etiqueta[]>("/api/produtividade/etiquetas"),
      pessoaId ? api.get<Etiqueta[]>(`/api/produtividade/pessoas/${pessoaId}/etiquetas`) : Promise.resolve([]),
    ]).then(([todas, selecionadas]) => { setEtiquetas(todas); setEtiquetasIds(selecionadas.map((item) => item.id)); })
      .catch((error) => notify(errorMessage(error), "erro"));
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
      // Prepare the selected file before persisting the form, including on retries.
      const fotoPreparada = foto ? await prepareProfilePhoto(foto) : null;
      let destinoId = pessoaId || pessoaSalvaId;
      if (!destinoId) {
        const criada = await api.post<PessoaDetalhe>("/api/pessoas", {
          nome: nome.trim(),
          categoria_id: categoriaId,
          descricao: descricao.trim() || null,
          pessoa_juridica: pessoaJuridica,
          contatos: contatos.map(({ tipo_contato_id, valor }) => ({ tipo_contato_id, valor: valor.trim() })),
        });
        destinoId = criada.id;
        setPessoaSalvaId(criada.id);
        setContatos(criada.contatos.map((contato) => ({ ...contato })));
      } else {
        const atualizada = await api.put<PessoaDetalhe>(`/api/pessoas/${destinoId}`, {
          nome: nome.trim(),
          categoria_id: categoriaId,
          descricao: descricao.trim() || null,
          pessoa_juridica: pessoaJuridica,
          contatos: contatos.map(({ id, tipo_contato_id, valor }) => ({ id, tipo_contato_id, valor: valor.trim() })),
        });
        setContatos(atualizada.contatos.map((contato) => ({ ...contato })));
      }

      try {
        await api.put(`/api/produtividade/pessoas/${destinoId}/etiquetas`, { etiquetas_ids: etiquetasIds });
      } catch (error) {
        notify(`Dados da pessoa salvos, mas as etiquetas não foram atualizadas: ${errorMessage(error)}. Você pode tentar salvar novamente.`, "erro");
        return;
      }

      try {
        if (fotoPreparada) {
          const form = new FormData();
          form.append("arquivo", fotoPreparada, fotoPreparada.type === "image/webp" ? "foto.webp" : "foto.png");
          setProgressoFoto(0);
          await api.upload(`/api/dossie/pessoas/${destinoId}/foto`, form, setProgressoFoto);
        }
        else if (removerFoto && temFoto) await api.delete(`/api/dossie/pessoas/${destinoId}/foto`);
      } catch (error) {
        notify(`Dados da pessoa salvos, mas a foto não foi atualizada: ${errorMessage(error)}. Você pode tentar salvar novamente.`, "erro");
        return;
      }

      notify(editando ? "Pessoa atualizada" : "Pessoa cadastrada");
      navigate(`/pessoas/${destinoId}`, { replace: true });
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
      setProgressoFoto(null);
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
              <label className="flex cursor-pointer items-start gap-3 rounded-2xl border border-slate-200 bg-slate-50 p-4 transition hover:border-teal-300">
                <input type="checkbox" className="mt-0.5 size-4 accent-teal-700" checked={pessoaJuridica} onChange={(event) => setPessoaJuridica(event.target.checked)} />
                <span><span className="block text-sm font-semibold text-slate-800">Pessoa jurídica</span><span className="mt-0.5 block text-xs leading-5 text-slate-500">Identifica empresas e organizações com um avatar quadrado.</span></span>
              </label>
            </div>
            <div>
              <label className="field-label">Foto principal</label>
              <p className="mb-2 text-xs text-slate-500">A foto será otimizada para até 1600 px. Para guardar o original, use o dossiê.</p>
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
                <input className="sr-only" type="file" accept="image/*" disabled={salvando} onChange={escolherFoto} />
                <span className={cn("absolute inset-x-3 bottom-3 rounded-xl bg-slate-950/65 px-3 py-2 text-center text-xs font-medium text-white backdrop-blur transition", arrastandoFoto ? "opacity-100" : "opacity-0 group-hover:opacity-100")}><Camera className="mr-1 inline size-3.5" /> {arrastandoFoto ? "Solte a imagem aqui" : "Clique ou arraste uma foto"}</span>
              </label>
              {temFoto && !foto && <button type="button" className="mt-2 w-full text-xs font-medium text-rose-600 hover:underline" onClick={() => setRemoverFoto((value) => !value)}>{removerFoto ? "Manter foto atual" : "Remover foto atual"}</button>}
            </div>
          </div>
        </section>

        {etiquetas.length > 0 && <section className="panel p-5 sm:p-7">
          <div className="mb-4 flex items-center gap-2"><Tags className="size-5 text-teal-700" /><h2 className="font-display text-xl font-semibold">Etiquetas</h2></div>
          <p className="mb-4 text-sm text-slate-500">Use várias etiquetas para cruzar contextos além da categoria principal.</p>
          <div className="flex flex-wrap gap-2">{etiquetas.map((etiqueta) => <label key={etiqueta.id} className="flex cursor-pointer items-center gap-2 rounded-xl border px-3 py-2 text-sm" style={{ borderColor: etiquetasIds.includes(etiqueta.id) ? etiqueta.cor_hex : undefined, backgroundColor: etiquetasIds.includes(etiqueta.id) ? `${etiqueta.cor_hex}18` : undefined }}><input type="checkbox" checked={etiquetasIds.includes(etiqueta.id)} onChange={(event) => setEtiquetasIds((ids) => event.target.checked ? [...ids, etiqueta.id] : ids.filter((id) => id !== etiqueta.id))} /><span className="size-2 rounded-full" style={{ backgroundColor: etiqueta.cor_hex }} />{etiqueta.nome}</label>)}</div>
        </section>}

        <section className="panel p-5 sm:p-7">
          <div className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div><h2 className="font-display text-xl font-semibold">Meios de contato</h2><p className="text-sm text-slate-500">WhatsApp, e-mail, Nostr, sites e outros canais.</p></div>
            <Button type="button" variant="secondary" className="btn-add-contact" onClick={adicionarContato} disabled={tipos.length === 0}><Plus className="size-4" /> Adicionar meio</Button>
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
          <p className="mb-3 text-sm text-slate-500">Use Markdown: # títulos, **negrito**, listas, links e tabelas.</p>
          <Button type="button" variant="secondary" className="mb-3" onClick={() => setPreviewDescricao(!previewDescricao)}>{previewDescricao ? "Editar descrição" : "Visualizar formatação"}</Button>
          {previewDescricao ? <div className="min-h-80 rounded-xl border border-slate-200 p-4"><MarkdownText>{descricao || "Nenhuma descrição."}</MarkdownText></div> : <textarea
            id="descricao"
            className="field min-h-80 resize-y font-mono leading-7"
            maxLength={50000}
            value={descricao}
            onChange={(event) => setDescricao(event.target.value)}
            placeholder="Descreva esta pessoa…"
          />}
          <p className="mt-2 text-right text-xs text-slate-400">{descricao.length}/50000</p>
        </section>

        <div className="flex justify-end gap-3">
          {progressoFoto !== null && <span role="status" className="self-center text-sm text-slate-500">Enviando foto: {progressoFoto}%</span>}
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
