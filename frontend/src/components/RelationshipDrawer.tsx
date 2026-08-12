import { ArrowRight, Edit3, GitFork, Paperclip, Save, Tag, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type {
  AnexoVinculo,
  GrafoEdge,
  GrafoNode,
  PessoaVinculo,
  VinculoPayload,
} from "../types/api";
import { RelationshipAttachmentEditor, RelationshipMediaList } from "./RelationshipMedia";
import { Button } from "./ui";

interface RelationshipDrawerProps {
  edge: GrafoEdge | null;
  nodes: GrafoNode[];
  onClose: () => void;
  onUpdated: (relationship: PessoaVinculo) => void | Promise<void>;
}

export function RelationshipDrawer({ edge, nodes, onClose, onUpdated }: RelationshipDrawerProps) {
  const [attachments, setAttachments] = useState<AnexoVinculo[]>([]);
  const [pendingFiles, setPendingFiles] = useState<File[]>([]);
  const [attachmentError, setAttachmentError] = useState("");
  const [loadingAttachments, setLoadingAttachments] = useState(false);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState<VinculoPayload>({
    pessoa_origem_id: 0,
    pessoa_destino_id: 0,
    tipo_vinculo: "",
    descricao: "",
  });
  const { notify } = useToast();

  useEffect(() => {
    if (!edge) return;
    const closeOnEscape = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [edge, onClose]);

  useEffect(() => {
    if (!edge) {
      setAttachments([]);
      setPendingFiles([]);
      setEditing(false);
      return;
    }
    setForm({
      pessoa_origem_id: edge.source,
      pessoa_destino_id: edge.target,
      tipo_vinculo: edge.label,
      descricao: edge.descricao || "",
    });
    setEditing(false);
    setPendingFiles([]);
    let active = true;
    setLoadingAttachments(true);
    setAttachmentError("");
    api.get<AnexoVinculo[]>(`/api/vinculos/${edge.id}/anexos`)
      .then((items) => { if (active) setAttachments(items); })
      .catch((error) => { if (active) setAttachmentError(errorMessage(error)); })
      .finally(() => { if (active) setLoadingAttachments(false); });
    return () => { active = false; };
  }, [edge]);

  if (!edge) return null;
  const source = nodes.find((node) => node.id === form.pessoa_origem_id);
  const target = nodes.find((node) => node.id === form.pessoa_destino_id);

  const save = async (event: FormEvent) => {
    event.preventDefault();
    if (!form.pessoa_origem_id || !form.pessoa_destino_id || !form.tipo_vinculo.trim()) {
      return notify("Selecione duas pessoas e informe o tipo de vínculo", "erro");
    }
    if (form.pessoa_origem_id === form.pessoa_destino_id) {
      return notify("Escolha pessoas diferentes", "erro");
    }
    setSaving(true);
    try {
      const updated = await api.put<PessoaVinculo>(`/api/vinculos/${edge.id}`, {
        ...form,
        tipo_vinculo: form.tipo_vinculo.trim(),
        descricao: form.descricao?.trim() || null,
      });
      const uploads = await Promise.allSettled(
        pendingFiles.map((file) => {
          const data = new FormData();
          data.append("arquivo", file);
          return api.post<AnexoVinculo>(`/api/vinculos/${edge.id}/anexos`, data);
        }),
      );
      const uploaded = uploads.flatMap((result) => result.status === "fulfilled" ? [result.value] : []);
      const failed = pendingFiles.filter((_, index) => uploads[index]?.status === "rejected");
      setAttachments((items) => [...uploaded, ...items]);
      setPendingFiles(failed);
      await onUpdated(updated);
      if (failed.length > 0) {
        notify(`Relação salva, mas ${failed.length} anexo(s) falharam`, "erro");
      } else {
        setEditing(false);
        notify("Relação e arquivos atualizados");
      }
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSaving(false);
    }
  };

  const deleteAttachment = async (attachment: AnexoVinculo) => {
    if (!window.confirm(`Excluir “${attachment.nome_arquivo}” desta relação?`)) return;
    try {
      await api.delete(`/api/vinculos/anexos/${attachment.id}`);
      setAttachments((items) => items.filter((item) => item.id !== attachment.id));
      notify("Anexo excluído");
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const renameAttachment = async (attachment: AnexoVinculo) => {
    const name = window.prompt("Novo nome do arquivo:", attachment.nome_arquivo)?.trim();
    if (!name || name === attachment.nome_arquivo) return;
    try {
      const updated = await api.put<AnexoVinculo>(`/api/vinculos/anexos/${attachment.id}`, { nome_arquivo: name });
      setAttachments((items) => items.map((item) => item.id === updated.id ? updated : item));
      notify("Anexo renomeado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const cancelEditing = () => {
    setForm({
      pessoa_origem_id: edge.source,
      pessoa_destino_id: edge.target,
      tipo_vinculo: edge.label,
      descricao: edge.descricao || "",
    });
    setPendingFiles([]);
    setEditing(false);
  };

  return (
    <div className="fixed inset-0 z-50" role="dialog" aria-modal="true" aria-label="Detalhes do vínculo">
      <button type="button" className="absolute inset-0 bg-slate-950/40 backdrop-blur-[2px]" onClick={onClose} aria-label="Fechar detalhes" />
      <aside className="animate-drawer absolute inset-y-0 right-0 flex w-full max-w-xl flex-col bg-white shadow-2xl">
        <header className="flex items-center justify-between gap-3 border-b border-slate-100 px-5 py-5 sm:px-6">
          <div className="flex min-w-0 items-center gap-3">
            <div className="grid size-10 shrink-0 place-items-center rounded-2xl bg-coral/10 text-coral"><GitFork className="size-5" /></div>
            <div className="min-w-0"><p className="text-xs font-bold uppercase tracking-[0.16em] text-teal-700">Relação</p><h2 className="truncate font-display text-xl font-semibold text-slate-950">{editing ? "Editar vínculo" : edge.label}</h2></div>
          </div>
          <div className="flex items-center gap-2">
            {!editing && <Button type="button" variant="secondary" onClick={() => setEditing(true)}><Edit3 className="size-4" /> Editar</Button>}
            <button type="button" className="icon-button" onClick={onClose} aria-label="Fechar"><X className="size-5" /></button>
          </div>
        </header>

        <div className="flex-1 overflow-y-auto p-5 sm:p-6">
          {editing ? (
            <form className="space-y-5" onSubmit={save}>
              <div className="grid gap-3 sm:grid-cols-2">
                <div><label className="field-label" htmlFor="drawer-source">Pessoa origem</label><select id="drawer-source" className="field" value={form.pessoa_origem_id || ""} onChange={(event) => setForm({ ...form, pessoa_origem_id: Number(event.target.value) })} required><option value="">Selecione…</option>{nodes.map((node) => <option key={node.id} value={node.id}>{node.label}</option>)}</select></div>
                <div><label className="field-label" htmlFor="drawer-target">Pessoa destino</label><select id="drawer-target" className="field" value={form.pessoa_destino_id || ""} onChange={(event) => setForm({ ...form, pessoa_destino_id: Number(event.target.value) })} required><option value="">Selecione…</option>{nodes.map((node) => <option key={node.id} value={node.id}>{node.label}</option>)}</select></div>
              </div>
              <div><label className="field-label" htmlFor="drawer-type">Tipo de vínculo</label><input id="drawer-type" className="field" value={form.tipo_vinculo} onChange={(event) => setForm({ ...form, tipo_vinculo: event.target.value })} maxLength={255} required /></div>
              <div><label className="field-label" htmlFor="drawer-description">Histórico e descrição</label><textarea id="drawer-description" className="field min-h-40 resize-y" value={form.descricao || ""} onChange={(event) => setForm({ ...form, descricao: event.target.value })} placeholder="Histórico e contexto desta relação…" /></div>
              <div>
                <div className="mb-2 flex items-center gap-2"><Paperclip className="size-4 text-teal-700" /><h3 className="text-sm font-semibold text-slate-800">Adicionar e editar arquivos</h3></div>
                {loadingAttachments ? <p className="rounded-2xl bg-slate-50 p-4 text-sm text-slate-400">Carregando anexos…</p> : (
                  <RelationshipAttachmentEditor
                    existing={attachments}
                    pending={pendingFiles}
                    disabled={saving}
                    onPendingChange={setPendingFiles}
                    onDeleteExisting={(attachment) => void deleteAttachment(attachment)}
                    onRenameExisting={(attachment) => void renameAttachment(attachment)}
                  />
                )}
              </div>
              <div className="sticky bottom-0 flex gap-2 border-t border-slate-100 bg-white py-4">
                <Button className="flex-1" type="submit" loading={saving}><Save className="size-4" /> Salvar relação</Button>
                <Button type="button" variant="ghost" disabled={saving} onClick={cancelEditing}>Cancelar</Button>
              </div>
            </form>
          ) : (
            <>
              <div className="flex items-center gap-3 rounded-2xl border border-teal-100 bg-teal-50/70 p-4">
                <PersonBadge node={source} />
                <ArrowRight className="size-5 shrink-0 text-coral" />
                <PersonBadge node={target} />
              </div>

              <div className="mt-6">
                <div className="mb-3 flex items-center gap-2"><Tag className="size-4 text-teal-700" /><h3 className="text-sm font-semibold text-slate-800">Histórico e descrição</h3></div>
                <div className="min-h-32 whitespace-pre-wrap rounded-2xl border border-slate-200 bg-slate-50 p-4 text-sm leading-7 text-slate-600">
                  {edge.descricao || "Nenhuma descrição detalhada foi registrada para esta relação."}
                </div>
              </div>

              <div className="mt-6">
                <div className="mb-3 flex items-center gap-2"><Paperclip className="size-4 text-teal-700" /><h3 className="text-sm font-semibold text-slate-800">Fotos, áudios e arquivos</h3></div>
                {loadingAttachments ? (
                  <p className="rounded-2xl bg-slate-50 p-4 text-sm text-slate-400">Carregando anexos…</p>
                ) : attachmentError ? (
                  <p className="rounded-2xl bg-rose-50 p-4 text-sm text-rose-700">{attachmentError}</p>
                ) : attachments.length > 0 ? (
                  <RelationshipMediaList attachments={attachments} />
                ) : (
                  <p className="rounded-2xl border border-dashed border-slate-200 p-4 text-sm text-slate-400">Nenhum anexo registrado nesta relação.</p>
                )}
              </div>
            </>
          )}
        </div>
      </aside>
    </div>
  );
}

function PersonBadge({ node }: { node?: GrafoNode }) {
  const color = node?.color || "#86A6A3";
  return (
    <div className="min-w-0 flex-1 text-center">
      <div className="mx-auto grid size-11 place-items-center rounded-full border-[3px] bg-white text-sm font-bold uppercase" style={{ borderColor: color, color }}>
        {node?.label.slice(0, 2) || "?"}
      </div>
      <p className="mt-2 truncate text-sm font-semibold text-slate-900">{node?.label || "Pessoa removida"}</p>
      <p className="truncate text-[11px] text-slate-400">{node?.categoria || "Sem categoria"}</p>
    </div>
  );
}
