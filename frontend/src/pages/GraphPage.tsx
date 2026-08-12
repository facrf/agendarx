import {
  Edit3,
  Filter,
  GitFork,
  Info,
  Move,
  Network,
  Orbit,
  Plus,
  Save,
  Search,
  Trash2,
  Workflow,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { GraphViewer } from "../components/GraphViewer";
import type { GraphLayout } from "../components/GraphViewer";
import { RelationshipDrawer } from "../components/RelationshipDrawer";
import { RelationshipAttachmentEditor } from "../components/RelationshipMedia";
import { Button, EmptyState, PageHeader, Spinner, cn } from "../components/ui";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type {
  AnexoVinculo,
  Categoria,
  GrafoEdge,
  GrafoNode,
  GrafoResponse,
  PessoaResumo,
  PessoaVinculo,
  VinculoPayload,
} from "../types/api";

const emptyRelationship: VinculoPayload = {
  pessoa_origem_id: 0,
  pessoa_destino_id: 0,
  tipo_vinculo: "",
  descricao: "",
};

export function GraphPage() {
  const [searchParams] = useSearchParams();
  const [graph, setGraph] = useState<GrafoResponse>({ nodes: [], edges: [] });
  const [people, setPeople] = useState<PessoaResumo[]>([]);
  const [categories, setCategories] = useState<Categoria[]>([]);
  const [relationships, setRelationships] = useState<PessoaVinculo[]>([]);
  const [search, setSearch] = useState(searchParams.get("busca") || "");
  const [category, setCategory] = useState("");
  const [depth, setDepth] = useState(1);
  const [layout, setLayout] = useState<GraphLayout>("force");
  const [form, setForm] = useState<VinculoPayload>(emptyRelationship);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [selectedEdge, setSelectedEdge] = useState<GrafoEdge | null>(null);
  const [relationshipAttachments, setRelationshipAttachments] = useState<AnexoVinculo[]>([]);
  const [pendingRelationshipFiles, setPendingRelationshipFiles] = useState<File[]>([]);
  const [loadingRelationshipAttachments, setLoadingRelationshipAttachments] = useState(false);
  const navigate = useNavigate();
  const { notify } = useToast();

  const loadGraphData = useCallback(async () => {
    const [graphData, peopleData, categoryData, relationshipData] = await Promise.all([
      api.get<GrafoResponse>("/api/vinculos/grafo"),
      api.get<PessoaResumo[]>("/api/pessoas"),
      api.get<Categoria[]>("/api/configuracoes/categorias"),
      api.get<PessoaVinculo[]>("/api/vinculos"),
    ]);
    setGraph(graphData);
    setPeople(peopleData);
    setCategories(categoryData);
    setRelationships(relationshipData);
  }, []);

  useEffect(() => {
    loadGraphData()
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setLoading(false));
  }, [loadGraphData, notify]);

  const categoryGraph = useMemo(() => filterByCategory(graph, category), [graph, category]);
  const focusedNode = useMemo(
    () => findFocusedNode(categoryGraph.nodes, search),
    [categoryGraph.nodes, search],
  );
  const visibleGraph = useMemo(
    () => isolateConnections(categoryGraph, focusedNode?.id ?? null, depth),
    [categoryGraph, focusedNode?.id, depth],
  );

  const submitRelationship = async (event: FormEvent) => {
    event.preventDefault();
    if (!form.pessoa_origem_id || !form.pessoa_destino_id || !form.tipo_vinculo.trim()) {
      return notify("Selecione duas pessoas e informe o tipo de vínculo", "erro");
    }
    if (form.pessoa_origem_id === form.pessoa_destino_id) {
      return notify("Escolha pessoas diferentes", "erro");
    }

    setSaving(true);
    try {
      const payload = {
        ...form,
        tipo_vinculo: form.tipo_vinculo.trim(),
        descricao: form.descricao?.trim() || null,
      };
      let saved: PessoaVinculo;
      if (editingId) {
        saved = await api.put<PessoaVinculo>(`/api/vinculos/${editingId}`, payload);
      } else {
        saved = await api.post<PessoaVinculo>("/api/vinculos", payload);
      }
      const uploads = await Promise.allSettled(
        pendingRelationshipFiles.map((file) => {
          const data = new FormData();
          data.append("arquivo", file);
          return api.post<AnexoVinculo>(`/api/vinculos/${saved.id}/anexos`, data);
        }),
      );
      const failedUploads = uploads.filter((result) => result.status === "rejected");
      if (failedUploads.length > 0) {
        notify(`Vínculo salvo, mas ${failedUploads.length} anexo(s) falharam`, "erro");
      } else {
        notify(editingId ? "Vínculo atualizado" : "Vínculo criado");
      }
      setForm(emptyRelationship);
      setEditingId(null);
      setSelectedEdge(null);
      setRelationshipAttachments([]);
      setPendingRelationshipFiles([]);
      await loadGraphData();
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSaving(false);
    }
  };

  const editRelationship = async (relationship: PessoaVinculo) => {
    setEditingId(relationship.id);
    setForm({
      pessoa_origem_id: relationship.pessoa_origem_id,
      pessoa_destino_id: relationship.pessoa_destino_id,
      tipo_vinculo: relationship.tipo_vinculo,
      descricao: relationship.descricao || "",
    });
    setPendingRelationshipFiles([]);
    setRelationshipAttachments([]);
    setLoadingRelationshipAttachments(true);
    try {
      setRelationshipAttachments(await api.get<AnexoVinculo[]>(`/api/vinculos/${relationship.id}/anexos`));
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setLoadingRelationshipAttachments(false);
    }
  };

  const deleteRelationshipAttachment = async (attachment: AnexoVinculo) => {
    if (!window.confirm(`Excluir “${attachment.nome_arquivo}” desta relação?`)) return;
    try {
      await api.delete(`/api/vinculos/anexos/${attachment.id}`);
      setRelationshipAttachments((items) => items.filter((item) => item.id !== attachment.id));
      notify("Anexo excluído");
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const renameRelationshipAttachment = async (attachment: AnexoVinculo) => {
    const name = window.prompt("Novo nome do arquivo:", attachment.nome_arquivo)?.trim();
    if (!name || name === attachment.nome_arquivo) return;
    try {
      const updated = await api.put<AnexoVinculo>(`/api/vinculos/anexos/${attachment.id}`, { nome_arquivo: name });
      setRelationshipAttachments((items) => items.map((item) => item.id === updated.id ? updated : item));
      notify("Anexo renomeado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const deleteRelationship = async (relationship: PessoaVinculo) => {
    if (!window.confirm(`Excluir o vínculo “${relationship.tipo_vinculo}”?`)) return;
    try {
      await api.delete(`/api/vinculos/${relationship.id}`);
      notify("Vínculo excluído");
      setSelectedEdge(null);
      await loadGraphData();
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const openEdge = useCallback(
    (edgeId: number) => setSelectedEdge(graph.edges.find((edge) => edge.id === edgeId) || null),
    [graph.edges],
  );
  const openPerson = useCallback(
    (nodeId: number) => navigate(`/pessoas/${nodeId}`),
    [navigate],
  );

  if (loading) return <Spinner label="Desenhando sua rede" />;

  return (
    <div>
      <PageHeader
        eyebrow="Mapa interpessoal"
        title="Teia de vínculos"
        description="Alterne entre uma rede orgânica e um diagrama hierárquico, investigue conexões e reposicione pessoas livremente."
      />

      <div className="grid gap-5 2xl:grid-cols-[23rem_1fr]">
        <aside className="space-y-5">
          <RelationshipForm
            people={people}
            form={form}
            editing={editingId !== null}
            saving={saving}
            loadingAttachments={loadingRelationshipAttachments}
            attachments={relationshipAttachments}
            pendingFiles={pendingRelationshipFiles}
            onChange={setForm}
            onPendingFiles={setPendingRelationshipFiles}
            onDeleteAttachment={(attachment) => void deleteRelationshipAttachment(attachment)}
            onRenameAttachment={(attachment) => void renameRelationshipAttachment(attachment)}
            onSubmit={submitRelationship}
            onCancel={() => {
              setEditingId(null);
              setForm(emptyRelationship);
              setRelationshipAttachments([]);
              setPendingRelationshipFiles([]);
            }}
          />

          {relationships.length > 0 && (
            <section className="panel p-4">
              <h2 className="mb-3 px-1 font-display text-lg font-semibold">Vínculos cadastrados</h2>
              <div className="max-h-72 space-y-2 overflow-auto pr-1">
                {relationships.map((relationship) => (
                  <article key={relationship.id} className="rounded-2xl border border-slate-100 bg-slate-50 p-3">
                    <div className="flex items-start gap-2">
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-semibold text-slate-800">
                          {personName(people, relationship.pessoa_origem_id)} <span className="text-coral">—</span>{" "}
                          {personName(people, relationship.pessoa_destino_id)}
                        </p>
                        <p className="mt-1 text-xs text-slate-500">{relationship.tipo_vinculo}</p>
                      </div>
                      <button type="button" className="rounded-lg p-1.5 text-slate-400 hover:bg-white hover:text-teal-700" onClick={() => void editRelationship(relationship)} aria-label="Editar vínculo"><Edit3 className="size-3.5" /></button>
                      <button type="button" className="rounded-lg p-1.5 text-slate-400 hover:bg-rose-50 hover:text-rose-600" onClick={() => void deleteRelationship(relationship)} aria-label="Excluir vínculo"><Trash2 className="size-3.5" /></button>
                    </div>
                  </article>
                ))}
              </div>
            </section>
          )}
        </aside>

        <section className="panel overflow-hidden">
          <GraphToolbar
            graph={graph}
            categories={categories}
            search={search}
            category={category}
            depth={depth}
            layout={layout}
            focusedNode={focusedNode}
            onSearch={setSearch}
            onCategory={setCategory}
            onDepth={setDepth}
            onLayout={setLayout}
          />

          <div className="relative min-h-[42rem] bg-[radial-gradient(#D9E2E0_1px,transparent_1px)] [background-size:22px_22px]">
            {visibleGraph.nodes.length === 0 ? (
              <div className="p-5">
                <EmptyState
                  icon={<Network className="size-7" />}
                  title="Nenhum nó para exibir"
                  description={
                    graph.nodes.length === 0
                      ? "Cadastre pessoas e vínculos para construir sua teia."
                      : "Nenhuma pessoa corresponde aos filtros escolhidos."
                  }
                />
              </div>
            ) : (
              <GraphViewer
                graph={visibleGraph}
                layout={layout}
                focusedNodeId={focusedNode?.id ?? null}
                onEdgeClick={openEdge}
                onNodeDoubleClick={openPerson}
              />
            )}
            <div className="pointer-events-none absolute bottom-4 left-4 right-4 flex flex-wrap gap-2 text-[11px] text-slate-500">
              <span className="rounded-xl border border-white bg-white/88 px-3 py-2 shadow-sm backdrop-blur"><Move className="mr-1 inline size-3" /> Arraste os nós para reorganizar</span>
              <span className="rounded-xl border border-white bg-white/88 px-3 py-2 shadow-sm backdrop-blur"><Info className="mr-1 inline size-3" /> Linha: detalhes · duplo clique: perfil</span>
            </div>
          </div>
        </section>
      </div>

      <RelationshipDrawer
        edge={selectedEdge}
        nodes={graph.nodes}
        onClose={() => setSelectedEdge(null)}
        onUpdated={async (relationship) => {
          await loadGraphData();
          setSelectedEdge({
            id: relationship.id,
            source: relationship.pessoa_origem_id,
            target: relationship.pessoa_destino_id,
            label: relationship.tipo_vinculo,
            descricao: relationship.descricao,
          });
        }}
      />
    </div>
  );
}

interface RelationshipFormProps {
  people: PessoaResumo[];
  form: VinculoPayload;
  editing: boolean;
  saving: boolean;
  loadingAttachments: boolean;
  attachments: AnexoVinculo[];
  pendingFiles: File[];
  onChange: (value: VinculoPayload) => void;
  onPendingFiles: (files: File[]) => void;
  onDeleteAttachment: (attachment: AnexoVinculo) => void;
  onRenameAttachment: (attachment: AnexoVinculo) => void;
  onSubmit: (event: FormEvent) => void;
  onCancel: () => void;
}

function RelationshipForm({ people, form, editing, saving, loadingAttachments, attachments, pendingFiles, onChange, onPendingFiles, onDeleteAttachment, onRenameAttachment, onSubmit, onCancel }: RelationshipFormProps) {
  return (
    <section className="panel p-5">
      <div className="mb-4 flex items-center gap-2"><Plus className="size-5 text-coral" /><h2 className="font-display text-lg font-semibold">{editing ? "Editar vínculo" : "Novo vínculo"}</h2></div>
      {people.length < 2 ? (
        <div className="rounded-2xl bg-amber-50 p-4 text-sm text-amber-800">Cadastre pelo menos duas pessoas para criar uma conexão.</div>
      ) : (
        <form className="space-y-3" onSubmit={onSubmit}>
          <div><label className="field-label">Pessoa origem</label><select className="field" value={form.pessoa_origem_id || ""} onChange={(event) => onChange({ ...form, pessoa_origem_id: Number(event.target.value) })} required><option value="">Selecione...</option>{people.map((person) => <option key={person.id} value={person.id}>{person.nome}</option>)}</select></div>
          <div><label className="field-label">Pessoa destino</label><select className="field" value={form.pessoa_destino_id || ""} onChange={(event) => onChange({ ...form, pessoa_destino_id: Number(event.target.value) })} required><option value="">Selecione...</option>{people.map((person) => <option key={person.id} value={person.id}>{person.nome}</option>)}</select></div>
          <div><label className="field-label">Tipo de vínculo</label><input className="field" value={form.tipo_vinculo} onChange={(event) => onChange({ ...form, tipo_vinculo: event.target.value })} placeholder="Ex.: Sócio, Irmão" required /></div>
          <div><label className="field-label">Descrição</label><textarea className="field min-h-28 resize-y" value={form.descricao || ""} onChange={(event) => onChange({ ...form, descricao: event.target.value })} placeholder="Histórico e contexto desta relação..." /></div>
          <div>
            <label className="field-label">Anexos da relação</label>
            {loadingAttachments ? <p className="rounded-xl bg-slate-50 p-3 text-xs text-slate-400">Carregando anexos…</p> : (
              <RelationshipAttachmentEditor
                existing={attachments}
                pending={pendingFiles}
                disabled={saving}
                onPendingChange={onPendingFiles}
                onDeleteExisting={onDeleteAttachment}
                onRenameExisting={onRenameAttachment}
              />
            )}
          </div>
          <div className="flex gap-2">
            <Button className="flex-1" type="submit" loading={saving}>{editing ? <Save className="size-4" /> : <GitFork className="size-4" />}{editing ? "Salvar" : "Conectar"}</Button>
            {editing && <button className="icon-button" type="button" onClick={onCancel} aria-label="Cancelar edição"><X className="size-4" /></button>}
          </div>
        </form>
      )}
    </section>
  );
}

interface GraphToolbarProps {
  graph: GrafoResponse;
  categories: Categoria[];
  search: string;
  category: string;
  depth: number;
  layout: GraphLayout;
  focusedNode?: GrafoNode;
  onSearch: (value: string) => void;
  onCategory: (value: string) => void;
  onDepth: (value: number) => void;
  onLayout: (value: GraphLayout) => void;
}

function GraphToolbar({ graph, categories, search, category, depth, layout, focusedNode, onSearch, onCategory, onDepth, onLayout }: GraphToolbarProps) {
  return (
    <div className="space-y-3 border-b border-slate-100 p-4">
      <div className="grid gap-3 lg:grid-cols-[minmax(15rem,1fr)_13rem_11rem_auto]">
        <label className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
          <input className="field pl-9" list="graph-people" placeholder="Buscar e focar uma pessoa..." value={search} onChange={(event) => onSearch(event.target.value)} />
          <datalist id="graph-people">{graph.nodes.map((node) => <option key={node.id} value={node.label} />)}</datalist>
        </label>
        <select className="field" value={category} onChange={(event) => onCategory(event.target.value)}><option value="">Todas as categorias</option>{categories.map((item) => <option key={item.id} value={item.nome_categoria}>{item.nome_categoria}</option>)}</select>
        <label className="relative"><Filter className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-400" /><select className="field pl-9" value={depth} onChange={(event) => onDepth(Number(event.target.value))} disabled={!focusedNode} title={focusedNode ? "Nível de conexão" : "Busque uma pessoa para isolar conexões"}><option value={1}>1º grau</option><option value={2}>2º grau</option><option value={3}>3º grau</option></select></label>
        <div className="flex rounded-xl border border-slate-200 bg-slate-50 p-1">
          <button type="button" className={cn("flex items-center gap-1.5 rounded-lg px-3 py-2 text-xs font-semibold transition", layout === "force" ? "bg-white text-teal-800 shadow-sm" : "text-slate-500")} onClick={() => onLayout("force")} title="Layout em teia"><Orbit className="size-4" /> Teia</button>
          <button type="button" className={cn("flex items-center gap-1.5 rounded-lg px-3 py-2 text-xs font-semibold transition", layout === "hierarchical" ? "bg-white text-teal-800 shadow-sm" : "text-slate-500")} onClick={() => onLayout("hierarchical")} title="Layout hierárquico"><Workflow className="size-4" /> UML</button>
        </div>
      </div>
      <div className="flex min-h-7 flex-wrap items-center justify-between gap-2 text-xs text-slate-400">
        <span>{graph.nodes.length} pessoas · {graph.edges.length} vínculos</span>
        {focusedNode && <span className="inline-flex items-center gap-2 rounded-full bg-teal-50 px-3 py-1 font-medium text-teal-800"><span className="size-2 rounded-full" style={{ backgroundColor: focusedNode.color }} /> Foco: {focusedNode.label} · até {depth}º grau <button type="button" onClick={() => onSearch("")} aria-label="Limpar foco"><X className="size-3" /></button></span>}
      </div>
    </div>
  );
}

function personName(people: PessoaResumo[], id: number) {
  return people.find((person) => person.id === id)?.nome || `Pessoa #${id}`;
}

function normalize(value: string) {
  return value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLocaleLowerCase("pt-BR");
}

function findFocusedNode(nodes: GrafoNode[], search: string): GrafoNode | undefined {
  const term = normalize(search.trim());
  if (!term) return undefined;
  return nodes.find((node) => normalize(node.label) === term)
    || nodes.find((node) => normalize(node.label).startsWith(term))
    || nodes.find((node) => normalize(node.label).includes(term));
}

function filterByCategory(graph: GrafoResponse, category: string): GrafoResponse {
  if (!category) return graph;
  const nodeIds = new Set(graph.nodes.filter((node) => node.categoria === category).map((node) => node.id));
  return {
    nodes: graph.nodes.filter((node) => nodeIds.has(node.id)),
    edges: graph.edges.filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)),
  };
}

function isolateConnections(graph: GrafoResponse, focusId: number | null, depth: number): GrafoResponse {
  if (focusId === null) return graph;
  const visible = new Set([focusId]);
  let frontier = new Set([focusId]);

  for (let level = 0; level < depth; level += 1) {
    const next = new Set<number>();
    for (const edge of graph.edges) {
      if (frontier.has(edge.source) && !visible.has(edge.target)) next.add(edge.target);
      if (frontier.has(edge.target) && !visible.has(edge.source)) next.add(edge.source);
    }
    next.forEach((id) => visible.add(id));
    frontier = next;
  }

  return {
    nodes: graph.nodes.filter((node) => visible.has(node.id)),
    edges: graph.edges.filter((edge) => visible.has(edge.source) && visible.has(edge.target)),
  };
}
