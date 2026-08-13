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
  if (options.body && !(options.body instanceof FormData) && !headers.has("content-type")) {
    headers.set("content-type", "application/json");
  }

  const response = await fetch(apiUrl(path), {
    ...options,
    headers,
    credentials: "include",
  });

  if (!response.ok) {
    const body = await response.json().catch(() => null);
    const message =
      body && typeof body.erro === "string"
        ? body.erro
        : `A requisição falhou (${response.status})`;
    if (response.status === 401 && path !== "/api/auth/login") {
      window.dispatchEvent(new Event("agendarx:unauthorized"));
    }
    throw new ApiError(response.status, message);
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export function apiUrl(path: string): string {
  if (/^https?:\/\//.test(path)) return path;
  return `${API_BASE}${path.startsWith("/") ? path : `/${path}`}`;
}

export const api = {
  get: <T>(path: string) => request<T>(path),
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
          onProgress?.(100);
          resolve(xhr.response as T);
          return;
        }
        if (xhr.status === 401) window.dispatchEvent(new Event("agendarx:unauthorized"));
        const message = xhr.response && typeof xhr.response.erro === "string"
          ? xhr.response.erro
          : `A requisição falhou (${xhr.status})`;
        reject(new ApiError(xhr.status, message));
      });
      xhr.addEventListener("error", () => reject(new ApiError(0, "Falha de rede durante o upload")));
      xhr.addEventListener("abort", () => reject(new ApiError(0, "Upload cancelado")));
      xhr.send(data);
    }),
  delete: <T = void>(path: string) => request<T>(path, { method: "DELETE" }),
};

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return "Ocorreu um erro inesperado";
}
