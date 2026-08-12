import { useState } from "react";
import type { ImgHTMLAttributes } from "react";
import { Download, File, FileAudio, FileText, FileVideo, ImageIcon } from "lucide-react";
import { apiUrl } from "../services/api";
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
      className="max-w-6xl"
    >
      {attachment && (
        <div className="space-y-4">
          <PreviewContent attachment={attachment} kind={kind} />
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-slate-100 pt-4">
            <p className="min-w-0 truncate text-xs text-slate-400">{attachment.mime_type || "Tipo não informado"}</p>
            <a className="btn btn-secondary" href={apiUrl(attachment.url_download)} download>
              <Download className="size-4" /> Baixar arquivo
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
    return <img className="mx-auto max-h-[72vh] max-w-full rounded-xl object-contain" src={source} alt={attachment.nome_arquivo} />;
  }
  if (kind === "video") {
    return <video className="mx-auto max-h-[72vh] max-w-full rounded-xl bg-slate-950" controls preload="metadata" src={source}>Seu navegador não suporta vídeo.</video>;
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
        className="h-[72vh] w-full rounded-xl border border-slate-200 bg-slate-50"
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

export function PreviewTypeIcon({ kind, className = "size-5" }: { kind: PreviewKind; className?: string }) {
  if (kind === "image") return <ImageIcon className={className} />;
  if (kind === "audio") return <FileAudio className={className} />;
  if (kind === "video") return <FileVideo className={className} />;
  if (kind === "pdf" || kind === "text") return <FileText className={className} />;
  return <File className={className} />;
}
