/** Read into memory and resize portraits before uploading them. */
export async function prepareProfilePhoto(file: File): Promise<Blob> {
  const source = URL.createObjectURL(new Blob([await file.arrayBuffer()], { type: file.type }));
  try {
    const image = new Image();
    image.src = source;
    await image.decode();
    const scale = Math.min(1, 1600 / Math.max(image.naturalWidth, image.naturalHeight));
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
    canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Não foi possível preparar a foto neste navegador");
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    return await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error("Não foi possível processar a foto")), "image/webp", 0.85);
    });
  } finally { URL.revokeObjectURL(source); }
}
