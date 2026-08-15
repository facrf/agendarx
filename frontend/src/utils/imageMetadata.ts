export interface ImageMetadata {
  latitude?: number;
  longitude?: number;
  make?: string;
  model?: string;
  capturedAt?: string;
  altitude?: number;
}

// EXIF usa uma estrutura TIFF, diretamente ou dentro de JPEG, PNG e WebP.
// A leitura local evita enviar as coordenadas a um serviço de metadados.
export async function readImageMetadata(url: string): Promise<ImageMetadata | null> {
  const response = await fetch(url, { credentials: "include" });
  if (!response.ok) throw new Error("Não foi possível ler os metadados da imagem");
  return parseImageExif(await response.arrayBuffer());
}

function parseImageExif(buffer: ArrayBuffer): ImageMetadata | null {
  const view = new DataView(buffer);
  if (view.byteLength < 8) return null;

  // TIFF também pode ser anexado diretamente, sem um contêiner JPEG.
  if (view.getUint16(0) === 0x4949 || view.getUint16(0) === 0x4d4d) {
    return parseTiff(view, 0, view.byteLength);
  }
  if (isPng(view)) return parsePngExif(view);
  if (ascii(view, 0, 4) === "RIFF" && ascii(view, 8, 4) === "WEBP") return parseWebpExif(view);
  if (view.getUint16(0) !== 0xffd8) return null;

  let cursor = 2;
  while (cursor + 4 <= view.byteLength) {
    if (view.getUint8(cursor) !== 0xff) break;
    const marker = view.getUint8(cursor + 1);
    const length = view.getUint16(cursor + 2);
    if (marker === 0xe1 && length >= 8 && ascii(view, cursor + 4, 6) === "Exif\0\0") {
      return parseTiff(view, cursor + 10, cursor + 2 + length);
    }
    if (length < 2) break;
    cursor += 2 + length;
  }
  return null;
}

function isPng(view: DataView) {
  return view.byteLength >= 8
    && view.getUint32(0) === 0x89504e47
    && view.getUint32(4) === 0x0d0a1a0a;
}

function parsePngExif(view: DataView): ImageMetadata | null {
  let cursor = 8;
  while (cursor + 12 <= view.byteLength) {
    const length = view.getUint32(cursor);
    const dataStart = cursor + 8;
    const dataEnd = dataStart + length;
    if (dataEnd + 4 > view.byteLength) return null;
    if (ascii(view, cursor + 4, 4) === "eXIf") return parseTiff(view, dataStart, dataEnd);
    cursor = dataEnd + 4;
  }
  return null;
}

function parseWebpExif(view: DataView): ImageMetadata | null {
  let cursor = 12;
  while (cursor + 8 <= view.byteLength) {
    const type = ascii(view, cursor, 4);
    const length = view.getUint32(cursor + 4, true);
    const dataStart = cursor + 8;
    const dataEnd = dataStart + length;
    if (dataEnd > view.byteLength) return null;
    if (type === "EXIF") {
      const tiffStart = ascii(view, dataStart, 6) === "Exif\0\0" ? dataStart + 6 : dataStart;
      return parseTiff(view, tiffStart, dataEnd);
    }
    cursor = dataEnd + (length % 2);
  }
  return null;
}

function parseTiff(view: DataView, base: number, limit: number): ImageMetadata | null {
  if (base + 8 > limit) return null;
  const byteOrder = view.getUint16(base);
  const little = byteOrder === 0x4949;
  if (!little && byteOrder !== 0x4d4d) return null;
  const u16 = (offset: number) => view.getUint16(offset, little);
  const u32 = (offset: number) => view.getUint32(offset, little);
  if (u16(base + 2) !== 42) return null;

  const entries = (ifdOffset: number) => {
    const result = new Map<number, { type: number; count: number; value: number }>();
    const start = base + ifdOffset;
    if (start + 2 > limit) return result;
    const count = u16(start);
    for (let index = 0; index < count; index += 1) {
      const offset = start + 2 + index * 12;
      if (offset + 12 > limit) break;
      result.set(u16(offset), { type: u16(offset + 2), count: u32(offset + 4), value: offset + 8 });
    }
    return result;
  };
  const valueOffset = (entry: { type: number; count: number; value: number }) => {
    const bytes = entry.count * ({ 1: 1, 2: 1, 3: 2, 4: 4, 5: 8 }[entry.type] || 1);
    return bytes <= 4 ? entry.value : base + u32(entry.value);
  };
  const text = (entry?: { type: number; count: number; value: number }) => {
    if (!entry) return undefined;
    const offset = valueOffset(entry);
    if (offset < base || offset + entry.count > limit) return undefined;
    return ascii(view, offset, entry.count).replace(/\0+$/, "").trim() || undefined;
  };
  const pointer = (entry?: { value: number }) => entry ? u32(entry.value) : undefined;
  const rational = (offset: number) => {
    if (offset + 8 > limit) return NaN;
    const denominator = u32(offset + 4);
    return denominator ? u32(offset) / denominator : NaN;
  };

  const root = entries(u32(base + 4));
  const exifPointer = pointer(root.get(0x8769));
  const exif = exifPointer ? entries(exifPointer) : new Map();
  const gpsPointer = pointer(root.get(0x8825));
  const gps = gpsPointer ? entries(gpsPointer) : new Map();
  const coordinate = (tag: number, refTag: number) => {
    const entry = gps.get(tag);
    if (!entry || entry.type !== 5 || entry.count < 3) return undefined;
    const offset = valueOffset(entry);
    const value = rational(offset) + rational(offset + 8) / 60 + rational(offset + 16) / 3600;
    if (!Number.isFinite(value)) return undefined;
    const ref = text(gps.get(refTag));
    return ref === "S" || ref === "W" ? -value : value;
  };
  const altitudeEntry = gps.get(0x0006);
  const altitudeOffset = altitudeEntry?.type === 5 && altitudeEntry.count >= 1
    ? valueOffset(altitudeEntry)
    : undefined;
  const altitudeValue = altitudeOffset === undefined ? undefined : rational(altitudeOffset);
  const altitudeRef = gps.get(0x0005);
  const belowSeaLevel = altitudeRef ? view.getUint8(valueOffset(altitudeRef)) === 1 : false;

  const metadata: ImageMetadata = {
    make: text(root.get(0x010f)),
    model: text(root.get(0x0110)),
    capturedAt: text(exif.get(0x9003)) || text(exif.get(0x9004)) || text(root.get(0x0132)),
    latitude: coordinate(0x0002, 0x0001),
    longitude: coordinate(0x0004, 0x0003),
    altitude: altitudeValue !== undefined && Number.isFinite(altitudeValue)
      ? (belowSeaLevel ? -altitudeValue : altitudeValue)
      : undefined,
  };
  return Object.values(metadata).some((value) => value !== undefined) ? metadata : null;
}

function ascii(view: DataView, offset: number, length: number) {
  let result = "";
  for (let index = 0; index < length && offset + index < view.byteLength; index += 1) {
    result += String.fromCharCode(view.getUint8(offset + index));
  }
  return result;
}
