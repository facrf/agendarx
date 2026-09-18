/* Developed with care by FACRF - https://github.com/facrf */
import { useId, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { Link } from "react-router-dom";
import type { ConfigHpPsicossocial, IndicadoresPsicossociais } from "../types/api";

export function percentual(value: number) {
  return `${(value * 100).toLocaleString("pt-BR", { maximumFractionDigits: 2 })}%`;
}

export function GraphVitalityBar({ hp }: { hp: number }) {
  const value = Math.max(0, Math.min(1, hp));
  const color = value >= 0.7 ? "#22C55E" : value >= 0.4 ? "#EAB308" : "#EF4444";
  return <div className="h-1 w-14 overflow-hidden rounded-full bg-slate-200" role="progressbar" aria-label="Vitalidade psicossocial · HP" aria-valuemin={0} aria-valuemax={100} aria-valuenow={value * 100} aria-valuetext={percentual(value)}>
    <div className="h-full rounded-full" style={{ width: `${value * 100}%`, backgroundColor: color }} />
  </div>;
}

export function VitalityBar({ hp, color, compact = false, label = "Vitalidade psicossocial · HP" }: {
  hp: number; color: string; compact?: boolean; label?: string;
}) {
  const value = Math.max(0, Math.min(1, hp));
  return <div className={compact ? "vitality-bar vitality-bar-compact" : "vitality-bar"}>
    {!compact && <span className="mb-1.5 block text-sm font-semibold text-ink">{label}</span>}
    <div className="flex items-center gap-2">
      <div className="vitality-segments" role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={value * 100} aria-valuetext={percentual(value)}>
        {Array.from({ length: 20 }, (_, index) => <span key={index} className="vitality-segment" aria-hidden="true"><span style={{ width: `${Math.max(0, Math.min(1, value * 20 - index)) * 100}%`, backgroundColor: color }} /></span>)}
      </div>
      <strong className="shrink-0 tabular-nums" style={{ color }}>{percentual(value)}</strong>
    </div>
  </div>;
}

export function RiskAura({ indicadores, children, square = false }: {
  indicadores: IndicadoresPsicossociais; children: ReactNode; square?: boolean;
}) {
  return <div className={`risk-aura ${indicadores.aura_pulsante ? "risk-aura-pulse" : ""} ${square ? "risk-aura-square" : ""}`} style={{ "--aura-color": indicadores.aura_cor_hex } as CSSProperties} title={`Aura de risco cadastrado: ${indicadores.aura_nome}. O HP considera o risco próprio e os vínculos.`} aria-label={`Aura de risco: ${indicadores.aura_nome}`}>
    {children}
  </div>;
}

export function ImpactDetails({ indicadores }: { indicadores: IndicadoresPsicossociais }) {
  const rows = indicadores.contribuicoes;
  const propria = indicadores.penalidade_propria ?? 0;
  const vinculos = indicadores.penalidade_direta + indicadores.penalidade_residual;
  const bruto = indicadores.hp_base - propria - vinculos;
  return <div className="space-y-3 text-sm">
    <p className="text-slate-500">A aura indica o risco cadastrado. O HP desconta esse risco próprio e os impactos recebidos das conexões.</p>
    <div className="rounded-xl bg-slate-50 p-3" aria-label="Cálculo do HP"><p>Base <strong>{percentual(indicadores.hp_base)}</strong> − risco próprio <strong>{percentual(propria)}</strong> − vínculos <strong>{percentual(vinculos)}</strong> = <strong>{percentual(bruto)}</strong>.</p><p className="mt-1">HP final: <strong>{percentual(indicadores.hp)}</strong>{bruto < indicadores.hp_min && " (limitado pelo piso configurado)"}.</p><p className="mt-1 text-xs text-slate-500">Vínculos: {percentual(indicadores.penalidade_direta)} diretos + {percentual(indicadores.penalidade_residual)} de 2º grau.</p></div>
    <p>Base: <strong>{percentual(indicadores.hp_base)}</strong> · piso: <strong>{percentual(indicadores.hp_min)}</strong> · perda aplicada: <strong>{percentual(indicadores.hp_base - indicadores.hp)}</strong>.</p>
    {!indicadores.calculo_ativo && <p className="rounded-xl bg-slate-100 p-3">Cálculo de impactos desativado nas configurações.</p>}
    {rows.length === 0 ? <p className="text-slate-500">Nenhum impacto recebido dos vínculos.</p> : <ul className="max-h-64 space-y-2 overflow-auto">
      {rows.map((row, index) => <li key={`${row.vinculo_id}-${row.grau}-${index}`} className="rounded-xl border border-slate-100 bg-slate-50 p-3">
        <div className="flex justify-between gap-3"><Link className="font-semibold text-teal-800 underline" to={`/pessoas/${row.fonte_id}`}>{row.fonte_nome}</Link><strong>{(row.penalidade * 100).toLocaleString("pt-BR", { maximumFractionDigits: 2 })} p.p.</strong></div>
        <p className="mt-1 text-xs text-slate-500">{row.tipo_vinculo} · intensidade = {row.toxicidade_fonte.toLocaleString("pt-BR")} · peso = {percentual(row.peso)} · {row.grau === 1 ? "impacto direto (1º grau)" : `residual (2º grau) via ${row.alvo_direto_nome}`}</p>
      </li>)}
    </ul>}
    {(indicadores.penalidade_propria ?? 0) + indicadores.penalidade_direta + indicadores.penalidade_residual > indicadores.hp_base - indicadores.hp_min && <p className="text-xs text-slate-500">O HP atingiu o piso. As contribuições exibem os valores brutos antes do limite.</p>}
  </div>;
}

export function PsychosocialStatus({ indicadores }: { indicadores: IndicadoresPsicossociais }) {
  const [open, setOpen] = useState(false);
  const id = useId();
  return <div className="relative mt-3 w-full max-w-xl" onMouseEnter={() => setOpen(true)} onMouseLeave={() => setOpen(false)} onKeyDown={(event) => { if (event.key === "Escape") setOpen(false); }} onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false); }}>
    <button type="button" className="block w-full rounded-lg text-left focus-visible:outline-2 focus-visible:outline-teal-700" aria-expanded={open} aria-controls={id} onFocus={() => setOpen(true)} onClick={() => setOpen(true)} onKeyDown={(event) => { if (event.key === "Escape") setOpen(false); }}>
      <VitalityBar hp={indicadores.hp} color={indicadores.vitalidade_cor_hex} />
      <span className="mt-1 block text-xs text-slate-500">{indicadores.vitalidade_nome} · Ver composição do impacto</span>
    </button>
    {open && <div id={id} className="absolute left-0 top-full z-20 w-full min-w-64 rounded-2xl border border-slate-200 bg-white p-4 shadow-xl" role="region" aria-label="Composição do impacto psicossocial"><ImpactDetails indicadores={indicadores} /></div>}
  </div>;
}

