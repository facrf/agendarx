/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { api, errorMessage } from "../services/api";
import type { AnexoVinculo, PessoaResumo, PessoaVinculo, VinculoPayload } from "../types/api";
import { useToast } from "../contexts/ToastContext";
import { useFormDraft } from "../hooks/useFormDraft";
import { useLinkedEvent } from "../hooks/useLinkedEvent";
import { DraftNotice } from "./DraftNotice";
import { LinkedEventFields } from "./LinkedEventFields";
import { RelationshipForm } from "./RelationshipForm";
import { Button, Modal, Spinner } from "./ui";

type FocusPerson = { id: number; nome: string };

export function CreateRelationshipDialog({ person, onClose }: { person: FocusPerson; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  return <Modal open title={`Criar vínculo · ${person.nome}`} onClose={() => { if (!busy) onClose(); }}>
    <RelationshipEditor person={person} onSaved={onClose} onCancel={onClose} onBusy={setBusy} />
  </Modal>;
}

export function RelationshipEditor({ person, relationship, active = true, draftName, onSaved, onCancel, onBusy }: {
  person?: FocusPerson;
  relationship?: PessoaVinculo;
  active?: boolean;
  draftName?: string;
  onSaved: () => void;
  onCancel: () => void;
  onBusy?: (busy: boolean) => void;
}) {
  const agenda = useLinkedEvent();
  const { notify } = useToast();
  const [people, setPeople] = useState<PessoaResumo[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [revision, setRevision] = useState(0);
  const [saving, setSaving] = useState(false);
  const submitting = useRef(false);
  const [editingId, setEditingId] = useState<number | null>(relationship?.id ?? null);
  const [attachments, setAttachments] = useState<AnexoVinculo[]>([]);
  const [pendingFiles, setPendingFiles] = useState<File[]>([]);
  const [form, setForm] = useState<VinculoPayload>(() => ({
    ...(relationship ? { versao: relationship.versao } : {}),
    pessoa_origem_id: relationship?.pessoa_origem_id ?? person?.id ?? 0,
    pessoa_destino_id: relationship?.pessoa_destino_id ?? 0,
    tipo_vinculo: relationship?.tipo_vinculo ?? "",
    descricao: relationship?.descricao ?? "",
  }));
  const formDraft = useFormDraft({
    name: draftName ?? `vinculo:criar:${person?.id ?? "geral"}`,
    value: { form, editingId, agenda: agenda.draft },
    enabled: active && people.length > 0 && !loadError,
    meaningful: Boolean(form.tipo_vinculo.trim() || form.descricao?.trim() || form.pessoa_destino_id),
    validate: item => (!person || item.form.pessoa_origem_id === person.id) && (!relationship || item.editingId === relationship.id),
    restore: item => { setForm(item.form); setEditingId(item.editingId); agenda.setDraft(item.agenda); },
  });

  useEffect(() => {
    if (!active || submitting.current) return;
    const controller = new AbortController();
    setLoading(true);
    setLoadError("");
    Promise.all([
      api.get<PessoaResumo[]>("/api/pessoas", { signal: controller.signal }),
      editingId ? api.get<AnexoVinculo[]>(`/api/vinculos/${editingId}/anexos`, { signal: controller.signal }) : Promise.resolve([]),
    ]).then(([items, files]) => { if (!controller.signal.aborted) { setPeople(items); setAttachments(files); } })
      .catch(error => { if (!controller.signal.aborted) setLoadError(errorMessage(error)); })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [active, editingId, revision]);

  const save = async (event: FormEvent) => {
    event.preventDefault();
    if (submitting.current) return;
    if (!form.pessoa_origem_id || !form.pessoa_destino_id || !form.tipo_vinculo.trim()) return notify("Selecione duas pessoas e informe o tipo de vínculo", "erro");
    if (form.pessoa_origem_id === form.pessoa_destino_id) return notify("Escolha pessoas diferentes", "erro");
    try { agenda.validate(); } catch (error) { return notify(errorMessage(error), "erro"); }
    submitting.current = true;
    setSaving(true); onBusy?.(true);
    try {
      const payload = { ...form, tipo_vinculo: form.tipo_vinculo.trim(), descricao: form.descricao?.trim() || null };
      const saved = editingId
        ? await api.put<PessoaVinculo>(`/api/vinculos/${editingId}`, payload)
        : await api.post<PessoaVinculo>("/api/vinculos", payload);
      // Retrying an attachment or calendar failure must update the saved relation.
      setEditingId(saved.id);
      setForm(current => ({ ...current, versao: saved.versao }));
      const uploads = await Promise.allSettled(pendingFiles.map(file => {
        const data = new FormData(); data.append("arquivo", file);
        return api.post<AnexoVinculo>(`/api/vinculos/${saved.id}/anexos`, data);
      }));
      const uploaded = uploads.flatMap(result => result.status === "fulfilled" ? [result.value] : []);
      const failed = pendingFiles.filter((_, index) => uploads[index].status === "rejected");
      setAttachments(items => [...items.filter(item => !uploaded.some(file => file.id === item.id)), ...uploaded]);
      setPendingFiles(failed);
      if (failed.length) { notify(`Vínculo salvo, mas ${failed.length} anexo(s) falharam. Salve novamente para reenviar.`, "erro"); return; }
      await agenda.save({ people: [saved.pessoa_origem_id, saved.pessoa_destino_id], title: saved.tipo_vinculo,
        references: [`Vínculo #${saved.id}: ${saved.tipo_vinculo}`, ...[...attachments, ...uploaded].map(a => `[${a.nome_arquivo.replaceAll("[", "").replaceAll("]", "")}](${a.url_stream})`)] });
      formDraft.clear();
      notify(editingId ? "Vínculo atualizado" : "Vínculo criado");
      onSaved();
    } catch (error) { notify(errorMessage(error), "erro"); }
    finally { submitting.current = false; setSaving(false); onBusy?.(false); }
  };

  const deleteAttachment = async (attachment: AnexoVinculo) => {
    if (!window.confirm(`Excluir “${attachment.nome_arquivo}” desta relação?`)) return;
    try {
      await api.delete(`/api/vinculos/anexos/${attachment.id}`);
      setAttachments(items => items.filter(item => item.id !== attachment.id));
      notify("Anexo excluído");
    } catch (error) { notify(errorMessage(error), "erro"); }
  };
  const renameAttachment = async (attachment: AnexoVinculo) => {
    const name = window.prompt("Novo nome do arquivo:", attachment.nome_arquivo)?.trim();
    if (!name || name === attachment.nome_arquivo) return;
    try {
      const updated = await api.put<AnexoVinculo>(`/api/vinculos/anexos/${attachment.id}`, { nome_arquivo: name });
      setAttachments(items => items.map(item => item.id === updated.id ? updated : item));
      notify("Anexo renomeado");
    } catch (error) { notify(errorMessage(error), "erro"); }
  };

  if (loading && !saving) return <Spinner label="Carregando pessoas e vínculo" />;
  if (loadError) return <div role="alert" className="rounded-xl bg-amber-50 p-4 text-sm">{loadError}<Button type="button" variant="secondary" className="mt-3" onClick={() => setRevision(value => value + 1)}>Tentar novamente</Button></div>;
  return <>
    <DraftNotice draft={formDraft} />
    <RelationshipForm
      fixedPerson={person}
      people={people} form={form} editing={editingId !== null} saving={saving} loadingAttachments={loading}
      attachments={attachments} pendingFiles={pendingFiles} onChange={setForm} onPendingFiles={setPendingFiles}
      onDeleteAttachment={attachment => void deleteAttachment(attachment)} onRenameAttachment={attachment => void renameAttachment(attachment)}
      onInvalidFiles={message => notify(message, "erro")} onSubmit={save} onCancel={onCancel}
      eventFields={<LinkedEventFields value={agenda.draft} onChange={agenda.setDraft} disabled={saving} />}
    />
  </>;
}
