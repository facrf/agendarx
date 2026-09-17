import { useRef, useState } from "react";
import { api, errorMessage } from "../services/api";
import type { TarefaCalendario } from "../types/api";

export interface EventDraft { enabled: boolean; title: string; start: string; end: string; allDay: boolean }
export interface EventContext { people: number[]; title: string; references: string[] }
const empty = (): EventDraft => ({ enabled: false, title: "", start: "", end: "", allDay: false });

export function useLinkedEvent() {
  const [draft, setDraft] = useState<EventDraft>(empty);
  const busy = useRef(false);
  const validate = () => {
    if (!draft.enabled) return;
    const start = new Date(draft.allDay ? `${draft.start}T00:00:00Z` : draft.start);
    const end = draft.end ? new Date(draft.allDay ? `${draft.end}T00:00:00Z` : draft.end) : null;
    if (!draft.start || !Number.isFinite(start.getTime()) || (end && (!Number.isFinite(end.getTime()) || end <= start))) {
      throw new Error("Informe uma data válida para o evento e um término posterior ao início.");
    }
  };
  const save = async (context: EventContext) => {
    if (!draft.enabled || busy.current) return;
    validate();
    busy.current = true;
    try {
      const event = await api.post<TarefaCalendario>("/api/calendario/tarefas", {
        titulo: (draft.title.trim() || context.title).slice(0, 160),
        descricao: context.references.join("\n\n").slice(0, 5000),
        inicio_em: new Date(draft.allDay ? `${draft.start}T00:00:00Z` : draft.start).toISOString(),
        fim_em: draft.end ? new Date(draft.allDay ? `${draft.end}T00:00:00Z` : draft.end).toISOString() : null,
        dia_inteiro: draft.allDay, status: "PENDENTE", prioridade: "NORMAL", cor_hex: "#13716D",
        pessoas_ids: [...new Set(context.people)], recorrencia: "NENHUMA", recorrencia_fim_em: null, lembrete_minutos: null,
      });
      setDraft(empty());
      return event;
    } catch (error) {
      throw new Error(`Dados salvos, mas o evento não foi criado: ${errorMessage(error)}. Tente agendar novamente.`, { cause: error });
    } finally { busy.current = false; }
  };
  return { draft, setDraft, validate, save, reset: () => setDraft(empty()) };
}
