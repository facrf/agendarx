/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { api, errorMessage } from "../services/api";
import type { ClassificacaoRisco, PreviaRisco } from "../types/api";
import { ImpactDetails, percentual, VitalityBar } from "./PsychosocialStatus";
import { Button } from "./ui";

export function RiskPreview({ pessoaId, nome, classificacao, intensidade, valid }: {
  pessoaId: number | null; nome: string; classificacao: ClassificacaoRisco; intensidade: number; valid: boolean;
}) {
  const [state, setState] = useState<{ key: string; value?: PreviaRisco; error?: string } | null>(null);
  const [retry, setRetry] = useState(0);
  const requestKey = JSON.stringify({ pessoa_id: pessoaId, nome, classificacao_risco: classificacao, toxicidade: intensidade });
  useEffect(() => {
    if (!valid) return;
    let active = true;
    const timer = window.setTimeout(() => {
      api.post<PreviaRisco>("/api/pessoas/risco/previa", JSON.parse(requestKey))
        .then(value => { if (active) setState({ key: requestKey, value }); })
        .catch(error => { if (active) setState({ key: requestKey, error: errorMessage(error) }); });
    }, 400);
    return () => { active = false; window.clearTimeout(timer); };
  }, [requestKey, valid, retry]);
  const current = state?.key === requestKey ? state : null;
  return <section className="space-y-3 rounded-xl border border-teal-100 bg-teal-50/50 p-4" aria-label="Prévia dos impactos">
    <h3 className="text-sm font-semibold text-teal-900">Prévia antes de salvar</h3>
    <p className="text-xs text-slate-500">Simulação com a rede atual. Não altera os cadastros. Os resultados podem mudar se outra sessão editar a rede antes de salvar.</p>
    {!valid ? <p className="text-sm">Informe uma intensidade dentro dos limites para calcular a prévia.</p>
      : current?.error ? <div role="alert" className="text-sm">Não foi possível calcular a prévia: {current.error} <Button type="button" variant="secondary" onClick={() => setRetry(value => value + 1)}>Tentar calcular novamente</Button></div>
      : !current?.value ? <p role="status" className="text-sm">Calculando prévia…</p>
      : <>
        <VitalityBar hp={current.value.pessoa.hp} color={current.value.pessoa.vitalidade_cor_hex} label="HP previsto desta pessoa" />
        <ImpactDetails indicadores={current.value.pessoa} />
        <h4 className="text-sm font-semibold">Pessoas com alteração no HP, composição ou aura</h4>
        {current.value.alteracoes.length === 0 ? <p className="text-sm">Nenhum indicador mudará com estes valores.</p> : <div className="max-h-64 overflow-auto"><table className="w-full text-left text-sm"><thead><tr><th className="p-2">Pessoa</th><th className="p-2">HP atual → previsto</th><th className="p-2">Aura atual → prevista</th></tr></thead><tbody>{current.value.alteracoes.map(row => <tr key={row.pessoa_id} className="border-t border-teal-100"><td className="p-2">{row.nome}</td><td className="whitespace-nowrap p-2">{row.hp_antes === null ? "Novo cadastro" : percentual(row.hp_antes)} → {percentual(row.hp_depois)}</td><td className="p-2">{row.aura_antes ?? "—"} → {row.aura_depois}</td></tr>)}</tbody></table></div>}
      </>}
  </section>;
}
