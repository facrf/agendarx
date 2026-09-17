/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { Plus, Save, Trash2 } from "lucide-react";
import { api, errorMessage } from "../services/api";
import { useToast } from "../contexts/ToastContext";
import type { ConfigHpPsicossocial, FaixaIndicador } from "../types/api";
import { Button } from "./ui";

export function PsychosocialSettings() {
  const [config, setConfig] = useState<ConfigHpPsicossocial | null>(null);
  const [weights, setWeights] = useState<{ tipo: string; peso: number }[]>([]);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);
  const { notify } = useToast();
  const load = () => api.get<ConfigHpPsicossocial>("/api/configuracoes/hp-psicossocial").then((value) => {
    setConfig(value); setWeights(Object.entries(value.pesos_vinculo).map(([tipo, peso]) => ({ tipo, peso }))); setFailed(false);
  }).catch((error) => { setFailed(true); notify(errorMessage(error), "erro"); });
  useEffect(() => { let active = true; api.get<ConfigHpPsicossocial>("/api/configuracoes/hp-psicossocial").then((value) => {
    if (active) { setConfig(value); setWeights(Object.entries(value.pesos_vinculo).map(([tipo, peso]) => ({ tipo, peso }))); }
  }).catch((error) => { if (active) { setFailed(true); notify(errorMessage(error), "erro"); } }); return () => { active = false; }; }, [notify]);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!config) return;
    const keys = weights.map((row) => row.tipo.trim().toLowerCase());
    if (keys.some((key) => !key) || new Set(keys).size !== keys.length) return notify("Preencha tipos únicos para os pesos de vínculo", "erro");
    setSaving(true);
    try {
      const { versao, ...parameters } = config;
      const saved = await api.put<ConfigHpPsicossocial>("/api/configuracoes/hp-psicossocial", {
        ...parameters, versao_esperada: versao, pesos_vinculo: Object.fromEntries(weights.map((row, index) => [keys[index], row.peso])),
      });
      setConfig(saved);
      setWeights(Object.entries(saved.pesos_vinculo).map(([tipo, peso]) => ({ tipo, peso })));
      notify("Parâmetros psicossociais atualizados");
    } catch (error) { notify(errorMessage(error), "erro"); } finally { setSaving(false); }
  };

  const fields = [
    ["toxicidade_min", "T mínimo"], ["toxicidade_max", "T máximo"],
    ["hp_base", "HP base"], ["hp_min", "HP mínimo"],
    ["fator_segundo_grau", "Fator de 2º grau"], ["peso_padrao", "Peso de vínculos sem correspondência"],
  ] as const;
  return <section className="panel p-5 sm:p-7">
    <h2 className="font-display text-xl font-semibold">HP psicossocial</h2>
    <p className="mt-1 text-sm text-slate-500">A aura usa T próprio. A barra usa o HP recebido dos vínculos. Parâmetros aplicados sem reiniciar.</p>
    {!config ? <div className="mt-4 text-sm">{failed ? <Button type="button" variant="secondary" onClick={() => void load()}>Tentar carregar parâmetros</Button> : "Carregando parâmetros…"}</div> : <form className="mt-5 space-y-5" onSubmit={submit}>
      <label className="flex items-center gap-2 text-sm font-semibold"><input type="checkbox" checked={config.ativo} onChange={(event) => setConfig({ ...config, ativo: event.target.checked })} /> Calcular impactos na vitalidade</label>
      <div className="grid gap-3 sm:grid-cols-2">{fields.map(([key, name]) => <label key={key} className="text-sm"><span className="field-label">{name}</span><input className="field" aria-label={name} type="number" min="0" max="1" step="any" required value={config[key]} onChange={(event) => setConfig({ ...config, [key]: Number(event.target.value) })} /><span className="mt-1 block text-xs text-slate-500">{(config[key] * 100).toLocaleString("pt-BR", { maximumFractionDigits: 2 })}%</span></label>)}</div>
      <fieldset className="space-y-2"><legend className="mb-2 text-sm font-semibold">Pesos por tipo de vínculo</legend>{weights.map((row, index) => <div key={index} className="flex items-center gap-2"><input className="field min-w-0 flex-1" aria-label={`Tipo de vínculo ${index + 1}`} required maxLength={120} value={row.tipo} onChange={(event) => setWeights(weights.map((value, i) => i === index ? { ...value, tipo: event.target.value } : value))} /><input className="field w-24" aria-label={`Peso do vínculo ${index + 1}`} required type="number" min="0" max="1" step="any" value={row.peso} onChange={(event) => setWeights(weights.map((value, i) => i === index ? { ...value, peso: Number(event.target.value) } : value))} /><button className="icon-button" type="button" aria-label={`Remover peso ${index + 1}`} onClick={() => setWeights(weights.filter((_, i) => i !== index))}><Trash2 className="size-4" /></button></div>)}<Button type="button" variant="secondary" onClick={() => setWeights([...weights, { tipo: "", peso: config.peso_padrao }])}><Plus className="size-4" /> Adicionar peso</Button></fieldset>
      <BandEditor title="Faixas da aura (T próprio)" rows={config.faixas_aura} onChange={(rows) => setConfig({ ...config, faixas_aura: rows })} allowPulse />
      <BandEditor title="Faixas da vitalidade (HP recebido)" rows={config.faixas_vitalidade} onChange={(rows) => setConfig({ ...config, faixas_vitalidade: rows })} />
      <p className="text-xs text-slate-500">Faixas começam em zero, em ordem crescente. Limites de T incompatíveis com perfis existentes são rejeitados. Versão atual: {config.versao}.</p>
      <div className="flex flex-wrap gap-2"><Button type="submit" loading={saving}><Save className="size-4" /> Salvar parâmetros psicossociais</Button><Button type="button" variant="secondary" disabled={saving} onClick={() => void load()}>Recarregar parâmetros</Button></div>
    </form>}
  </section>;
}

