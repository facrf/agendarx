/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { api, errorMessage } from "../services/api";
import { formatBytes } from "../utils/format";
import { Button } from "./ui";
interface Health { armazenamento: { banco_bytes: number; midia_total_bytes: number; anexos_total: number }; backup: { ultimo_validado: { nome_arquivo: string; data_criacao: string } | null; configuracao: { ultimo_erro: string | null; ultima_tentativa_em: string | null } }; pesquisas_por_estado: [string, number][]; falhas_pesquisa: [string, number, string, string | null, string][] }
export function SystemHealth() {
 const [data,setData]=useState<Health>();const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
 const load=async()=>{setBusy(true);try{setData(await api.get<Health>("/api/saude"));}catch(e){setMessage(errorMessage(e));}finally{setBusy(false)}};
 useEffect(()=>{void load();},[]);
 return <section className="panel p-5 xl:col-span-2"><h2 className="font-display text-xl font-semibold">Saúde do sistema</h2><div className="my-3 flex flex-wrap gap-2"><Button variant="secondary" disabled={busy} onClick={()=>void load()}>Atualizar saúde do sistema</Button><Button disabled={busy} onClick={async()=>{setBusy(true);try{const result=await api.post<{integridade_ok:boolean;erro:string|null}>("/api/saude/backup/verificar");setMessage(result.integridade_ok?"Backup verificado: integridade e checksum válidos":result.erro||"Backup inválido");await load();}catch(e){setMessage(errorMessage(e));}finally{setBusy(false)}}}>Verificar último backup</Button></div>
 {message&&<p role="status" className="my-3">{message}</p>}
 {data?.armazenamento&&<div className="grid gap-4 sm:grid-cols-3"><div><h3 className="font-semibold">Armazenamento</h3><p>Banco: {formatBytes(data.armazenamento.banco_bytes)}</p><p>Mídia: {formatBytes(data.armazenamento.midia_total_bytes)} · {data.armazenamento.anexos_total} arquivos</p></div><div><h3 className="font-semibold">Último backup validado</h3><p>{data.backup.ultimo_validado?new Date(data.backup.ultimo_validado.data_criacao).toLocaleString("pt-BR"):"Nenhum backup válido disponível"}</p><p className="break-all text-xs">{data.backup.ultimo_validado?.nome_arquivo}</p><p className="text-sm">Validação registrada na criação. Use a verificação para conferir o arquivo atual.</p>{data.backup.configuracao.ultimo_erro&&<p className="text-rose-700">Última falha: {data.backup.configuracao.ultimo_erro}</p>}</div><div><h3 className="font-semibold">Pesquisas públicas</h3>{data.pesquisas_por_estado.map(([state,count])=><p key={state}>{state}: {count}</p>)}{data.pesquisas_por_estado.length===0&&<p>Nenhum trabalho registrado.</p>}</div></div>}
 {data?.falhas_pesquisa?.length? <div className="mt-4"><h3 className="font-semibold">Falhas recentes</h3><ul>{data.falhas_pesquisa.map(([id,personId,name,error,date])=><li key={id}><a className="text-teal-700 underline" href={`/pessoas/${personId}?aba=osint`}>{name}</a> · {new Date(date).toLocaleString("pt-BR")} · {error}</li>)}</ul></div>:null}
 </section>;
}
