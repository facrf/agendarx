import {
  Download,
  Edit3,
  Eye,
  File,
  FileAudio,
  FileVideo,
  ImageIcon,
  Paperclip,
  Trash2,
  UploadCloud,
} from "lucide-react";
import { useRef, useState } from "react";
import type { ChangeEvent, DragEvent } from "react";
import { apiUrl } from "../services/api";
import type { AnexoVinculo } from "../types/api";
import { formatBytes, formatDate } from "../utils/format";
import { AttachmentPreviewModal, AttachmentThumbnail, previewKind } from "./AttachmentPreview";
import { Button, cn } from "./ui";

interface RelationshipAttachmentEditorProps {
  existing: AnexoVinculo[];
  pending: File[];
  disabled?: boolean;
  onPendingChange: (files: File[]) => void;
  onDeleteExisting: (attachment: AnexoVinculo) => void;
  onRenameExisting: (attachment: AnexoVinculo) => void;
}

export function RelationshipAttachmentEditor({
  existing,
  pending,
  disabled,
  onPendingChange,
  onDeleteExisting,
  onRenameExisting,
}: RelationshipAttachmentEditorProps) {
  const [dragging, setDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const addFiles = (files: File[]) => {
    const valid = files.filter((file) => file.size > 0);
    if (valid.length > 0) onPendingChange([...pending, ...valid]);
  };
  const chooseFiles = (event: ChangeEvent<HTMLInputElement>) => {
    addFiles(Array.from(event.target.files || []));
    event.target.value = "";
  };
  const dropFiles = (event: DragEvent<HTMLDivElement>) => {
    if (!Array.from(event.dataTransfer.types || []).includes("Files")) return;
    event.preventDefault();
    event.stopPropagation();
    setDragging(false);
    if (!disabled) addFiles(Array.from(event.dataTransfer.files));
  };
  const dragHasFiles = (event: DragEvent) => Array.from(event.dataTransfer.types || []).includes("Files");

  return (
    <div className="space-y-3">
      <div
        className={cn(
          "rounded-2xl border-2 border-dashed p-4 text-center transition",
          dragging ? "border-teal-500 bg-teal-50" : "border-slate-200 bg-slate-50",
          disabled && "cursor-not-allowed opacity-60",
        )}
        onDragEnter={(event) => { if (dragHasFiles(event)) { event.preventDefault(); event.stopPropagation(); if (!disabled) setDragging(true); } }}
        onDragOver={(event) => { if (dragHasFiles(event)) { event.preventDefault(); event.stopPropagation(); event.dataTransfer.dropEffect = disabled ? "none" : "copy"; } }}
        onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragging(false); }}
        onDrop={dropFiles}
      >
        <UploadCloud className="mx-auto size-6 text-teal-700" />
        <p className="mt-2 text-xs leading-5 text-slate-500">Arraste fotos, áudios ou arquivos aqui</p>
        <input ref={inputRef} className="sr-only" type="file" multiple disabled={disabled} onChange={chooseFiles} />
        <Button className="mt-2" type="button" variant="secondary" disabled={disabled} onClick={() => inputRef.current?.click()}><Paperclip className="size-4" /> Selecionar arquivos</Button>
      </div>

      {pending.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs font-semibold uppercase tracking-wide text-slate-400">Aguardando salvamento</p>
          {pending.map((file, index) => (
            <div key={`${file.name}-${file.lastModified}-${index}`} className="flex items-center gap-2 rounded-xl border border-amber-100 bg-amber-50 px-3 py-2">
              <PendingIcon file={file} />
              <div className="min-w-0 flex-1"><p className="truncate text-xs font-semibold text-slate-700">{file.name}</p><p className="text-[11px] text-slate-400">{formatBytes(file.size)}</p></div>
              <button type="button" className="rounded-lg p-1.5 text-rose-600 hover:bg-white" onClick={() => onPendingChange(pending.filter((_, itemIndex) => itemIndex !== index))} aria-label={`Remover ${file.name}`}><Trash2 className="size-3.5" /></button>
            </div>
          ))}
        </div>
      )}

      {existing.length > 0 && <RelationshipMediaList attachments={existing} onDelete={onDeleteExisting} onRename={onRenameExisting} compact />}
    </div>
  );
}