function BandEditor({ title, rows, onChange, allowPulse = false }: {
  title: string; rows: FaixaIndicador[]; onChange: (rows: FaixaIndicador[]) => void; allowPulse?: boolean;
}) {
  const patch = (index: number, value: Partial<FaixaIndicador>) => onChange(rows.map((row, i) => i === index ? { ...row, ...value } : row));
  return <fieldset className="space-y-2"><legend className="mb-2 text-sm font-semibold">{title}</legend>{rows.map((row, index) => <div key={index} className="flex flex-wrap items-center gap-2 rounded-xl border border-slate-100 p-2"><input className="field w-24" type="number" min="0" max="1" step="any" required aria-label={`${title}: limite ${index + 1}`} value={row.min} onChange={(event) => patch(index, { min: Number(event.target.value) })} /><input className="field min-w-24 flex-1" required maxLength={80} aria-label={`${title}: nome ${index + 1}`} value={row.nome} onChange={(event) => patch(index, { nome: event.target.value })} /><input className="size-10 cursor-pointer rounded-lg" type="color" aria-label={`${title}: cor ${index + 1}`} value={row.cor_hex} onChange={(event) => patch(index, { cor_hex: event.target.value })} />{allowPulse && <label className="flex items-center gap-1 text-xs"><input type="checkbox" checked={row.pulsante} onChange={(event) => patch(index, { pulsante: event.target.checked })} /> Pulsante</label>}<button className="icon-button" type="button" disabled={rows.length === 1} aria-label={`${title}: remover faixa ${index + 1}`} onClick={() => onChange(rows.filter((_, i) => i !== index))}><Trash2 className="size-4" /></button></div>)}<Button type="button" variant="secondary" disabled={rows.length >= 32} onClick={() => onChange([...rows, { min: Math.min(1, (rows.at(-1)?.min ?? 0) + 0.1), nome: "Nova faixa", cor_hex: "#94A3B8", pulsante: false }])}><Plus className="size-4" /> Adicionar faixa</Button></fieldset>;
}
