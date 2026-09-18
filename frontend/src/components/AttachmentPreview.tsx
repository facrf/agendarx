/* Developed with care by FACRF - https://github.com/facrf */
import { LinkedEventFields } from "./LinkedEventFields";
import { useLinkedEvent } from "../hooks/useLinkedEvent";
import { useCallback, useEffect, useState } from "react";
import type { ImgHTMLAttributes } from "react";
import { Camera, ChevronLeft, ChevronRight, Download, ExternalLink, File, FileAudio, FileText, FileVideo, ImageIcon, LoaderCircle, MapPin } from "lucide-react";
import { api, apiUrl, errorMessage } from "../services/api";
import { useToast } from "../contexts/ToastContext";
import { MarkdownText } from "./MarkdownText";
import { readImageMetadata } from "../utils/imageMetadata";
import type { ImageMetadata } from "../utils/imageMetadata";
import { Button, Modal } from "./ui";

export interface PreviewAttachment {
  nome_arquivo: string;
  mime_type: string;
  url_stream: string;
  url_download: string;
  url_thumbnail?: string | null;
}

export type PreviewKind = "image" | "audio" | "video" | "pdf" | "text" | "file";

export function previewKind(attachment: Pick<PreviewAttachment, "nome_arquivo" | "mime_type">): PreviewKind {
  const mime = attachment.mime_type.toLowerCase();
  const name = attachment.nome_arquivo.toLowerCase();
  if (mime.startsWith("image/") || /\.(avif|bmp|gif|ico|jpe?g|png|webp)$/.test(name)) return "image";
  if (mime.startsWith("audio/") || /\.(aac|flac|m4a|mp3|oga|ogg|wav)$/.test(name)) return "audio";
  if (mime.startsWith("video/") || /\.(m4v|mkv|mov|mp4|ogv|webm)$/.test(name)) return "video";
  if (mime === "application/pdf" || name.endsWith(".pdf")) return "pdf";
  if (mime.startsWith("text/") || /\.(csv|json|log|md|txt|xml|ya?ml)$/.test(name)) return "text";
  return "file";
}

