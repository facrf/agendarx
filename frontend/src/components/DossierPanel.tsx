import {
  Archive,
  Download,
  File,
  FileAudio,
  FileText,
  ImageIcon,
  Maximize2,
  Music2,
  Plus,
  Trash2,
  UploadCloud,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent } from "react";
import { api, apiUrl, errorMessage } from "../services/api";
import type { AnexoDossie } from "../types/api";
import { formatBytes, formatDate } from "../utils/format";
import { useToast } from "../contexts/ToastContext";
import { Button, EmptyState, Modal, Spinner } from "./ui";

export function DossierPanel({ pessoaId }: { pessoaId: number }) {
  const [anexos, setAnexos] = useState<AnexoDossie[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [enviando, setEnviando] = useState(false);
  const [imagemAberta, setImagemAberta] = useState<AnexoDossie | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const { notify } = useToast();

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
    imagens: anexos.filter((anexo) => anexo.mime_type.startsWith("image/")),
    audios: anexos.filter((anexo) => anexo.mime_type.startsWith("audio/")),
    arquivos: anexos.filter((anexo) => !anexo.mime_type.startsWith("image/") && !anexo.mime_type.startsWith("audio/")),
  }), [anexos]);

  const enviar = async (event: ChangeEvent<HTMLInputElement>) => {
    const arquivo = event.target.files?.[0];
    if (!arquivo) return;
    const form = new FormData();
    form.append("arquivo", arquivo);
    setEnviando(true);
    try {
      const anexo = await api.post<AnexoDossie>(`/api/dossie/pessoas/${pessoaId}/anexos`, form);
      setAnexos((atuais) => [anexo, ...atuais]);
      notify("Arquivo adicionado ao dossiê");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setEnviando(false);
      event.target.value = "";
    }
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

  if (carregando) return <Spinner label="Carregando dossiê" />;

  return (
    <div className="space-y-7">
      <section className="flex flex-col gap-4 rounded-3xl border border-dashed border-teal-300 bg-teal-50/70 p-5 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-4">
          <div className="grid size-12 place-items-center rounded-2xl bg-white text-teal-700 shadow-sm"><UploadCloud className="size-6" /></div>
          <div><h3 className="font-display text-lg font-semibold text-slate-950">Adicionar ao dossiê</h3><p className="text-sm text-slate-500">Fotos, áudios, PDFs, ZIPs ou qualquer arquivo relevante.</p></div>
        </div>
        <input ref={inputRef} className="sr-only" type="file" onChange={enviar} />
        <Button type="button" loading={enviando} onClick={() => inputRef.current?.click()}><Plus className="size-4" /> Selecionar arquivo</Button>
      </section>

      {anexos.length === 0 ? (
        <EmptyState icon={<Archive className="size-7" />} title="Dossiê ainda vazio" description="Adicione fotos, gravações, documentos e outros registros relacionados a esta pessoa." />
      ) : (
        <>
          {grupos.imagens.length > 0 && (
            <MediaSection icon={<ImageIcon />} title="Galeria de fotos" count={grupos.imagens.length}>
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-4">
                {grupos.imagens.map((anexo) => (
                  <article key={anexo.id} className="group relative aspect-square overflow-hidden rounded-2xl bg-slate-100">
                    <img className="h-full w-full object-cover transition duration-300 group-hover:scale-105" src={apiUrl(anexo.url_stream)} alt={anexo.nome_arquivo} loading="lazy" />
                    <div className="absolute inset-0 flex items-end justify-between bg-gradient-to-t from-slate-950/75 via-transparent to-transparent p-3 opacity-0 transition group-hover:opacity-100">
                      <button type="button" className="rounded-xl bg-white/90 p-2 text-slate-800" onClick={() => setImagemAberta(anexo)} title="Ampliar"><Maximize2 className="size-4" /></button>
                      <button type="button" className="rounded-xl bg-rose-500/90 p-2 text-white" onClick={() => void excluir(anexo)} title="Excluir"><Trash2 className="size-4" /></button>
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
                    <div className="mb-3 flex items-start gap-3"><div className="grid size-10 place-items-center rounded-xl bg-violet-100 text-violet-700"><FileAudio className="size-5" /></div><div className="min-w-0 flex-1"><p className="truncate text-sm font-semibold text-slate-800">{anexo.nome_arquivo}</p><p className="text-xs text-slate-400">{formatBytes(anexo.tamanho_bytes)} · {formatDate(anexo.data_upload, true)}</p></div><button className="rounded-lg p-2 text-slate-400 hover:bg-rose-50 hover:text-rose-600" onClick={() => void excluir(anexo)} title="Excluir"><Trash2 className="size-4" /></button></div>
                    <audio className="h-10 w-full" controls preload="metadata" src={apiUrl(anexo.url_stream)}>Seu navegador não suporta áudio.</audio>
                  </article>
                ))}
              </div>
            </MediaSection>
          )}

          {grupos.arquivos.length > 0 && (
            <MediaSection icon={<FileText />} title="Documentos e anexos" count={grupos.arquivos.length}>
              <div className="divide-y divide-slate-100 overflow-hidden rounded-2xl border border-slate-100">
                {grupos.arquivos.map((anexo) => (
                  <article key={anexo.id} className="flex items-center gap-3 bg-white p-3.5 transition hover:bg-slate-50 sm:px-4">
                    <FileIcon mime={anexo.mime_type} nome={anexo.nome_arquivo} />
                    <div className="min-w-0 flex-1"><p className="truncate text-sm font-semibold text-slate-800">{anexo.nome_arquivo}</p><p className="text-xs text-slate-400">{anexo.mime_type} · {formatBytes(anexo.tamanho_bytes)}</p></div>
                    <a className="icon-button" href={apiUrl(anexo.url_download)} title="Baixar arquivo"><Download className="size-4" /></a>
                    <button className="icon-button text-rose-600" onClick={() => void excluir(anexo)} title="Excluir"><Trash2 className="size-4" /></button>
                  </article>
                ))}
              </div>
            </MediaSection>
          )}
        </>
      )}

      <Modal open={Boolean(imagemAberta)} onClose={() => setImagemAberta(null)} title={imagemAberta?.nome_arquivo || "Imagem"} className="max-w-5xl bg-slate-950">
        {imagemAberta && <img className="mx-auto max-h-[75vh] max-w-full rounded-xl object-contain" src={apiUrl(imagemAberta.url_stream)} alt={imagemAberta.nome_arquivo} />}
      </Modal>
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
