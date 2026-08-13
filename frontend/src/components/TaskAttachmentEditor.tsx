import {
  AlertTriangle,
  Download,
  Eye,
  File,
  ImageIcon,
  Paperclip,
  RefreshCcw,
  Trash2,
  UploadCloud,
} from "lucide-react";
import { useRef, useState } from "react";
import type { ChangeEvent, DragEvent } from "react";
import { apiUrl } from "../services/api";
import type { AnexoTarefaCalendario, ArmazenamentoTarefas } from "../types/api";
import { formatBytes } from "../utils/format";
import {
  AttachmentPreviewModal,
  AttachmentThumbnail,
  previewKind,
} from "./AttachmentPreview";
import { Button, cn } from "./ui";

interface TaskAttachmentEditorProps {
  existing: AnexoTarefaCalendario[];
  pending: File[];
  disabled?: boolean;
  onPendingChange: (files: File[]) => void;
  onDeleteExisting: (attachment: AnexoTarefaCalendario) => void;
  onInvalidFiles: (message: string) => void;
  storage: ArmazenamentoTarefas | null;
  uploadStates: Record<string, TaskUploadState>;
  onRetry: (file: File) => void;
}

export interface TaskUploadState {
  progress: number;
  status: "waiting" | "uploading" | "error";
  message?: string;
}

const MAX_ANEXOS = 30;

