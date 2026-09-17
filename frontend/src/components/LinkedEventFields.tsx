/* Developed with care by FACRF - https://github.com/facrf */
import { useId } from "react";
import type { EventDraft } from "../hooks/useLinkedEvent";

export function LinkedEventFields({ value, onChange, disabled = false }: {
  value: EventDraft; onChange: (draft: EventDraft) => void; disabled?: boolean;
}) {
  const id = useId();
  return <fieldset disabled={disabled} className="space-y-3 rounded-xl border border-teal-100 bg-teal-50/50 p-3">
    <label className="flex items-center gap-2 text-sm font-semibold"><input type="checkbox" checked={value.enabled} onChange={(e) => onChange({ ...value, enabled: e.target.checked })} /> Criar evento na agenda (opcional)</label>
    {value.enabled && <>
      <p className="text-xs text-slate-500">O evento ficará associado às pessoas e terá referências aos registros e arquivos salvos.</p>
      <label className="block text-sm" htmlFor={`${id}-title`}>Título<input id={`${id}-title`} className="field" maxLength={160} value={value.title} placeholder="Usar título do registro" onChange={(e) => onChange({ ...value, title: e.target.value })} /></label>
      <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={value.allDay} onChange={(e) => onChange({ ...value, allDay: e.target.checked, start: "", end: "" })} /> Dia inteiro</label>
      <div className="grid gap-2 sm:grid-cols-2">
        <label className="text-sm" htmlFor={`${id}-start`}>Início<input id={`${id}-start`} className="field" type={value.allDay ? "date" : "datetime-local"} value={value.start} onChange={(e) => onChange({ ...value, start: e.target.value })} /></label>
        <label className="text-sm" htmlFor={`${id}-end`}>Término (opcional)<input id={`${id}-end`} className="field" type={value.allDay ? "date" : "datetime-local"} min={value.start} value={value.end} onChange={(e) => onChange({ ...value, end: e.target.value })} /></label>
      </div>
    </>}
  </fieldset>;
}
