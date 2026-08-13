import { BellRing, CheckCircle2, CircleAlert, X } from "lucide-react";
import { createContext, useCallback, useContext, useMemo, useState } from "react";
import type { PropsWithChildren } from "react";

type ToastKind = "sucesso" | "erro" | "aviso";

interface ToastItem {
  id: number;
  mensagem: string;
  tipo: ToastKind;
}

interface ToastContextValue {
  notify: (mensagem: string, tipo?: ToastKind) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function ToastProvider({ children }: PropsWithChildren) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const remover = useCallback((id: number) => {
    setToasts((atuais) => atuais.filter((toast) => toast.id !== id));
  }, []);

  const notify = useCallback(
    (mensagem: string, tipo: ToastKind = "sucesso") => {
      const id = Date.now() + Math.random();
      setToasts((atuais) => [...atuais, { id, mensagem, tipo }]);
      window.setTimeout(() => remover(id), 4200);
    },
    [remover],
  );

  const value = useMemo(() => ({ notify }), [notify]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="fixed right-4 top-4 z-[100] flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`animate-toast flex items-start gap-3 rounded-2xl border px-4 py-3 shadow-xl backdrop-blur ${
              toast.tipo === "sucesso"
                ? "border-emerald-200 bg-emerald-50/95 text-emerald-900"
                : toast.tipo === "aviso"
                  ? "border-amber-200 bg-amber-50/95 text-amber-900"
                  : "border-rose-200 bg-rose-50/95 text-rose-900"
            }`}
          >
            {toast.tipo === "sucesso" ? (
              <CheckCircle2 className="mt-0.5 size-5 shrink-0" />
            ) : toast.tipo === "aviso" ? (
              <BellRing className="mt-0.5 size-5 shrink-0" />
            ) : (
              <CircleAlert className="mt-0.5 size-5 shrink-0" />
            )}
            <span className="flex-1 text-sm font-medium">{toast.mensagem}</span>
            <button type="button" onClick={() => remover(toast.id)} aria-label="Fechar aviso">
              <X className="size-4" />
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) throw new Error("useToast deve ser usado dentro de ToastProvider");
  return context;
}
