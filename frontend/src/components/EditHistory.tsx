/* Developed with care by FACRF - https://github.com/facrf */
import { useState } from "react";
import { api, errorMessage } from "../services/api";
import { Button } from "./ui";
interface Revision { id: number; autor_login: string; criado_em: string; anterior_json: string; versao_anterior: number }
interface History { versao: number; itens: Revision[] }
const detailFields = [["categoria_nome", "Categoria"], ["pessoa_juridica", "Pessoa jurídica"], ["inicio_em", "Início"], ["fim_em", "Fim"], ["status", "Status"], ["prioridade", "Prioridade"], ["lembrete_minutos", "Antecedência do lembrete (minutos)"], ["classificacao_risco", "Avaliação de risco"], ["toxicidade", "Intensidade"], ["risco_justificativa", "Justificativa da avaliação"], ["risco_revisado_em", "Data de revisão"]] as const;
export function EditHistory({ tipo, id, onRestored }: { tipo: "pessoa" | "tarefa" | "vinculo"; id: number; onRestored?: () => void }) {
  const [history, setHistory] = useState<History>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const load = async () => { setBusy(true); setError(""); try { setHistory(await api.get<History>(`/api/revisoes/${tipo}/${id}`)); } catch (e) { setError(errorMessage(e)); } finally { setBusy(false); } };
  const restore = async (r: Revision) => {
    if (!history || !window.confirm("Restaurar os dados desta versão? O estado atual ficará no histórico. Edições ainda não salvas serão descartadas.")) return;
    setBusy(true); setError("");
    try { await api.post(`/api/revisoes/${tipo}/${id}/${r.id}/restaurar`, { versao: history.versao }); if (onRestored) onRestored(); else window.location.reload(); }
    catch (e) { setError(errorMessage(e)); } finally { setBusy(false); }
  };
  return <details className="my-4 rounded-xl border p-4" onToggle={e => { if (e.currentTarget.open && !history && !busy) void load(); }}>
    <summary className="cursor-pointer font-semibold">Versões anteriores e desfazer</summary>
    <p className="my-2 text-sm text-slate-600">Restaure os dados do cadastro e seus contatos ou vínculos. Arquivos, fotos e etiquetas permanecem disponíveis. Revisões anteriores a uma mesclagem são somente para consulta.</p>
    <Button type="button" variant="secondary" disabled={busy} onClick={() => void load()}>Atualizar histórico de edições</Button>
    {error && <p role="alert" className="mt-2 text-rose-700">{error}</p>}
    {history?.itens.length === 0 && <p className="mt-3 text-sm">Nenhuma edição registrada ainda.</p>}
    <ol className="mt-3 space-y-3">{history?.itens.map(r => { const data = JSON.parse(r.anterior_json) as Record<string, unknown>; return <li key={r.id} className="rounded-lg bg-slate-50 p-3">
      <p className="text-sm">Antes da edição de {r.autor_login} · {new Date(r.criado_em).toLocaleString("pt-BR")}</p>
      <p className="font-medium">{String(data.nome ?? data.titulo ?? data.tipo_vinculo ?? "Registro")}</p>
      {typeof data.descricao === "string" && <p className="whitespace-pre-wrap text-sm">{data.descricao.slice(0, 1500)}</p>}
      {Array.isArray(data.contatos) && <p className="text-sm">Contatos: {data.contatos.map(c => String(c.valor)).join(" · ") || "nenhum"}</p>}
      <dl className="my-2 text-sm">{detailFields.filter(([key]) => key in data).map(([key,label]) => <div key={key} className="flex gap-2"><dt className="font-medium">{label}:</dt><dd>{data[key] == null ? "Não informado" : key === "pessoa_juridica" ? data[key] ? "Sim" : "Não" : key.endsWith("_em") && typeof data[key] === "string" ? new Date(data[key]).toLocaleString("pt-BR") : String(data[key])}</dd></div>)}</dl>
      <Button type="button" variant="secondary" disabled={busy} onClick={() => void restore(r)}>Restaurar esta versão</Button>
    </li>; })}</ol>
  </details>;
}