export function TaskAttachmentEditor({
  existing,
  pending,
  disabled,
  onPendingChange,
  onDeleteExisting,
  onInvalidFiles,
  storage,
  uploadStates,
  onRetry,
}: TaskAttachmentEditorProps) {
  const [dragging, setDragging] = useState(false);
  const [openAttachment, setOpenAttachment] = useState<AnexoTarefaCalendario | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const addFiles = (files: File[]) => {
    const nonEmpty = files.filter((file) => file.size > 0);
    if (nonEmpty.length !== files.length) {
      onInvalidFiles("Arquivos vazios não podem ser anexados");
    }
    const knownKeys = new Set(pending.map(taskFileKey));
    const unique = nonEmpty.filter((file) => !knownKeys.has(taskFileKey(file)));
    if (unique.length !== nonEmpty.length) {
      onInvalidFiles("Arquivos já selecionados foram ignorados");
    }
    const withinFileLimit = storage
      ? unique.filter((file) => file.size <= storage.max_arquivo_bytes)
      : unique;
    if (withinFileLimit.length !== unique.length && storage) {
      onInvalidFiles(`Cada arquivo pode ter no máximo ${formatBytes(storage.max_arquivo_bytes)}`);
    }
    const available = Math.max(0, MAX_ANEXOS - existing.length - pending.length);
    if (withinFileLimit.length > available) {
      onInvalidFiles(`Cada tarefa aceita no máximo ${MAX_ANEXOS} anexos`);
    }
    let selected = withinFileLimit.slice(0, available);
    if (storage) {
      const taskUsed = existing.reduce((total, item) => total + item.tamanho_bytes, 0)
        + pending.reduce((total, file) => total + file.size, 0);
      const globalPending = pending.reduce((total, file) => total + file.size, 0);
      let taskAvailable = Math.max(0, storage.limite_tarefa_bytes - taskUsed);
      let userAvailable = Math.max(0, storage.limite_usuario_bytes - storage.usado_bytes - globalPending);
      const accepted: File[] = [];
      for (const file of selected) {
        if (file.size > taskAvailable || file.size > userAvailable) continue;
        accepted.push(file);
        taskAvailable -= file.size;
        userAvailable -= file.size;
      }
      if (accepted.length !== selected.length) {
        onInvalidFiles("Alguns arquivos excedem o espaço disponível e foram ignorados");
      }
      selected = accepted;
    }
    if (selected.length > 0) onPendingChange([...pending, ...selected]);
  };

  const chooseFiles = (event: ChangeEvent<HTMLInputElement>) => {
    addFiles(Array.from(event.target.files || []));
    event.target.value = "";
  };

  const dropFiles = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();
    setDragging(false);
    if (!disabled) addFiles(Array.from(event.dataTransfer.files || []));
  };

  const dragHasFiles = (event: DragEvent) =>
    Array.from(event.dataTransfer.types || []).includes("Files");

  const taskBytes = existing.reduce((total, item) => total + item.tamanho_bytes, 0);
  const pendingBytes = pending.reduce((total, file) => total + file.size, 0);
  const taskPercentage = storage
    ? Math.min(100, Math.round(((taskBytes + pendingBytes) / storage.limite_tarefa_bytes) * 100))
    : 0;
  const userPercentage = storage
    ? Math.min(100, Math.round(((storage.usado_bytes + pendingBytes) / storage.limite_usuario_bytes) * 100))
    : 0;

  return (
    <div className="space-y-3">
      {storage && (
        <div className="grid gap-2 sm:grid-cols-2">
          <StorageMeter label="Nesta tarefa" used={taskBytes + pendingBytes} limit={storage.limite_tarefa_bytes} percentage={taskPercentage} />
          <StorageMeter label="Todas as tarefas" used={storage.usado_bytes + pendingBytes} limit={storage.limite_usuario_bytes} percentage={userPercentage} />
        </div>
      )}
      <div
        className={cn(
          "rounded-2xl border-2 border-dashed p-4 text-center transition",
          dragging ? "border-teal-500 bg-teal-50 ring-4 ring-teal-100" : "border-slate-200 bg-slate-50",
          disabled && "cursor-not-allowed opacity-60",
        )}
        onDragEnter={(event) => {
          if (!dragHasFiles(event)) return;
          event.preventDefault();
          event.stopPropagation();
          if (!disabled) setDragging(true);
        }}
        onDragOver={(event) => {
          if (!dragHasFiles(event)) return;
          event.preventDefault();
          event.stopPropagation();
          event.dataTransfer.dropEffect = disabled ? "none" : "copy";
        }}
        onDragLeave={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragging(false);
        }}
        onDrop={dropFiles}
      >
        <UploadCloud className="mx-auto size-6 text-teal-700" />
        <p className="mt-2 text-sm font-medium text-slate-700">
          {dragging ? "Solte os arquivos aqui" : "Arraste fotos e arquivos para esta tarefa"}
        </p>
        <p className="mt-1 text-xs text-slate-400">ou use o seletor de arquivos</p>
        <input ref={inputRef} className="sr-only" type="file" multiple disabled={disabled} onChange={chooseFiles} />
        <Button className="mt-3" type="button" variant="secondary" disabled={disabled || existing.length + pending.length >= MAX_ANEXOS} onClick={() => inputRef.current?.click()}>
          <Paperclip className="size-4" /> Selecionar arquivos
        </Button>
      </div>

      {pending.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs font-semibold uppercase tracking-wide text-slate-400">Serão enviados ao salvar</p>
          {pending.map((file, index) => (
            <div key={taskFileKey(file)} className="rounded-xl border border-amber-100 bg-amber-50 px-3 py-2">
              <div className="flex items-center gap-2">
              {file.type.startsWith("image/") ? <ImageIcon className="size-4 shrink-0 text-fuchsia-600" /> : <File className="size-4 shrink-0 text-sky-600" />}
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-semibold text-slate-700">{file.name}</p>
                <p className="text-[11px] text-slate-400">{formatBytes(file.size)}{uploadStates[taskFileKey(file)]?.status === "uploading" ? ` · ${uploadStates[taskFileKey(file)].progress}%` : ""}</p>
              </div>
              {uploadStates[taskFileKey(file)]?.status === "error" && <button type="button" disabled={disabled} className="rounded-lg p-1.5 text-amber-700 hover:bg-white disabled:opacity-50" onClick={() => onRetry(file)} title="Tentar novamente"><RefreshCcw className="size-3.5" /></button>}
              <button type="button" disabled={disabled} className="rounded-lg p-1.5 text-rose-600 hover:bg-white disabled:cursor-not-allowed disabled:opacity-50" onClick={() => onPendingChange(pending.filter((_, itemIndex) => itemIndex !== index))} aria-label={`Remover ${file.name}`}>
                <Trash2 className="size-3.5" />
              </button>
              </div>
              {uploadStates[taskFileKey(file)]?.status === "uploading" && <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-amber-100"><div className="h-full rounded-full bg-teal-600 transition-[width]" style={{ width: `${uploadStates[taskFileKey(file)].progress}%` }} /></div>}
              {uploadStates[taskFileKey(file)]?.status === "error" && <p className="mt-1 flex items-center gap-1 text-[11px] text-rose-600"><AlertTriangle className="size-3" /> {uploadStates[taskFileKey(file)].message || "Falha no envio"}</p>}
            </div>
          ))}
        </div>
      )}

      {existing.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs font-semibold uppercase tracking-wide text-slate-400">Anexos da tarefa</p>
          <div className="grid gap-2 sm:grid-cols-2">
            {existing.map((attachment) => {
              const isImage = previewKind(attachment) === "image";
              return (
                <article key={attachment.id} className="flex min-w-0 items-center gap-3 rounded-xl border border-slate-100 bg-white p-2.5">
                  {isImage ? (
                    <button type="button" className="size-11 shrink-0 overflow-hidden rounded-lg bg-slate-100" onClick={() => setOpenAttachment(attachment)}>
                      <AttachmentThumbnail attachment={attachment} className="h-full w-full object-cover" alt="" />
                    </button>
                  ) : <File className="mx-3 size-5 shrink-0 text-sky-600" />}
                  <button type="button" className="min-w-0 flex-1 text-left" onClick={() => setOpenAttachment(attachment)}>
                    <p className="truncate text-xs font-semibold text-slate-700">{attachment.nome_arquivo}</p>
                    <p className="text-[11px] text-slate-400">{formatBytes(attachment.tamanho_bytes)}</p>
                  </button>
                  <button type="button" className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-teal-700" onClick={() => setOpenAttachment(attachment)} title="Visualizar"><Eye className="size-3.5" /></button>
                  <a className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-teal-700" href={apiUrl(attachment.url_download)} title="Baixar"><Download className="size-3.5" /></a>
                  <button type="button" disabled={disabled} className="rounded-lg p-1.5 text-rose-600 hover:bg-rose-50 disabled:cursor-not-allowed disabled:opacity-50" onClick={() => onDeleteExisting(attachment)} title="Excluir"><Trash2 className="size-3.5" /></button>
                </article>
              );
            })}
          </div>
        </div>
      )}

      <AttachmentPreviewModal attachment={openAttachment} onClose={() => setOpenAttachment(null)} />
    </div>
  );
}

export function taskFileKey(file: File) {
  return `${file.name}:${file.size}:${file.lastModified}`;
}

function StorageMeter({ label, used, limit, percentage }: {
  label: string;
  used: number;
  limit: number;
  percentage: number;
}) {
  const warning = percentage >= 80;
  return (
    <div className={cn("rounded-xl border p-3", warning ? "border-amber-200 bg-amber-50" : "border-slate-100 bg-slate-50")}>
      <div className="flex items-center justify-between gap-2 text-[11px]"><span className="font-semibold text-slate-600">{label}</span><span className={warning ? "font-semibold text-amber-700" : "text-slate-400"}>{formatBytes(used)} / {formatBytes(limit)}</span></div>
      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-white"><div className={cn("h-full rounded-full transition-[width]", warning ? "bg-amber-500" : "bg-teal-600")} style={{ width: `${percentage}%` }} /></div>
    </div>
  );
}
