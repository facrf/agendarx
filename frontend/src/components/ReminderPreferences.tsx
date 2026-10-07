/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { api, errorMessage } from "../services/api";
import { Button } from "./ui";
export interface ReminderPreference { silencioso_inicio: string | null; silencioso_fim: string | null }
export function quietNow(p: ReminderPreference, now = new Date()) {
  if (!p.silencioso_inicio || !p.silencioso_fim) return false;
  const time = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  return p.silencioso_inicio < p.silencioso_fim ? time >= p.silencioso_inicio && time < p.silencioso_fim : time >= p.silencioso_inicio || time < p.silencioso_fim;
}
export function ReminderPreferences() {
  const [start, setStart] = useState(""); const [end, setEnd] = useState("");
  const [busy, setBusy] = useState(false); const [message, setMessage] = useState("");
  useEffect(() => { void api.get<ReminderPreference>("/api/preferencias/lembretes").then(p => { setStart(p.silencioso_inicio || ""); setEnd(p.silencioso_fim || ""); }).catch(e => setMessage(errorMessage(e))); }, []);
  return <section className="panel p-5"><h2 className="font-display text-xl font-semibold">Horário silencioso</h2><p className="my-3 text-sm">Durante este intervalo, os lembretes ficam pendentes e serão apresentados depois. Os horários seguem o relógio local de cada dispositivo. Deixe ambos vazios para desativar.</p>
    <form onSubmit={async e => { e.preventDefault(); setBusy(true); try { await api.put("/api/preferencias/lembretes", { silencioso_inicio: start || null, silencioso_fim: end || null }); setMessage("Preferência salva"); } catch (err) { setMessage(errorMessage(err)); } finally { setBusy(false); } }}>
      <label className="field-label" htmlFor="quiet-start">Início do horário silencioso</label><input id="quiet-start" type="time" className="field" value={start} onChange={e => setStart(e.target.value)} />
      <label className="field-label mt-3" htmlFor="quiet-end">Fim do horário silencioso</label><input id="quiet-end" type="time" className="field" value={end} onChange={e => setEnd(e.target.value)} />
      <Button className="mt-3" loading={busy}>Salvar horário silencioso</Button>
    </form>{message && <p role="status" className="mt-3 text-sm">{message}</p>}
  </section>;
}
