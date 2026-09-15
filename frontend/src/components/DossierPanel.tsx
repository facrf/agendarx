import {
  Archive,
  Download,
  Edit3,
  Eye,
  File,
  FileAudio,
  FileText,
  FileVideo,
  ImageIcon,
  Music2,
  Plus,
  RefreshCcw,
  Trash2,
  UploadCloud,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, DragEvent } from "react";
import { api, apiUrl, errorMessage } from "../services/api";
import type { AnexoDossie } from "../types/api";
import { dataTransferHasFiles, droppedFiles } from "../utils/dropFiles";
import { formatBytes, formatDate } from "../utils/format";
import { useToast } from "../contexts/ToastContext";
import { AttachmentPreviewModal, AttachmentThumbnail, previewKind } from "./AttachmentPreview";
import { Button, EmptyState, Spinner, cn } from "./ui";

export function DossierPanel({ pessoaId }: { pessoaId: number }) {
  return <DossierContent key={pessoaId} pessoaId={pessoaId} />;
}

interface EnvioArquivo {
  id: number;
  arquivo: File;
  progresso: number;
  status: "aguardando" | "enviando" | "salvo" | "falhou";
  erro?: string;
}

function DossierContent({ pessoaId }: { pessoaId: number }) {
  const [anexos, setAnexos] = useState<AnexoDossie[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [enviando, setEnviando] = useState(false);
  const [envios, setEnvios] = useState<EnvioArquivo[]>([]);
  const envioEmCurso = useRef(false);
  const proximoEnvioId = useRef(0);
  const ativo = useRef(true);
  const [arrastando, setArrastando] = useState(false);
  const [arquivoAberto, setArquivoAberto] = useState<AnexoDossie | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const { notify } = useToast();

  useEffect(() => {
    ativo.current = true;
    return () => { ativo.current = false; };
  }, []);

  const carregar = useCallback(async () => {
    try {
      setAnexos(await api.get(`/api/dossie/pessoas/${pessoaId}/anexos`));
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setCarregando(false);
    }
  }, [pessoaId, notify]);

  useEffect(() => { void carregar(); }, [carregar]);

  const grupos = useMemo(() => ({
    imagens: anexos.filter((anexo) => previewKind(anexo) === "image"),
    audios: anexos.filter((anexo) => previewKind(anexo) === "audio"),
    videos: anexos.filter((anexo) => previewKind(anexo) === "video"),
    arquivos: anexos.filter((anexo) => !["image", "audio", "video"].includes(previewKind(anexo))),
  }), [anexos]);

  const executarEnvios = async (lote: EnvioArquivo[]) => {
    if (envioEmCurso.current || lote.length === 0 || !ativo.current) return;
    envioEmCurso.current = true;
    setEnviando(true);
    const ids = new Set(lote.map((item) => item.id));
    setEnvios((atuais) => atuais.map((item) => ids.has(item.id)
      ? { ...item, status: "aguardando", progresso: 0, erro: undefined } : item));
    const atualizar = (id: number, patch: Partial<EnvioArquivo>) => {
      if (ativo.current) setEnvios((atuais) => atuais.map((item) => item.id === id ? { ...item, ...patch } : item));
    };
    let enviados = 0;
    let falhas = 0;
    try {
      for (const item of lote) {
        if (!ativo.current) return;
        atualizar(item.id, { status: "enviando" });
        try {
          const form = new FormData();
          form.append("arquivo", item.arquivo);
          const anexo = await api.upload<AnexoDossie>(`/api/dossie/pessoas/${pessoaId}/anexos`, form,
            (progresso) => atualizar(item.id, { progresso }));
          if (!ativo.current) return;
          setAnexos((atuais) => [anexo, ...atuais.filter((atual) => atual.id !== anexo.id)]);
          atualizar(item.id, { status: "salvo", progresso: 100 });
          enviados++;
        } catch (error) {
          atualizar(item.id, { status: "falhou", erro: errorMessage(error) });
          falhas++;
        }
      }
      if (!ativo.current) return;
      if (falhas > 0) notify(`${enviados} arquivo(s) salvo(s); ${falhas} falharam. Confira os detalhes e use “Reenviar falhas”.`, "erro");
      else notify(`${enviados} arquivo(s) adicionado(s) ao dossiê`);
    } finally {
      envioEmCurso.current = false;
      if (ativo.current) setEnviando(false);
    }
  };

  const enviarArquivos = async (arquivos: File[]) => {
    if (envioEmCurso.current || !ativo.current) return;
    const validos = arquivos.filter((arquivo) => arquivo.size > 0);
    if (validos.length !== arquivos.length) notify("Arquivos vazios não podem ser anexados", "erro");
    const lote: EnvioArquivo[] = validos.map((arquivo) => ({
      id: ++proximoEnvioId.current, arquivo, progresso: 0, status: "aguardando",
    }));
    if (lote.length === 0) return;
    setEnvios((atuais) => [...atuais, ...lote]);
    await executarEnvios(lote);
  };

  const enviar = (event: ChangeEvent<HTMLInputElement>) => {
    const arquivos = Array.from(event.target.files || []);
    event.target.value = "";
    void enviarArquivos(arquivos);
  };

  const soltarArquivos = async (event: DragEvent<HTMLElement>) => {
    if (!dataTransferHasFiles(event.dataTransfer)) return;
    event.preventDefault();
    event.stopPropagation();
    setArrastando(false);
    if (enviando) return;
    const dropped = await droppedFiles(event.dataTransfer);
    if (dropped.ignoredDirectories > 0) {
      notify("Pastas foram ignoradas; selecione apenas arquivos", "erro");
    }
    await enviarArquivos(dropped.files);
  };

  const excluir = async (anexo: AnexoDossie) => {
    if (!window.confirm(`Excluir “${anexo.nome_arquivo}” do dossiê?`)) return;
    try {
      await api.delete(`/api/dossie/anexos/${anexo.id}`);
      setAnexos((atuais) => atuais.filter((item) => item.id !== anexo.id));
      notify("Anexo excluído");
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const renomear = async (anexo: AnexoDossie) => {
    const nome = window.prompt("Novo nome do arquivo:", anexo.nome_arquivo)?.trim();
    if (!nome || nome === anexo.nome_arquivo) return;
    try {
      const atualizado = await api.put<AnexoDossie>(`/api/dossie/anexos/${anexo.id}`, { nome_arquivo: nome });
      setAnexos((atuais) => atuais.map((item) => item.id === atualizado.id ? atualizado : item));
      setArquivoAberto((atual) => atual?.id === atualizado.id ? atualizado : atual);
      notify("Anexo renomeado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  if (carregando) return <Spinner label="Carregando dossiê" />;

  return (
    <div className="space-y-7">
      <section
        className={cn(
          "flex flex-col gap-4 rounded-3xl border-2 border-dashed bg-teal-50/70 p-5 transition sm:flex-row sm:items-center sm:justify-between",
          arrastando ? "border-teal-500 ring-4 ring-teal-100" : "border-teal-300",
        )}
        onDragEnter={(event) => { if (dataTransferHasFiles(event.dataTransfer)) { event.preventDefault(); event.stopPropagation(); if (!enviando) setArrastando(true); } }}
        onDragOver={(event) => { if (dataTransferHasFiles(event.dataTransfer)) { event.preventDefault(); event.stopPropagation(); event.dataTransfer.dropEffect = enviando ? "none" : "copy"; } }}
        onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setArrastando(false); }}
        onDrop={(event) => void soltarArquivos(event)}
      >
        <div className="flex items-center gap-4">
          <div className="grid size-12 place-items-center rounded-2xl bg-white text-teal-700 shadow-sm"><UploadCloud className="size-6" /></div>
          <div><h3 className="font-display text-lg font-semibold text-slate-950">{arrastando ? "Solte os arquivos aqui" : "Adicionar ao dossiê"}</h3><p className="text-sm text-slate-500">Fotos, áudios, PDFs, ZIPs ou qualquer arquivo relevante.</p></div>
        </div>
        <input ref={inputRef} className="sr-only" type="file" multiple disabled={enviando} onChange={enviar} />
        <Button type="button" loading={enviando} onClick={() => inputRef.current?.click()}><Plus className="size-4" /> Selecionar arquivo</Button>
      </section>

      {envios.length > 0 && (
        <section className="rounded-2xl border border-slate-200 bg-white p-4" aria-label="Progresso dos uploads">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h3 className="font-semibold text-slate-800">Envio de arquivos</h3>
            <div className="flex flex-wrap gap-2">
              {envios.some((item) => item.status === "falhou") && <Button type="button" variant="secondary" disabled={enviando} onClick={() => void executarEnvios(envios.filter((item) => item.status === "falhou"))}><RefreshCcw className="size-4" /> Reenviar falhas ({envios.filter((item) => item.status === "falhou").length})</Button>}
              <Button type="button" variant="secondary" disabled={enviando} onClick={() => setEnvios([])}>Limpar lista</Button>
            </div>
          </div>
          <p className="mt-2 text-xs text-slate-500" role="status">{envios.filter((item) => item.status === "salvo").length} de {envios.length} arquivo(s) salvo(s). {enviando ? "Aguarde nesta página até o envio terminar." : "Limpar a lista não exclui os anexos salvos. As falhas ficam disponíveis para reenvio enquanto esta página estiver aberta."}</p>
          <ul className="mt-4 max-h-80 space-y-3 overflow-auto">
            {envios.map((item) => (
              <li key={item.id} className="rounded-xl bg-slate-50 p-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="min-w-0 flex-1"><p className="break-all text-sm font-medium text-slate-800">{item.arquivo.name}</p><p className="text-xs text-slate-500">{formatBytes(item.arquivo.size)} · {item.status === "salvo" ? "Salvo" : item.status === "falhou" ? "Falha no envio" : item.status === "aguardando" ? "Aguardando" : item.progresso === 100 ? "Salvando no dossiê…" : `Enviando: ${item.progresso}%`}</p></div>
                  {item.status === "falhou" && <Button type="button" variant="secondary" disabled={enviando} onClick={() => void executarEnvios([item])} aria-label={`Reenviar ${item.arquivo.name}`}><RefreshCcw className="size-4" /> Reenviar</Button>}
                </div>
                {(item.status === "enviando" || item.status === "aguardando") && <progress className="mt-2 h-2 w-full accent-teal-600" max={100} value={item.progresso} aria-label={`Envio de ${item.arquivo.name}`} />}
                {item.erro && <p className="mt-2 break-words text-xs text-rose-600">{item.erro}</p>}
              </li>
            ))}
          </ul>
        </section>
      )}

      {anexos.length === 0 ? (
        <EmptyState icon={<Archive className="size-7" />} title="Dossiê ainda vazio" description="Adicione fotos, gravações, documentos e outros registros relacionados a esta pessoa." />
      ) : (
        <>
          {grupos.imagens.length > 0 && (
            <MediaSection icon={<ImageIcon />} title="Galeria de fotos" count={grupos.imagens.length}>
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-4">
                {grupos.imagens.map((anexo) => (
                  <article key={anexo.id} className="group relative aspect-square overflow-hidden rounded-2xl bg-slate-100">
                    <button type="button" className="h-full w-full" onClick={() => setArquivoAberto(anexo)} title={`Visualizar ${anexo.nome_arquivo}`}>
                      <AttachmentThumbnail attachment={anexo} className="h-full w-full object-cover transition duration-300 group-hover:scale-105" alt={anexo.nome_arquivo} loading="lazy" />
                    </button>
                    <div className="pointer-events-none absolute inset-x-0 bottom-0 flex items-end justify-end gap-2 bg-gradient-to-t from-slate-950/75 via-slate-950/20 to-transparent p-3 opacity-100 transition sm:opacity-0 sm:group-hover:opacity-100">
                      <button type="button" className="pointer-events-auto rounded-xl bg-white/90 p-2 text-slate-800" onClick={() => void renomear(anexo)} title="Renomear"><Edit3 className="size-4" /></button>
                      <button type="button" className="pointer-events-auto rounded-xl bg-rose-500/90 p-2 text-white" onClick={() => void excluir(anexo)} title="Excluir"><Trash2 className="size-4" /></button>
                    </div>
                  </article>
                ))}
              </div>
            </MediaSection>
          )}

          {grupos.audios.length > 0 && (
            <MediaSection icon={<Music2 />} title="Áudios" count={grupos.audios.length}>
              <div className="grid gap-3 xl:grid-cols-2">
                {grupos.audios.map((anexo) => (
                  <article key={anexo.id} className="rounded-2xl border border-slate-100 bg-slate-50 p-4">
                    <div className="mb-3 flex items-start gap-3"><div className="grid size-10 place-items-center rounded-xl bg-violet-100 text-violet-700"><FileAudio className="size-5" /></div><button type="button" className="min-w-0 flex-1 text-left" onClick={() => setArquivoAberto(anexo)}><p className="truncate text-sm font-semibold text-slate-800">{anexo.nome_arquivo}</p><p className="text-xs text-slate-400">{formatBytes(anexo.tamanho_bytes)} · {formatDate(anexo.data_upload, true)}</p></button><button className="rounded-lg p-2 text-slate-400 hover:bg-slate-100 hover:text-teal-700" onClick={() => void renomear(anexo)} title="Renomear"><Edit3 className="size-4" /></button><button className="rounded-lg p-2 text-slate-400 hover:bg-rose-50 hover:text-rose-600" onClick={() => void excluir(anexo)} title="Excluir"><Trash2 className="size-4" /></button></div>
                    <audio className="h-10 w-full" controls preload="metadata" src={apiUrl(anexo.url_stream)}>Seu navegador não suporta áudio.</audio>
                  </article>
                ))}
              </div>
            </MediaSection>
          )}

          {grupos.videos.length > 0 && (
            <MediaSection icon={<FileVideo />} title="Vídeos" count={grupos.videos.length}>
              <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {grupos.videos.map((anexo) => (
                  <article key={anexo.id} className="group overflow-hidden rounded-2xl border border-slate-100 bg-slate-950">
                    <button type="button" className="relative aspect-video w-full" onClick={() => setArquivoAberto(anexo)} title={`Visualizar ${anexo.nome_arquivo}`}>
                      <video className="h-full w-full object-cover" muted preload="metadata" src={apiUrl(anexo.url_stream)} />
                      <span className="absolute inset-0 grid place-items-center bg-slate-950/20 text-white"><Eye className="size-7 drop-shadow" /></span>
                    </button>
                    <div className="flex items-center gap-2 bg-white p-3"><p className="min-w-0 flex-1 truncate text-xs font-semibold">{anexo.nome_arquivo}</p><button type="button" className="text-slate-400 hover:text-teal-700" onClick={() => void renomear(anexo)} title="Renomear"><Edit3 className="size-4" /></button><button type="button" className="text-slate-400 hover:text-rose-600" onClick={() => void excluir(anexo)} title="Excluir"><Trash2 className="size-4" /></button></div>
                  </article>
                ))}
              </div>
            </MediaSection>
          )}

          {grupos.arquivos.length > 0 && (
            <MediaSection icon={<FileText />} title="Documentos e anexos" count={grupos.arquivos.length}>
              <div className="divide-y divide-slate-100 overflow-hidden rounded-2xl border border-slate-100">
                {grupos.arquivos.map((anexo) => (
                  <article key={anexo.id} className="flex flex-wrap items-center gap-3 bg-white p-3.5 transition hover:bg-slate-50 sm:flex-nowrap sm:px-4">
                    <FileIcon mime={anexo.mime_type} nome={anexo.nome_arquivo} />
                    <button type="button" className="min-w-0 flex-1 basis-[12rem] text-left" onClick={() => setArquivoAberto(anexo)} title="Abrir pré-visualização"><p className="truncate text-sm font-semibold text-slate-800">{anexo.nome_arquivo}</p><p className="text-xs text-slate-400">{anexo.mime_type} · {formatBytes(anexo.tamanho_bytes)}</p></button>
                    <div className="ml-auto flex items-center gap-2">
                      <button className="icon-button" onClick={() => setArquivoAberto(anexo)} title="Visualizar arquivo"><Eye className="size-4" /></button>
                      <a className="icon-button" href={apiUrl(anexo.url_download)} title="Baixar arquivo"><Download className="size-4" /></a>
                      <button className="icon-button" onClick={() => void renomear(anexo)} title="Renomear"><Edit3 className="size-4" /></button>
                      <button className="icon-button text-rose-600" onClick={() => void excluir(anexo)} title="Excluir"><Trash2 className="size-4" /></button>
                    </div>
                  </article>
                ))}
              </div>
            </MediaSection>
          )}
        </>
      )}

      <AttachmentPreviewModal attachment={arquivoAberto} attachments={anexos} onClose={() => setArquivoAberto(null)} />
    </div>
  );
}

function MediaSection({ icon, title, count, children }: { icon: React.ReactNode; title: string; count: number; children: React.ReactNode }) {
  return <section><div className="mb-3 flex items-center gap-2 text-slate-800"><span className="text-teal-700 [&>svg]:size-5">{icon}</span><h3 className="font-display text-lg font-semibold">{title}</h3><span className="chip">{count}</span></div>{children}</section>;
}

function FileIcon({ mime, nome }: { mime: string; nome: string }) {
  const compactado = mime.includes("zip") || /\.(zip|rar|7z)$/i.test(nome);
  return <div className={`grid size-11 shrink-0 place-items-center rounded-xl ${compactado ? "bg-amber-100 text-amber-700" : "bg-sky-100 text-sky-700"}`}>{compactado ? <Archive className="size-5" /> : <File className="size-5" />}</div>;
}
