export interface DroppedFiles {
  files: File[];
  ignoredDirectories: number;
}

export function dataTransferHasFiles(dataTransfer: DataTransfer): boolean {
  if (Array.from(dataTransfer.items).some((item) => item.kind === "file")) return true;
  return Array.from(dataTransfer.types).some((type) => type.toLowerCase() === "files");
}

export async function droppedFiles(dataTransfer: DataTransfer): Promise<DroppedFiles> {
  // Capture os itens enquanto o evento de drop ainda está ativo. Alguns
  // navegadores protegem o DataTransfer assim que o handler devolve o controle.
  const items = Array.from(dataTransfer.items).filter((item) => item.kind === "file");
  const fallbackFiles = Array.from(dataTransfer.files);
  let ignoredDirectories = 0;

  const candidates = items.flatMap((item) => {
    const entry = item.webkitGetAsEntry?.() ?? null;
    if (entry?.isDirectory) {
      ignoredDirectories += 1;
      return [];
    }
    return [{ item, entry }];
  });

  const resolved = await Promise.all(candidates.map(({ item, entry }) => {
    if (entry?.isFile) {
      return fileFromEntry(entry as FileSystemFileEntry, item.getAsFile());
    }
    return Promise.resolve(item.getAsFile());
  }));
  const files = resolved.filter((file): file is File => file !== null);

  return {
    files: files.length > 0 || items.length > 0 ? files : fallbackFiles,
    ignoredDirectories,
  };
}

function fileFromEntry(entry: FileSystemFileEntry, fallback: File | null): Promise<File | null> {
  return new Promise((resolve) => entry.file(resolve, () => resolve(fallback)));
}