export function RelationshipMediaList({
  attachments,
  onDelete,
  onRename,
  compact = false,
}: {
  attachments: AnexoVinculo[];
  onDelete?: (attachment: AnexoVinculo) => void;
  onRename?: (attachment: AnexoVinculo) => void;
  compact?: boolean;
}) {
  const [openAttachment, setOpenAttachment] = useState<AnexoVinculo | null>(null);
  const images = attachments.filter((item) => previewKind(item) === "image");
  const audios = attachments.filter((item) => previewKind(item) === "audio");
  const videos = attachments.filter((item) => previewKind(item) === "video");
  const files = attachments.filter((item) => !["image", "audio", "video"].includes(previewKind(item)));

  return (
    <div className={compact ? "space-y-3" : "space-y-5"}>
      {images.length > 0 && (
        <div>
          <MediaTitle icon={<ImageIcon />} title="Fotos" count={images.length} />
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
            {images.map((attachment) => (
              <article key={attachment.id} className="group relative aspect-square overflow-hidden rounded-xl bg-slate-100">
                <button type="button" className="h-full w-full" onClick={() => setOpenAttachment(attachment)} title="Ampliar foto">
                  <AttachmentThumbnail attachment={attachment} className="h-full w-full object-cover transition group-hover:scale-105" alt={attachment.nome_arquivo} loading="lazy" />
                </button>
                <div className="pointer-events-none absolute inset-x-0 bottom-0 flex justify-end gap-1.5 bg-gradient-to-t from-slate-950/70 p-2 opacity-100 transition sm:opacity-0 sm:group-hover:opacity-100">
                  {onRename && <button type="button" className="pointer-events-auto rounded-lg bg-white/90 p-1.5 text-slate-700" onClick={() => onRename(attachment)} title="Renomear"><Edit3 className="size-3.5" /></button>}
                  {onDelete && <button type="button" className="pointer-events-auto rounded-lg bg-rose-500 p-1.5 text-white" onClick={() => onDelete(attachment)} title="Excluir"><Trash2 className="size-3.5" /></button>}
                </div>
              </article>
            ))}
          </div>
        </div>
      )}

      {audios.length > 0 && (
        <div>
          <MediaTitle icon={<FileAudio />} title="Áudios" count={audios.length} />
          <div className="space-y-2">
            {audios.map((attachment) => (
              <article key={attachment.id} className="rounded-xl border border-slate-100 bg-slate-50 p-3">
                <div className="mb-2 flex items-center gap-2"><FileAudio className="size-4 text-violet-600" /><button type="button" className="min-w-0 flex-1 truncate text-left text-xs font-semibold" onClick={() => setOpenAttachment(attachment)}>{attachment.nome_arquivo}</button>{onRename && <button type="button" className="text-slate-400 hover:text-teal-700" onClick={() => onRename(attachment)} title="Renomear"><Edit3 className="size-3.5" /></button>}{onDelete && <button type="button" className="text-rose-600" onClick={() => onDelete(attachment)} title="Excluir"><Trash2 className="size-3.5" /></button>}</div>
                <audio className="h-9 w-full" controls preload="metadata" src={apiUrl(attachment.url_stream)}>Seu navegador não suporta áudio.</audio>
              </article>
            ))}
          </div>
        </div>
      )}

      {videos.length > 0 && (
        <div>
          <MediaTitle icon={<FileVideo />} title="Vídeos" count={videos.length} />
          <div className="grid grid-cols-2 gap-2">
            {videos.map((attachment) => (
              <article key={attachment.id} className="group overflow-hidden rounded-xl border border-slate-100 bg-slate-950">
                <button type="button" className="relative aspect-video w-full" onClick={() => setOpenAttachment(attachment)} title="Visualizar vídeo">
                  <video className="h-full w-full object-cover" muted preload="metadata" src={apiUrl(attachment.url_stream)} />
                  <span className="absolute inset-0 grid place-items-center bg-slate-950/20 text-white"><Eye className="size-5" /></span>
                </button>
                {(onRename || onDelete) && <div className="flex justify-end gap-2 bg-white p-2">{onRename && <button type="button" className="text-slate-400 hover:text-teal-700" onClick={() => onRename(attachment)} title="Renomear"><Edit3 className="size-3.5" /></button>}{onDelete && <button type="button" className="text-rose-600" onClick={() => onDelete(attachment)} title="Excluir"><Trash2 className="size-3.5" /></button>}</div>}
              </article>
            ))}
          </div>
        </div>
      )}

      {files.length > 0 && (
        <div>
          <MediaTitle icon={<File />} title="Arquivos" count={files.length} />
          <div className="divide-y divide-slate-100 overflow-hidden rounded-xl border border-slate-100">
            {files.map((attachment) => (
              <article key={attachment.id} className="flex items-center gap-2 bg-white p-3">
                <File className="size-4 shrink-0 text-sky-600" />
                <button type="button" className="min-w-0 flex-1 text-left" onClick={() => setOpenAttachment(attachment)} title="Abrir pré-visualização"><p className="truncate text-xs font-semibold text-slate-700">{attachment.nome_arquivo}</p><p className="text-[11px] text-slate-400">{formatBytes(attachment.tamanho_bytes)} · {formatDate(attachment.data_upload, true)}</p></button>
                <button type="button" className="rounded-lg p-1.5 text-slate-500 hover:bg-slate-100" onClick={() => setOpenAttachment(attachment)} title="Visualizar"><Eye className="size-3.5" /></button>
                <a className="rounded-lg p-1.5 text-slate-500 hover:bg-slate-100" href={apiUrl(attachment.url_download)} title="Baixar"><Download className="size-3.5" /></a>
                {onRename && <button type="button" className="rounded-lg p-1.5 text-slate-500 hover:bg-teal-50 hover:text-teal-700" onClick={() => onRename(attachment)} title="Renomear"><Edit3 className="size-3.5" /></button>}
                {onDelete && <button type="button" className="rounded-lg p-1.5 text-rose-600 hover:bg-rose-50" onClick={() => onDelete(attachment)} title="Excluir"><Trash2 className="size-3.5" /></button>}
              </article>
            ))}
          </div>
        </div>
      )}

      <AttachmentPreviewModal attachment={openAttachment} onClose={() => setOpenAttachment(null)} />
    </div>
  );
}

function PendingIcon({ file }: { file: File }) {
  if (file.type.startsWith("image/")) return <ImageIcon className="size-4 shrink-0 text-emerald-600" />;
  if (file.type.startsWith("audio/")) return <FileAudio className="size-4 shrink-0 text-violet-600" />;
  if (file.type.startsWith("video/")) return <FileVideo className="size-4 shrink-0 text-fuchsia-600" />;
  return <File className="size-4 shrink-0 text-sky-600" />;
}

function MediaTitle({ icon, title, count }: { icon: React.ReactNode; title: string; count: number }) {
  return <div className="mb-2 flex items-center gap-2 text-xs font-semibold text-slate-600"><span className="text-teal-700 [&>svg]:size-4">{icon}</span><span>{title}</span><span className="chip">{count}</span></div>;
}
