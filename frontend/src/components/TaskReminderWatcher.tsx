import { useEffect } from "react";
import { api } from "../services/api";
import type { TarefaCalendario } from "../types/api";
import { useToast } from "../contexts/ToastContext";

export const NOTIFICACOES_TAREFAS_KEY = "agendarx:notificacoes-tarefas";
const processados = new Set<number>();

export function TaskReminderWatcher() {
  const { notify } = useToast();

  useEffect(() => {
    let ativo = true;
    let timer: number | undefined;

    const verificar = async () => {
      try {
        const tarefas = await api.get<TarefaCalendario[]>("/api/calendario/lembretes");
        for (const tarefa of tarefas) {
          if (!ativo || processados.has(tarefa.id)) continue;
          processados.add(tarefa.id);
          const mensagem = `${tarefa.titulo} — ${formatarMomento(tarefa)}`;
          notify(`Lembrete: ${mensagem}`, "aviso");
          notificarNavegador(tarefa, mensagem);
          await api.patch(`/api/calendario/lembretes/${tarefa.id}/dispensar`);
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
      if (timer) window.clearTimeout(timer);
    };
  }, [notify]);

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
