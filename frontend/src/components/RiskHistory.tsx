/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { api, errorMessage } from "../services/api";
import type { HistoricoRisco, RegistroRisco } from "../types/api";
import { formatDate } from "../utils/format";
import { reviewDate, riskLabels } from "../utils/risk";
import { percentual } from "./PsychosocialStatus";
import { Button } from "./ui";

export function RiskHistory({ pessoaId }: { pessoaId: number }) {
  const [rows, setRows] = useState<HistoricoRisco[] | null>(null);
  const [error, setError] = useState("");
  const [retry, setRetry] = useState(0);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) return;
    let active = true;
    api.get<HistoricoRisco[]>(`/api/pessoas/${pessoaId}/risco/historico`)
      .then(value => { if (active) { setRows(value); setError(""); } })
      .catch(reason => { if (active) setError(errorMessage(reason)); });
    return () => { active = false; };
  }, [pessoaId, retry, open]);
  return <details className="rounded-xl border border-slate-200 p-3" onToggle={event => setOpen(event.currentTarget.open)}>
    <summary className="cursor-pointer text-sm font-semibold text-teal-800">Histórico de risco e revisões</summary>
    <div className="mt-3 space-y-3 text-sm">
      <p className="text-xs text-slate-500">Até 100 registros recentes. O histórico começa nesta atualização; registros anteriores não são reconstruídos.</p>
      {error ? <div role="alert">{error} <Button type="button" variant="secondary" onClick={() => setRetry(value => value + 1)}>Tentar carregar histórico</Button></div>
        : rows === null ? <p role="status">Carregando histórico…</p>
        : rows.length === 0 ? <p className="text-slate-500">Nenhuma alteração registrada.</p>
        : <ol className="max-h-96 space-y-3 overflow-auto">{rows.map(row => <li key={row.id} className="rounded-xl bg-slate-50 p-3">
          <p className="font-semibold">{row.autor_login} · {formatDate(row.registrado_em, true)}</p>
          <div className="mt-2 grid gap-3 sm:grid-cols-2">
            <div><p className="mb-1 text-xs font-semibold text-slate-500">Anterior</p>{row.anterior ? <Record value={row.anterior} /> : <p>Cadastro inicial</p>}</div>
            <div><p className="mb-1 text-xs font-semibold text-slate-500">Novo</p><Record value={row.novo} /></div>
          </div>
        </li>)}</ol>}
    </div>
  </details>;
}

function Record({ value }: { value: RegistroRisco }) {
  return <div className="space-y-1"><p>{riskLabels[value.classificacao_risco]} · {percentual(value.toxicidade)}</p><p className="text-xs text-slate-500">Revisão: {reviewDate(value.revisado_em)}</p><p className="whitespace-pre-wrap break-words">{value.justificativa || "Sem justificativa registrada."}</p></div>;
}

export function RiskRecord({ registro, pessoaId }: { registro?: RegistroRisco; pessoaId: number }) {
  return <section className="my-4 space-y-3" aria-label="Registro de risco psicossocial">
    <h3 className="text-sm font-semibold">Risco cadastrado e revisão</h3>
    {registro && <Record value={registro} />}
    <RiskHistory key={pessoaId} pessoaId={pessoaId} />
  </section>;
}
