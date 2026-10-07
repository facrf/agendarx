import { quietNow } from "./ReminderPreferences";
import type { ReminderPreference } from "./ReminderPreferences";
import { useEffect } from "react";
import { api } from "../services/api";
import type { TarefaCalendario } from "../types/api";
import { useAuth } from "../contexts/AuthContext";
import { useToast } from "../contexts/ToastContext";

export const NOTIFICACOES_TAREFAS_KEY = "agendarx:notificacoes-tarefas";

export function TaskReminderWatcher() {
  const { notify } = useToast();
  const { usuario } = useAuth();

  useEffect(() => {
    if (!usuario?.id) return;
    let ativo = true;
    const notificados = new Set<string>();
    const controller = new AbortController();
    let timer: number | undefined;

    const verificar = async () => {
      try {
        const preference = await api.get<ReminderPreference>("/api/preferencias/lembretes", { signal: controller.signal });
        if (!ativo || quietNow(preference)) return;
        const tarefas = await api.get<TarefaCalendario[]>("/api/calendario/lembretes", { signal: controller.signal });
        const chaves = new Set(tarefas.map(tarefa => reminderKey(tarefa)));
        for (const key of notificados) if (!chaves.has(key)) notificados.delete(key);
        for (const tarefa of tarefas) {
          if (!ativo) return;
          const key = reminderKey(tarefa);
          if (!notificados.has(key)) {
            notificados.add(key);
            const mensagem = `${tarefa.titulo} — ${formatarMomento(tarefa)}`;
            notify(`Lembrete: ${mensagem}`, "aviso", { rotulo: "Adiar por 15 minutos", executar: () => { void api.post(`/api/preferencias/lembretes/${tarefa.id}/adiar`, { minutos: 15, versao: tarefa.versao }).then(() => notify("Lembrete adiado por 15 minutos")).catch(e => notify(e instanceof Error ? e.message : "Não foi possível adiar", "erro")); } });
            notificarNavegador(tarefa, mensagem);
          }
          try {
            await api.patch(`/api/calendario/lembretes/${tarefa.id}/dispensar`, { versao: tarefa.versao, inicio_em: tarefa.inicio_em, lembrete_minutos: tarefa.lembrete_minutos, data_atualizacao: tarefa.data_atualizacao, lembrete_adiado_ate: tarefa.lembrete_adiado_ate ?? null });
          } catch { /* Retry acknowledgement without repeating the displayed notification. */ }
        }
      } catch {
        // O observador é silencioso para não poluir a interface em falhas transitórias.
      } finally {
        if (ativo) timer = window.setTimeout(verificar, 60_000);
      }
    };

    timer = window.setTimeout(verificar, 1_500);
    return () => {
      ativo = false;
      controller.abort();
      if (timer) window.clearTimeout(timer);
    };
  }, [notify, usuario]);

  return null;
}

function notificarNavegador(tarefa: TarefaCalendario, mensagem: string) {
  if (!("Notification" in window)) return;
  if (localStorage.getItem(NOTIFICACOES_TAREFAS_KEY) !== "true") return;
  if (Notification.permission !== "granted") return;
  const notificacao = new Notification("AgendarX · Lembrete", {
    body: mensagem,
    icon: "/api/identidade/icone",
    tag: `agendarx-tarefa-${tarefa.id}`,
  });
  notificacao.onclick = () => {
    window.focus();
    window.location.assign(`/calendario?tarefa=${tarefa.id}`);
    notificacao.close();
  };
}

function formatarMomento(tarefa: TarefaCalendario) {
  if (tarefa.dia_inteiro) {
    const [ano, mes, dia] = tarefa.inicio_em.slice(0, 10).split("-").map(Number);
    return new Date(ano, mes - 1, dia).toLocaleDateString("pt-BR", {
      day: "2-digit",
      month: "long",
    });
  }
  return new Date(tarefa.inicio_em).toLocaleString("pt-BR", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function reminderKey(tarefa: TarefaCalendario) {
  return `${tarefa.id}:${tarefa.inicio_em}:${tarefa.lembrete_minutos}:${tarefa.data_atualizacao}:${tarefa.versao ?? ""}:${tarefa.lembrete_adiado_ate ?? ""}`;
}
