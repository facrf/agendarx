export interface DroppedFiles {
  files: File[];
  ignoredDirectories: number;
}

export function dataTransferHasFiles(dataTransfer: DataTransfer): boolean {
  if (dataTransfer.files.length > 0) return true;
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
    return [{ entry, fallback: item.getAsFile() }];
  });

  const resolved = await Promise.all(candidates.map(({ entry, fallback }) => {
    if (entry?.isFile) {
      return fileFromEntry(entry as FileSystemFileEntry, fallback);
    }
    return Promise.resolve(fallback);
  }));
  const files = resolved.filter((file): file is File => file !== null);

  return {
    // Alguns navegadores/gerenciadores expõem DataTransferItem, mas retornam
    // null em getAsFile()/entry.file(). Nessa situação, DataTransfer.files ainda
    // contém a seleção correta e não deve ser descartado.
    files: files.length > 0 ? files : candidates.length > 0 || items.length === 0 ? fallbackFiles : [],
    ignoredDirectories,
  };
}

function fileFromEntry(entry: FileSystemFileEntry, fallback: File | null): Promise<File | null> {
  return new Promise((resolve) => {
    try {
      entry.file(resolve, () => resolve(fallback));
    } catch {
      resolve(fallback);
    }
  });
}
