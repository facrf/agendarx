/* Developed with care by FACRF - https://github.com/facrf */
import { GitFork, Plus, Save, X } from "lucide-react";
import { useId, useState } from "react";
import type { FormEvent } from "react";
import type { AnexoVinculo, PessoaResumo, VinculoPayload } from "../types/api";
import { RelationshipAttachmentEditor } from "./RelationshipMedia";
import { Button } from "./ui";

interface RelationshipFormProps {
  fixedPerson?: { id: number; nome: string };
  eventFields: React.ReactNode;
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
  onInvalidFiles: (message: string) => void;
  onSubmit: (event: FormEvent) => void;
  onCancel: () => void;
}

export function RelationshipForm({ fixedPerson, eventFields, people, form, editing, saving, loadingAttachments, attachments, pendingFiles, onChange, onPendingFiles, onDeleteAttachment, onRenameAttachment, onInvalidFiles, onSubmit, onCancel }: RelationshipFormProps) {
  const id = useId();
  const [search, setSearch] = useState("");
  const normalize = (value: string) => value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLocaleLowerCase("pt-BR");
  const destinations = people.filter(person => person.id !== form.pessoa_origem_id && (person.id === form.pessoa_destino_id || normalize(person.nome).includes(normalize(search.trim()))));
  const source = people.find(person => person.id === form.pessoa_origem_id);
  const target = people.find(person => person.id === form.pessoa_destino_id);
  return (
    <section className="panel p-5">
      <div className="mb-4 flex items-center gap-2"><Plus className="size-5 text-coral" /><h2 className="font-display text-lg font-semibold">{editing ? "Editar vínculo" : "Novo vínculo"}</h2></div>
      {people.length < 2 ? (
        <div className="rounded-2xl bg-amber-50 p-4 text-sm text-amber-800">Cadastre pelo menos duas pessoas para criar uma conexão.</div>
      ) : (
        <form className="space-y-3" onSubmit={onSubmit}>
          <fieldset disabled={saving} className="space-y-3">
          {fixedPerson ? <div className="rounded-xl border border-teal-200 bg-teal-50 p-3"><p className="text-xs font-medium text-teal-800">Pessoa origem · selecionada · #{fixedPerson.id}</p><p className="mt-1 font-semibold text-teal-950">{fixedPerson.nome}</p><p className="mt-1 text-xs text-teal-800">Escolha a outra pessoa e informe o tipo de vínculo.</p></div> :
            <div><label htmlFor={`${id}-source`} className="field-label">Pessoa origem</label><select id={`${id}-source`} className="field" value={form.pessoa_origem_id || ""} onChange={event => { const sourceId = Number(event.target.value); onChange({ ...form, pessoa_origem_id: sourceId, pessoa_destino_id: form.pessoa_destino_id === sourceId ? 0 : form.pessoa_destino_id }); }} required><option value="">Selecione...</option>{people.map(person => <option key={person.id} value={person.id}>{person.nome} · #{person.id}</option>)}</select></div>}
          <div><label htmlFor={`${id}-search`} className="field-label">Buscar outra pessoa</label><input id={`${id}-search`} type="search" className="field" placeholder="Digite o nome para filtrar as opções" value={search} onChange={event => setSearch(event.target.value)} /></div>
          <div><label htmlFor={`${id}-target`} className="field-label">Pessoa destino</label><select id={`${id}-target`} className="field" value={form.pessoa_destino_id || ""} onChange={event => onChange({ ...form, pessoa_destino_id: Number(event.target.value) })} required><option value="">Selecione a outra pessoa...</option>{destinations.map(person => <option key={person.id} value={person.id}>{person.nome} · {person.nome_categoria || "Sem categoria"} · #{person.id}</option>)}</select>{destinations.length === 0 && <p role="status" className="mt-1 text-xs text-slate-500">Nenhuma outra pessoa encontrada. Tente outro nome.</p>}</div>
          <div><label htmlFor={`${id}-type`} className="field-label">Tipo de vínculo</label><input id={`${id}-type`} className="field" value={form.tipo_vinculo} onChange={event => onChange({ ...form, tipo_vinculo: event.target.value })} placeholder="Ex.: Sócio, Irmão" maxLength={255} required /></div>
          {source && target && <p className="rounded-xl bg-slate-50 p-3 text-sm text-slate-600" aria-live="polite"><strong>{source.nome}</strong> → <strong>{target.nome}</strong>{form.tipo_vinculo.trim() && ` · ${form.tipo_vinculo.trim()}`}</p>}
          <details className="rounded-xl border border-slate-200 p-3" open={editing || undefined}>
            <summary className="cursor-pointer text-sm font-semibold text-teal-800">Descrição, anexos e evento (opcional)</summary>
            <div className="mt-4 space-y-4">
          <div><label htmlFor={`${id}-description`} className="field-label">Descrição (Markdown)</label><textarea id={`${id}-description`} className="field min-h-32 resize-y font-mono" value={form.descricao || ""} onChange={event => onChange({ ...form, descricao: event.target.value })} placeholder="Histórico e contexto desta relação..." /></div>
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
                onInvalidFiles={onInvalidFiles}
              />
            )}
          </div>
          {eventFields}
            </div>
          </details>
          </fieldset>
          <div className="sticky bottom-0 flex flex-wrap gap-2 border-t border-slate-100 bg-white py-3">
            <Button className="flex-1" type="submit" loading={saving}>{editing ? <Save className="size-4" /> : <GitFork className="size-4" />}{editing ? "Salvar alterações" : "Criar vínculo"}</Button>
            <Button variant="secondary" type="button" disabled={saving} onClick={onCancel}><X className="size-4" />Cancelar</Button>
          </div>
        </form>
      )}
    </section>
  );
}

