import { LoaderCircle, UserRound, X } from "lucide-react";
import { useEffect, useState } from "react";
import type {
  ButtonHTMLAttributes,
  CSSProperties,
  PropsWithChildren,
  ReactNode,
} from "react";
import { apiUrl } from "../services/api";

export function cn(...classes: Array<string | false | null | undefined>) {
  return classes.filter(Boolean).join(" ");
}

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  loading?: boolean;
}

export function Button({
  className,
  variant = "primary",
  loading,
  disabled,
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      className={cn("btn", `btn-${variant}`, className)}
      disabled={disabled || loading}
      {...props}
    >
      {loading && <LoaderCircle className="size-4 animate-spin" />}
      {children}
    </button>
  );
}

interface AvatarProps {
  pessoaId: number;
  nome: string;
  temFoto?: boolean;
  cor?: string | null;
  size?: "sm" | "md" | "lg" | "xl";
  cacheKey?: string | number;
  className?: string;
  pessoaJuridica?: boolean;
}

export function Avatar({
  pessoaId,
  nome,
  temFoto,
  cor = "#86A6A3",
  size = "md",
  cacheKey,
  className,
  pessoaJuridica = false,
}: AvatarProps) {
  const [erro, setErro] = useState(false);
  const dimensoes = { sm: "size-10", md: "size-14", lg: "size-20", xl: "size-28" }[size];
  const style = { "--avatar-color": cor || "#86A6A3" } as CSSProperties;
  const query = cacheKey === undefined ? "" : `?v=${cacheKey}`;

  useEffect(() => setErro(false), [pessoaId, temFoto, cacheKey]);

  return (
    <div className={cn("avatar", pessoaJuridica && "avatar-juridica", dimensoes, className)} style={style}>
      {temFoto && !erro ? (
        <img
          src={apiUrl(`/api/dossie/pessoas/${pessoaId}/foto${query}`)}
          alt={`Foto de ${nome}`}
          onError={() => setErro(true)}
        />
      ) : (
        <UserRound className="h-1/2 w-1/2 text-slate-400" aria-hidden="true" />
      )}
    </div>
  );
}

export function Spinner({ label = "Carregando" }: { label?: string }) {
  return (
    <div className="flex min-h-40 flex-col items-center justify-center gap-3 text-slate-500">
      <LoaderCircle className="size-7 animate-spin text-teal-700" />
      <span className="text-sm">{label}</span>
    </div>
  );
}

export function EmptyState({ icon, title, description, action }: {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex min-h-64 flex-col items-center justify-center rounded-3xl border border-dashed border-slate-300 bg-white/60 px-6 py-12 text-center">
      <div className="mb-4 rounded-2xl bg-teal-50 p-3 text-teal-700">{icon}</div>
      <h3 className="font-display text-xl font-semibold text-slate-900">{title}</h3>
      <p className="mt-2 max-w-md text-sm leading-6 text-slate-500">{description}</p>
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}

export function PageHeader({ eyebrow, title, description, action }: {
  eyebrow?: string;
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <header className="mb-7 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
      <div>
        {eyebrow && <p className="eyebrow">{eyebrow}</p>}
        <h1 className="font-display text-3xl font-semibold tracking-tight text-slate-950 sm:text-4xl">
          {title}
        </h1>
        {description && <p className="mt-2 max-w-2xl text-sm leading-6 text-slate-500">{description}</p>}
      </div>
      {action}
    </header>
  );
}

export function Modal({ open, onClose, title, children, className, bodyClassName }: PropsWithChildren<{
  open: boolean;
  onClose: () => void;
  title: string;
  className?: string;
  bodyClassName?: string;
}>) {
  useEffect(() => {
    if (!open) return;
    const handler = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("keydown", handler);
    document.body.style.overflow = "hidden";
    return () => {
      window.removeEventListener("keydown", handler);
      document.body.style.overflow = "";
    };
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center overflow-y-auto bg-slate-950/55 p-2 backdrop-blur-sm sm:p-4" onMouseDown={onClose}>
      <section
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cn("animate-modal flex max-h-[calc(100dvh-1rem)] min-h-0 w-full max-w-2xl flex-col overflow-hidden rounded-2xl bg-white shadow-2xl sm:max-h-[calc(100dvh-2rem)] sm:rounded-3xl", className)}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="sticky top-0 z-10 flex shrink-0 items-center justify-between gap-3 border-b border-slate-100 bg-white/95 px-4 py-3 backdrop-blur sm:px-6 sm:py-4">
          <h2 className="min-w-0 truncate font-display text-lg font-semibold text-slate-950 sm:text-xl" title={title}>{title}</h2>
          <button className="icon-button" type="button" onClick={onClose} aria-label="Fechar">
            <X className="size-5" />
          </button>
        </header>
        <div className={bodyClassName ?? "min-h-0 overflow-y-auto p-4 sm:p-6"}>{children}</div>
      </section>
    </div>
  );
}