export function AttachmentPreviewModal({ attachment, attachments = [], onClose }: {
  attachment: PreviewAttachment | null;
  attachments?: PreviewAttachment[];
  onClose: () => void;
}) {
  const [selection, setSelection] = useState<{ origin: string; url: string } | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [savingNote, setSavingNote] = useState(false);
  const active = (attachment && selection?.origin === attachment.url_stream
    ? attachments.find((item) => item.url_stream === selection.url) : null) || attachment;
  const kind = active ? previewKind(active) : "file";
  const images = attachments.filter((item) => previewKind(item) === "image");
  const index = images.findIndex((item) => item.url_stream === active?.url_stream);
  const navigate = useCallback((delta: number) => {
    if (!attachment || index < 0 || images.length < 2 || savingNote) return;
    setSelection({ origin: attachment.url_stream, url: images[(index + delta + images.length) % images.length].url_stream });
  }, [attachment, index, images, savingNote]);
  const close = () => {
    if (savingNote) return;
    if (Object.keys(drafts).length && !window.confirm("Há notas não salvas. Fechar e descartar essas alterações?")) return;
    setDrafts({});
    setSelection(null);
    onClose();
  };
  useEffect(() => {
    if (!attachment || kind !== "image") return;
    const handler = (event: KeyboardEvent) => {
      if (event.target instanceof HTMLElement && event.target.closest("input, textarea, select, [contenteditable=true]")) return;
      if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        event.preventDefault();
        navigate(event.key === "ArrowRight" ? 1 : -1);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [attachment, kind, navigate]);
  return (
    <Modal
      open={Boolean(attachment)}
      onClose={close}
      title={active?.nome_arquivo || "Pré-visualização"}
      className="h-[min(85dvh,56rem)] max-w-6xl"
      bodyClassName="flex min-h-0 flex-1 flex-col overflow-hidden"
    >
      {active && (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-hidden p-2 sm:p-4">
            <PreviewContent key={active.url_stream} attachment={active} kind={kind} />
          </div>
          <AttachmentNotes key={active.url_stream} attachment={active} draft={drafts[active.url_stream]} saving={savingNote} onSaving={setSavingNote} onDraft={(value) => setDrafts((current) => {
            const next = { ...current };
            if (value === undefined) delete next[active.url_stream];
            else next[active.url_stream] = value;
            return next;
          })} />
          <div className="flex shrink-0 flex-wrap items-center gap-2 border-t border-slate-100 bg-white px-3 py-3 sm:px-4">
            {kind === "image" && images.length > 1 && <div className="flex items-center gap-2">
              <Button type="button" variant="secondary" disabled={savingNote} onClick={() => navigate(-1)} aria-label="Imagem anterior"><ChevronLeft className="size-4" /></Button>
              <span className="text-xs" aria-live="polite">{index + 1} / {images.length}</span>
              <Button type="button" variant="secondary" disabled={savingNote} onClick={() => navigate(1)} aria-label="Próxima imagem"><ChevronRight className="size-4" /></Button>
            </div>}
            <p className="min-w-0 flex-1 truncate text-xs text-slate-400">{active.mime_type || "Tipo não informado"}</p>
            <a className="btn btn-ghost" href={apiUrl(active.url_stream)} target="_blank" rel="noopener noreferrer">
              <ExternalLink className="size-4" /> <span className="hidden sm:inline">Abrir em nova aba</span><span className="sm:hidden">Abrir</span>
            </a>
            <a className="btn btn-secondary" href={apiUrl(active.url_download)} download>
              <Download className="size-4" /> <span className="hidden sm:inline">Baixar arquivo</span><span className="sm:hidden">Baixar</span>
            </a>
          </div>
        </div>
      )}
    </Modal>
  );
}

function AttachmentNotes({ attachment, draft, onDraft, saving, onSaving }: {
  attachment: PreviewAttachment;
  draft?: string;
  onDraft: (value: string | undefined) => void;
  saving: boolean;
  onSaving: (value: boolean) => void;
}) {
  const agenda = useLinkedEvent();
  const [people, setPeople] = useState<number[]>([]);
  const [scheduling, setScheduling] = useState(false);
  const [savedEventId, setSavedEventId] = useState<number | null>(null);
  const [saved, setSaved] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [editing, setEditing] = useState(false);
  const { notify } = useToast();
  const endpoint = attachment.url_stream.replace(/\/stream$/, "/notas");
  const supported = /\/anexos\/\d+\/notas$/.test(endpoint);
  useEffect(() => {
    if (!supported) return;
    let active = true;
    api.get<{ notas: string; pessoas_ids?: number[] }>(endpoint).then((data) => { if (active) { setSaved(data.notas); setPeople(data.pessoas_ids || []); } })
      .catch((err) => { if (active) setError(errorMessage(err)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [endpoint, supported]);
  if (!supported) return null;
  const value = draft ?? saved;
  const save = async () => {
    onSaving(true);
    try {
      const data = await api.put<{ notas: string }>(endpoint, { notas: value });
      setSaved(data.notas);
      onDraft(undefined);
      setEditing(false);
      notify("Notas do arquivo salvas");
    } catch (err) { notify(errorMessage(err), "erro"); }
    finally { onSaving(false); }
  };
  const schedule = async () => {
    try {
      agenda.validate();
      setScheduling(true);
      onSaving(true);
      if (draft !== undefined) {
        const data = await api.put<{ notas: string }>(endpoint, { notas: value });
        setSaved(data.notas);
        onDraft(undefined);
        setEditing(false);
      }
      const event = await agenda.save({ people, title: attachment.nome_arquivo, references: [`[Arquivo: ${attachment.nome_arquivo.replaceAll("[", "").replaceAll("]", "")}](${attachment.url_stream})`] });
      if (event) { setSavedEventId(event.id); notify("Evento vinculado ao arquivo criado"); }
    } catch (err) { notify(errorMessage(err), "erro"); }
    finally { setScheduling(false); onSaving(false); }
  };
  return <details className="max-h-[35dvh] shrink-0 overflow-y-auto border-t border-slate-200 bg-slate-50 px-4 py-2">
    <summary className="cursor-pointer text-sm font-semibold">Notas do arquivo{draft !== undefined ? " · alterações não salvas" : saved ? " · com anotações" : " · adicionar"}</summary>
    {loading ? <p className="py-2 text-sm">Carregando notas…</p> : error ? <p role="alert" className="py-2 text-sm text-rose-700">Não foi possível carregar as notas: {error}. Feche e abra o arquivo para tentar novamente.</p> : <div className="space-y-2 py-2">
      {editing || draft !== undefined ? <textarea aria-label="Notas do arquivo" className="field min-h-28 resize-y font-mono" maxLength={50000} disabled={saving} value={value} onChange={(event) => onDraft(event.target.value === saved ? undefined : event.target.value)} placeholder="Descreva o conteúdo, a origem ou o contexto deste arquivo. Aceita Markdown." /> : <MarkdownText>{value || "Nenhuma nota registrada."}</MarkdownText>}
      <div className="flex gap-2">
        {editing || draft !== undefined ? <><Button type="button" loading={saving} onClick={() => void save()}>Salvar notas</Button><Button type="button" variant="ghost" disabled={saving} onClick={() => { onDraft(undefined); setEditing(false); }}>Cancelar</Button></> : <Button type="button" variant="secondary" onClick={() => setEditing(true)}>Editar notas</Button>}
      </div>
      <LinkedEventFields value={agenda.draft} onChange={agenda.setDraft} disabled={saving || scheduling} />
      {agenda.draft.enabled && <Button type="button" loading={scheduling} disabled={saving && !scheduling} onClick={() => void schedule()}>Salvar notas e criar evento</Button>}
      {savedEventId && <a className="btn btn-secondary" href={`/calendario?tarefa=${savedEventId}`}>Abrir evento criado</a>}
    </div>}
  </details>;
}

export function AttachmentThumbnail({ attachment, ...props }: {
  attachment: PreviewAttachment;
} & Omit<ImgHTMLAttributes<HTMLImageElement>, "src">) {
  const [thumbnailComFalha, setThumbnailComFalha] = useState<string | null>(null);
  const source = !attachment.url_thumbnail || thumbnailComFalha === attachment.url_thumbnail
    ? attachment.url_stream
    : attachment.url_thumbnail;

  return <img {...props} src={apiUrl(source)} onError={() => setThumbnailComFalha(attachment.url_thumbnail ?? null)} />;
}

function PreviewContent({ attachment, kind }: { attachment: PreviewAttachment; kind: PreviewKind }) {
  const source = apiUrl(attachment.url_stream);
  if (kind === "image") {
    return <ImagePreview source={source} name={attachment.nome_arquivo} />;
  }
  if (kind === "video") {
    return <video className="mx-auto block h-full max-h-full w-full max-w-full rounded-xl bg-slate-950 object-contain" controls preload="metadata" src={source}>Seu navegador não suporta vídeo.</video>;
  }
  if (kind === "audio") {
    return (
      <div className="rounded-2xl bg-violet-50 p-6 text-center">
        <FileAudio className="mx-auto size-12 text-violet-600" />
        <audio className="mt-5 w-full" controls autoPlay preload="metadata" src={source}>Seu navegador não suporta áudio.</audio>
      </div>
    );
  }
  if (kind === "pdf" || kind === "text") {
    return (
      <iframe
        className="block h-full min-h-0 w-full rounded-xl border border-slate-200 bg-slate-50"
        src={source}
        title={`Pré-visualização de ${attachment.nome_arquivo}`}
      />
    );
  }
  return (
    <div className="grid min-h-64 place-items-center rounded-2xl border border-dashed border-slate-200 bg-slate-50 p-8 text-center">
      <div>
        <File className="mx-auto size-12 text-sky-600" />
        <p className="mt-4 font-semibold text-slate-800">Este formato não possui miniatura no navegador.</p>
        <p className="mt-1 text-sm text-slate-500">Use o botão de download para abrir o arquivo em um aplicativo compatível.</p>
      </div>
    </div>
  );
}

function ImagePreview({ source, name }: { source: string; name: string }) {
  const [metadata, setMetadata] = useState<ImageMetadata | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    setLoading(true);
    readImageMetadata(source)
      .then((result) => { if (active) setMetadata(result); })
      .catch(() => { if (active) setMetadata(null); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [source]);

  const hasLocation = metadata?.latitude !== undefined && metadata.longitude !== undefined;
  return (
    <div className="flex h-full min-h-0 flex-col gap-4 overflow-y-auto lg:grid lg:grid-cols-[minmax(0,1fr)_20rem] lg:overflow-hidden">
      <div className="grid min-h-64 flex-1 place-items-center overflow-hidden rounded-xl bg-slate-950/95 lg:min-h-0">
        <img className="block max-h-[65dvh] max-w-full rounded-xl object-contain lg:h-full lg:max-h-full lg:w-full" src={source} alt={name} />
      </div>
      <aside className="shrink-0 rounded-2xl border border-slate-200 bg-slate-50 p-4 lg:min-h-0 lg:overflow-y-auto">
        <div className="flex items-center gap-2 font-semibold text-slate-800"><MapPin className="size-4 text-teal-700" /> Local da foto</div>
        {loading ? (
          <p className="mt-4 flex items-center gap-2 text-sm text-slate-500"><LoaderCircle className="size-4 animate-spin" /> Lendo geotag da imagem…</p>
        ) : hasLocation ? (
          <>
            <iframe className="mt-4 h-52 w-full rounded-xl border-0" title={`Mapa de ${name}`} loading="lazy" src={osmEmbed(metadata.latitude!, metadata.longitude!)} />
            <p className="mt-3 font-mono text-xs text-slate-600">{metadata.latitude!.toFixed(6)}, {metadata.longitude!.toFixed(6)}</p>
            {metadata.altitude !== undefined && <p className="mt-1 text-xs text-slate-500">Altitude: {metadata.altitude.toFixed(1)} m</p>}
            <a className="btn btn-secondary mt-3 w-full" href={`https://www.openstreetmap.org/?mlat=${metadata.latitude}&mlon=${metadata.longitude}#map=16/${metadata.latitude}/${metadata.longitude}`} target="_blank" rel="noopener noreferrer"><ExternalLink className="size-4" /> Abrir mapa</a>
          </>
        ) : <p className="mt-4 text-sm leading-6 text-slate-500">Esta imagem não contém coordenadas GPS nos metadados EXIF.</p>}
        {(metadata?.make || metadata?.model || metadata?.capturedAt) && (
          <div className="mt-5 border-t border-slate-200 pt-4 text-xs text-slate-500">
            <p className="mb-2 flex items-center gap-2 font-semibold text-slate-700"><Camera className="size-4" /> Dados da captura</p>
            {(metadata.make || metadata.model) && <p>{[metadata.make, metadata.model].filter(Boolean).join(" ")}</p>}
            {metadata.capturedAt && <p className="mt-1">Capturada em {formatExifDate(metadata.capturedAt)}</p>}
          </div>
        )}
      </aside>
    </div>
  );
}

function osmEmbed(latitude: number, longitude: number) {
  const delta = 0.008;
  const bbox = [longitude - delta, latitude - delta, longitude + delta, latitude + delta].join(",");
  return `https://www.openstreetmap.org/export/embed.html?bbox=${encodeURIComponent(bbox)}&layer=mapnik&marker=${latitude},${longitude}`;
}

function formatExifDate(value: string) {
  const match = value.match(/^(\d{4}):(\d{2}):(\d{2})[ T](\d{2}):(\d{2}):(\d{2})/);
  if (!match) return value;
  return `${match[3]}/${match[2]}/${match[1]} às ${match[4]}:${match[5]}`;
}

export function PreviewTypeIcon({ kind, className = "size-5" }: { kind: PreviewKind; className?: string }) {
  if (kind === "image") return <ImageIcon className={className} />;
  if (kind === "audio") return <FileAudio className={className} />;
  if (kind === "video") return <FileVideo className={className} />;
  if (kind === "pdf" || kind === "text") return <FileText className={className} />;
  return <File className={className} />;
}