export function PsychosocialSummary({ indicadores }: { indicadores: IndicadoresPsicossociais }) {
  const metrics = [
    { name: "Risco próprio", value: indicadores.penalidade_propria ?? 0, color: "#EF4444" },
    { name: "Exposição direta", value: indicadores.penalidade_direta, color: "#EF4444" },
    { name: "Estresse secundário", value: indicadores.penalidade_residual, color: "#F97316" },
    { name: "Vitalidade", value: indicadores.hp, color: indicadores.vitalidade_cor_hex },
  ];
  return <section className="mb-6 space-y-4 border-b border-slate-100 pb-6" aria-label="Resumo psicossocial">
    <div><h2 className="font-display text-xl font-semibold">Resumo psicossocial</h2><p className="mt-1 text-sm text-slate-500">Risco próprio, exposições recebidas e vitalidade atual na rede ativa.</p></div>
    <div className="space-y-3">{metrics.map((metric) => <div key={metric.name} className="grid grid-cols-[minmax(7rem,10rem)_1fr_4.5rem] items-center gap-3 text-xs sm:text-sm"><span>{metric.name}</span><div className="h-3 overflow-hidden rounded-full bg-slate-100"><div className="h-full rounded-full" style={{ width: `${Math.min(1, metric.value) * 100}%`, backgroundColor: metric.color }} /></div><strong className="text-right tabular-nums">{percentual(metric.value)}</strong></div>)}</div>
    <details className="rounded-xl border border-slate-100 p-3"><summary className="cursor-pointer text-sm font-semibold text-teal-800">Quais vínculos contribuíram?</summary><div className="mt-3"><ImpactDetails indicadores={indicadores} /></div></details>
  </section>;
}

export function PsychosocialLegend({ config }: { config: ConfigHpPsicossocial }) {
  return <section className="space-y-3 border-b border-slate-100 p-4 text-xs" aria-label="Legenda psicossocial">
    <div className="flex flex-wrap items-center gap-4"><strong>Auras de risco comportamental:</strong>{[...config.faixas_aura].reverse().map((band) => <span key={band.min} className="inline-flex items-center gap-2"><span className={`aura-sample ${band.pulsante ? "risk-aura-pulse" : ""}`} style={{ "--aura-color": band.cor_hex } as CSSProperties} />{band.nome}{band.pulsante && <span className="text-slate-500">(pulsante)</span>}</span>)}</div>
    <div className="flex flex-wrap items-center gap-4"><strong>Barras do mapa · HP:</strong>{[{ hp: 1, label: "70% a 100%" }, { hp: 0.5, label: "40% a menos de 70%" }, { hp: 0.2, label: "Abaixo de 40%" }].map(({ hp, label }) => <span key={hp} className="inline-flex items-center gap-2"><GraphVitalityBar hp={hp} />{label}</span>)}</div>
    <p className="text-slate-500">Aura = risco cadastrado · barra = HP após risco próprio e impactos dos vínculos · 1º grau: linha contínua · 2º grau: linha tracejada. Selecione um nó para explorar até o 2º grau.</p>
  </section>;
}
