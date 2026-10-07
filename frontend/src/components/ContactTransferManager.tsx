/* Developed with care by FACRF - https://github.com/facrf */
import { useRef, useState } from "react";
import type { ChangeEvent } from "react";
import { ContactRound, Download, Upload } from "lucide-react";
import { api, apiUrl, errorMessage } from "../services/api";
import { useToast } from "../contexts/ToastContext";
import { Button } from "./ui";

type Acao = "ignorar" | "criar" | "atualizar";
interface Registro {
  indice: number; contato: { nome: string; categoria: string | null; campos: Array<{ tipo: string; valor: string }> };
  coincidencias: Array<{ pessoa_id: number; nome: string }>; repetidos_no_arquivo: number[]; acao_sugerida: Acao;
}
interface Previa { token: string; expira_em: number; registros: Registro[]; avisos: string[]; registros_ignorados: number }
interface Decisao { indice: number; acao: Acao; pessoa_id?: number }
interface Resultado { pessoas_importadas: number; pessoas_atualizadas: number; contatos_importados: number; registros_ignorados: number; avisos: string[] }

export function ContactTransferManager() {
  const [busy, setBusy] = useState(false);
  const [previa, setPrevia] = useState<Previa | null>(null);
  const [decisoes, setDecisoes] = useState<Decisao[]>([]);
  const [resultado, setResultado] = useState<Resultado | null>(null);
  const [pagina, setPagina] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const { notify } = useToast();
  const preparar = async (event: ChangeEvent<HTMLInputElement>) => {
    const input = event.target; const arquivo = input.files?.[0];
    if (!arquivo) return;
    setBusy(true); setResultado(null);
    try {
      const form = new FormData(); form.append("arquivo", arquivo);
      const data = await api.post<Previa>("/api/configuracoes/contatos/importar/previa", form);
      setPrevia(data); setPagina(0);
      setDecisoes(data.registros.map(r => ({ indice: r.indice, acao: r.acao_sugerida })));
      notify("Prévia pronta; revise antes de importar");
    } catch (error) { notify(errorMessage(error), "erro"); }
    finally { setBusy(false); input.value = ""; }
  };
  const confirmar = async () => {
    if (!previa) return;
    setBusy(true);
    try {
      const data = await api.post<Resultado>(`/api/configuracoes/contatos/importacoes/${previa.token}/confirmar`, { decisoes });
      setResultado(data); setPrevia(null); setDecisoes([]);
      notify("Importação concluída");
    } catch (error) { notify(errorMessage(error), "erro"); }
    finally { setBusy(false); }
  };
  const descartar = async () => {
    if (!previa) return;
    setBusy(true);
    try { await api.delete(`/api/configuracoes/contatos/importacoes/${previa.token}`); setPrevia(null); setDecisoes([]); }
    catch (error) { notify(errorMessage(error), "erro"); }
    finally { setBusy(false); }
  };
  const decidir = (registro: Registro, acao: Acao, pessoaId?: number) => setDecisoes(atuais => atuais.map(d => d.indice === registro.indice
    ? { indice: d.indice, acao, pessoa_id: acao === "atualizar" ? pessoaId ?? registro.coincidencias[0]?.pessoa_id : undefined } : d));
  const selecionados = decisoes.filter(d => d.acao !== "ignorar").length;
  return <section className="panel overflow-hidden xl:col-span-2">
    <header className="flex items-center gap-3 border-b p-5"><ContactRound className="size-6" /><div><h2 className="font-display text-xl font-semibold">Importar e exportar contatos</h2><p className="text-sm text-slate-500">Revise coincidências antes de importar sua agenda.</p></div></header>
    <div className="space-y-4 p-5">
      <p className="text-sm">Aceita CSV do Google Contacts, Outlook e CSV genérico ou vCard/VCF. E-mails e telefones ajudam a identificar contatos existentes.</p>
      <div className="flex flex-wrap gap-2">
        <input ref={inputRef} className="sr-only" type="file" accept=".csv,.vcf,text/csv,text/vcard" aria-label="Arquivo de contatos" onChange={preparar} disabled={busy} />
        <Button loading={busy} onClick={() => inputRef.current?.click()}><Upload className="size-4" /> Selecionar arquivo para prévia</Button>
        <a className="btn btn-secondary" href={apiUrl("/api/configuracoes/contatos/exportar/csv")} download><Download className="size-4" /> Exportar CSV</a>
        <a className="btn btn-secondary" href={apiUrl("/api/configuracoes/contatos/exportar/vcf")} download><Download className="size-4" /> Exportar vCard</a>
      </div>
      {previa && <div className="space-y-3 rounded-xl border p-4" aria-label="Prévia da importação">
        <h3 className="font-semibold">{previa.registros.length} contatos para revisar</h3>
        <p className="text-sm">A prévia expira em {new Date(previa.expira_em * 1000).toLocaleTimeString("pt-BR")}. Atualizar acrescenta meios de contato e preenche categoria vazia, preservando os dados existentes. Criar sempre produz uma nova pessoa.</p>
        {previa.registros_ignorados > 0 && <p>{previa.registros_ignorados} registros sem nome ignorados.</p>}
        {previa.avisos.map((aviso, i) => <p key={i} className="text-sm text-amber-800">{aviso}</p>)}
        <Button variant="secondary" disabled={busy} onClick={() => setDecisoes(previa.registros.map(r => ({ indice: r.indice, acao: "ignorar" })))}>Ignorar todos</Button>
        {previa.registros.slice(pagina * 30, pagina * 30 + 30).map(r => {
          const decisao = decisoes.find(d => d.indice === r.indice)!;
          return <article key={r.indice} className="rounded-lg border p-3">
            <h4 className="font-semibold">{r.indice + 1}. {r.contato.nome}</h4>
            <p className="text-sm">{r.contato.campos.map(c => `${c.tipo}: ${c.valor}`).join(" · ")}</p>
            {r.coincidencias.length > 0 && <p className="text-sm text-amber-800">Coincidências: {r.coincidencias.map(c => c.nome).join(", ")}</p>}
            {r.repetidos_no_arquivo.length > 0 && <p className="text-sm text-amber-800">Também coincide com os registros {r.repetidos_no_arquivo.map(i => i + 1).join(", ")} do arquivo.</p>}
            <label className="mt-2 block" htmlFor={`importacao-acao-${r.indice}`}>Ação para {r.contato.nome}</label><select id={`importacao-acao-${r.indice}`} className="field" disabled={busy} value={decisao.acao} onChange={e => decidir(r, e.target.value as Acao)}>
              <option value="ignorar">Ignorar</option><option value="criar">Criar nova pessoa</option>
              {r.coincidencias.length > 0 && <option value="atualizar">Atualizar contato existente</option>}
            </select>
            {decisao.acao === "atualizar" && <><label className="block" htmlFor={`importacao-destino-${r.indice}`}>Contato a atualizar para {r.contato.nome}</label><select id={`importacao-destino-${r.indice}`} className="field" disabled={busy} value={decisao.pessoa_id} onChange={e => decidir(r, "atualizar", Number(e.target.value))}>{r.coincidencias.map(c => <option key={c.pessoa_id} value={c.pessoa_id}>{c.nome} (#{c.pessoa_id})</option>)}</select></>}
          </article>;
        })}
        {previa.registros.length > 30 && <div className="flex items-center gap-3"><Button variant="secondary" disabled={pagina === 0 || busy} onClick={() => setPagina(p => p - 1)}>Contatos anteriores</Button><span>Página {pagina + 1} de {Math.ceil(previa.registros.length / 30)}</span><Button variant="secondary" disabled={(pagina + 1) * 30 >= previa.registros.length || busy} onClick={() => setPagina(p => p + 1)}>Próximos contatos</Button></div>}
        <div className="flex gap-2"><Button loading={busy} onClick={() => void confirmar()}>Confirmar importação ({selecionados})</Button><Button variant="secondary" disabled={busy} onClick={() => void descartar()}>Descartar prévia</Button></div>
      </div>}
      {resultado && <div role="status" className="rounded-xl bg-emerald-50 p-3"><p>{resultado.pessoas_importadas} pessoas criadas, {resultado.pessoas_atualizadas} atualizadas e {resultado.contatos_importados} meios acrescentados. {resultado.registros_ignorados} registros ignorados.</p>{resultado.avisos.map((aviso,i) => <p key={i}>{aviso}</p>)}</div>}
    </div>
  </section>;
}
