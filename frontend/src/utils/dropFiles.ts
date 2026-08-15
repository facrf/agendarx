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
  // Copie primeiro DataTransfer.files. Firefox e alguns gerenciadores de
  // arquivos invalidam DataTransferItem assim que o handler devolve o controle.
  const directFiles = Array.from(dataTransfer.files);
  const items = Array.from(dataTransfer.items).filter((item) => item.kind === "file");
  const ignoredDirectories = items.reduce((total, item) => {
    return total + (item.webkitGetAsEntry?.()?.isDirectory ? 1 : 0);
  }, 0);

  if (directFiles.length > 0) {
    return { files: directFiles, ignoredDirectories };
  }

  const candidates = items.flatMap((item) => {
    const entry = item.webkitGetAsEntry?.() ?? null;
    if (entry?.isDirectory) return [];
    return [{ entry, fallback: item.getAsFile() }];
  });

  const resolved = await Promise.all(candidates.map(({ entry, fallback }) => {
    if (entry?.isFile) {
      return fileFromEntry(entry as FileSystemFileEntry, fallback);
    }
    return Promise.resolve(fallback);
  }));
  return { files: resolved.filter((file): file is File => file !== null), ignoredDirectories };
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
