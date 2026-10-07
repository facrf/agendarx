const configuredBase = import.meta.env.VITE_API_BASE_URL?.trim() || "";
const API_BASE = configuredBase.replace(/\/$/, "");

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers);
  if ((options.method === "PUT" || options.method === "PATCH") && typeof options.body === "string") {
    try { const data = JSON.parse(options.body); if (typeof data?.versao === "number") headers.set("if-match", String(data.versao)); } catch { /* Non-JSON uploads do not carry a record version. */ }
  }
  if (options.body && !(options.body instanceof FormData) && !headers.has("content-type")) {
    headers.set("content-type", "application/json");
  }

  const response = await fetch(apiUrl(path), {
    cache: "no-store",
    ...options,
    headers,
    credentials: "include",
  });

  if (!response.ok) {
    const body = await response.json().catch(() => null);
    const message =
      body && typeof body.erro === "string"
        ? body.erro
        : httpErrorMessage(response.status);
    if (response.status === 401 && path !== "/api/auth/login") {
      clearApiCache();
      window.dispatchEvent(new Event("agendarx:unauthorized"));
    }
    throw new ApiError(response.status, message);
  }

  const revision = path.match(/^\/api\/revisoes\/(pessoa|vinculo|tarefa)\/(\d+)\/\d+\/restaurar$/);
  const updatedPath = revision ? `/api/${revision[1] === "pessoa" ? "pessoas" : revision[1] === "vinculo" ? "vinculos" : "calendario/tarefas"}/${revision[2]}` : path;
  const data = response.status === 204 ? undefined : await response.json();
  if (options.method && !["GET", "HEAD"].includes(options.method)) {
    clearApiCache();
    window.dispatchEvent(new CustomEvent("agendarx:data-updated", { detail: { path: updatedPath, method: options.method } }));
  }
  if (options.method && !["GET", "HEAD"].includes(options.method)
    && (/^\/api\/(pessoas|vinculos)(\/\d+)?$/.test(updatedPath) || path === "/api/mesclagem/confirmar" || /^\/api\/vinculos\/lixeira\/\d+(\/restaurar)?$/.test(path) || path === "/api/configuracoes/hp-psicossocial" || /^\/api\/produtividade\/lixeira\/\d+(\/restaurar)?$/.test(path))) {
    window.dispatchEvent(new Event("agendarx:psychosocial-updated"));
    try { localStorage.setItem("agendarx:psychosocial-update", `${Date.now()}:${Math.random()}`); } catch { /* A atualização da aba atual permanece disponível. */ }
  }
  return data as T;
}

export function apiUrl(path: string): string {
  if (/^https?:\/\//.test(path)) return path;
  return `${API_BASE}${path.startsWith("/") ? path : `/${path}`}`;
}

const getCache = new Map<string, { value: unknown; expires: number }>();
const pendingGets = new Map<string, Promise<unknown>>();
let cacheRevision = 0;
export function clearApiCache() { cacheRevision += 1; getCache.clear(); pendingGets.clear(); }
async function get<T>(path: string, options: { signal?: AbortSignal; cacheMs?: number } = {}): Promise<T> {
  const cached = getCache.get(path);
  if (options.cacheMs && cached && cached.expires > Date.now()) return cached.value as T;
  if (!options.signal && pendingGets.has(path)) return pendingGets.get(path) as Promise<T>;
  const revision = cacheRevision;
  const pending = request<T>(path, { signal: options.signal }).then(value => {
    if (options.cacheMs && revision === cacheRevision) getCache.set(path, { value, expires: Date.now() + options.cacheMs });
    return value;
  });
  if (!options.signal) pendingGets.set(path, pending);
  try { return await pending; }
  finally { if (pendingGets.get(path) === pending) pendingGets.delete(path); }
}

export const api = {
  get,
  post: <T>(path: string, data?: unknown) =>
    request<T>(path, {
      method: "POST",
      body: data instanceof FormData ? data : data === undefined ? undefined : JSON.stringify(data),
    }),
  put: <T>(path: string, data?: unknown, contentType?: string) =>
    request<T>(path, {
      method: "PUT",
      body:
        data instanceof Blob || data instanceof FormData
          ? data
          : data === undefined
            ? undefined
            : JSON.stringify(data),
      headers: contentType ? { "content-type": contentType } : undefined,
    }),
  patch: <T>(path: string, data?: unknown) =>
    request<T>(path, {
      method: "PATCH",
      body: data === undefined ? undefined : JSON.stringify(data),
    }),
  upload: <T>(path: string, data: FormData, onProgress?: (percentage: number) => void) =>
    new Promise<T>((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open("POST", apiUrl(path));
      xhr.withCredentials = true;
      xhr.responseType = "json";
      xhr.upload.addEventListener("progress", (event) => {
        if (event.lengthComputable) onProgress?.(Math.round((event.loaded / event.total) * 100));
      });
      xhr.addEventListener("load", () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          clearApiCache();
          window.dispatchEvent(new CustomEvent("agendarx:data-updated", { detail: { path, method: "POST" } }));
          onProgress?.(100);
          resolve(xhr.response as T);
          return;
        }
        if (xhr.status === 401) window.dispatchEvent(new Event("agendarx:unauthorized"));
        const message = xhr.response && typeof xhr.response.erro === "string"
          ? xhr.response.erro
          : httpErrorMessage(xhr.status);
        reject(new ApiError(xhr.status, message));
      });
      xhr.addEventListener("error", () => reject(new ApiError(0, "Falha de rede durante o upload")));
      xhr.addEventListener("abort", () => reject(new ApiError(0, "Upload cancelado")));
      xhr.send(data);
    }),
  download: async (path: string, data?: unknown) => {
    const response = await fetch(apiUrl(path), {
      method: data === undefined ? "GET" : "POST",
      body: data === undefined ? undefined : JSON.stringify(data),
      headers: data === undefined ? undefined : { "content-type": "application/json" },
      credentials: "include",
    });
    if (!response.ok) {
      const body = await response.json().catch(() => null);
      throw new ApiError(response.status, body?.erro || httpErrorMessage(response.status));
    }
    const disposition = response.headers.get("content-disposition") || "";
    const filename = disposition.match(/filename="([^"]+)"/)?.[1] || "download";
    return { blob: await response.blob(), filename };
  },
  delete: <T = void>(path: string) => request<T>(path, { method: "DELETE" }),
};

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return "Ocorreu um erro inesperado";
}

function httpErrorMessage(status: number): string {
  return status === 413
    ? "O arquivo excede o limite de envio do servidor. Selecione um arquivo menor ou solicite ao administrador a revisão do limite."
    : `A requisição falhou (${status})`;
}
