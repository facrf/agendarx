import { useEffect, useState } from "react";
import type { ImgHTMLAttributes } from "react";
import { Camera, Download, ExternalLink, File, FileAudio, FileText, FileVideo, ImageIcon, LoaderCircle, MapPin } from "lucide-react";
import { apiUrl } from "../services/api";
import { readImageMetadata } from "../utils/imageMetadata";
import type { ImageMetadata } from "../utils/imageMetadata";
import { Modal } from "./ui";

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

export function AttachmentPreviewModal({ attachment, onClose }: {
  attachment: PreviewAttachment | null;
  onClose: () => void;
}) {
  const kind = attachment ? previewKind(attachment) : "file";
  return (
    <Modal
      open={Boolean(attachment)}
      onClose={onClose}
      title={attachment?.nome_arquivo || "Pré-visualização"}
      className="flex h-[calc(100dvh-1rem)] max-w-6xl flex-col overflow-hidden sm:h-[calc(100dvh-2rem)]"
      bodyClassName="flex min-h-0 flex-1 flex-col overflow-hidden"
    >
      {attachment && (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-auto p-2 sm:p-4">
            <PreviewContent attachment={attachment} kind={kind} />
          </div>
          <div className="flex shrink-0 flex-wrap items-center gap-2 border-t border-slate-100 bg-white px-3 py-3 sm:px-4">
            <p className="min-w-0 flex-1 truncate text-xs text-slate-400">{attachment.mime_type || "Tipo não informado"}</p>
            <a className="btn btn-ghost" href={apiUrl(attachment.url_stream)} target="_blank" rel="noopener noreferrer">
              <ExternalLink className="size-4" /> <span className="hidden sm:inline">Abrir em nova aba</span><span className="sm:hidden">Abrir</span>
            </a>
            <a className="btn btn-secondary" href={apiUrl(attachment.url_download)} download>
              <Download className="size-4" /> <span className="hidden sm:inline">Baixar arquivo</span><span className="sm:hidden">Baixar</span>
            </a>
          </div>
        </div>
      )}
    </Modal>
  );
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
    return <video className="mx-auto h-full max-h-full w-full rounded-xl bg-slate-950 object-contain" controls preload="metadata" src={source}>Seu navegador não suporta vídeo.</video>;
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
        className="h-full min-h-64 w-full rounded-xl border border-slate-200 bg-slate-50"
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
    <div className="grid min-h-full gap-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
      <div className="grid min-h-64 place-items-center rounded-xl bg-slate-950/95">
        <img className="max-h-[70dvh] w-full rounded-xl object-contain" src={source} alt={name} />
      </div>
      <aside className="rounded-2xl border border-slate-200 bg-slate-50 p-4">
        <div className="flex items-center gap-2 font-semibold text-slate-800"><MapPin className="size-4 text-teal-700" /> Local da foto</div>
        {loading ? (
          <p className="mt-4 flex items-center gap-2 text-sm text-slate-500"><LoaderCircle className="size-4 animate-spin" /> Lendo geotag da imagem…</p>
        ) : hasLocation ? (
          <>
            <iframe className="mt-4 h-52 w-full rounded-xl border-0" title={`Mapa de ${name}`} loading="lazy" src={osmEmbed(metadata.latitude!, metadata.longitude!)} />
            <p className="mt-3 font-mono text-xs text-slate-600">{metadata.latitude!.toFixed(6)}, {metadata.longitude!.toFixed(6)}</p>
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
