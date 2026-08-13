import {
  CalendarDays,
  ContactRound,
  GitFork,
  LogOut,
  Menu,
  Settings2,
  X,
} from "lucide-react";
import { useState } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { useAuth } from "../contexts/AuthContext";
import { AdminIcon } from "./AdminIcon";
import { BrandIcon } from "./BrandIcon";
import { cn } from "./ui";

const navegacao = [
  { to: "/pessoas", label: "Pessoas", icon: ContactRound },
  { to: "/calendario", label: "Calendário", icon: CalendarDays },
  { to: "/grafo", label: "Mapa de vínculos", icon: GitFork },
  { to: "/configuracoes", label: "Configurações", icon: Settings2 },
];

export function AppShell() {
  const [menuAberto, setMenuAberto] = useState(false);
  const { usuario, logout } = useAuth();
  const navigate = useNavigate();

  const sair = async () => {
    await logout();
    navigate("/login", { replace: true });
  };

  const links = (
    <nav className="flex flex-1 flex-col gap-1.5">
      {navegacao.map(({ to, label, icon: Icon }) => (
        <NavLink
          key={to}
          to={to}
          onClick={() => setMenuAberto(false)}
          className={({ isActive }) => cn("nav-link", isActive && "nav-link-active")}
        >
          <Icon className="size-5" />
          <span>{label}</span>
        </NavLink>
      ))}
    </nav>
  );

  return (
    <div className="min-h-screen bg-canvas">
      <aside className="fixed inset-y-0 left-0 z-30 hidden w-72 flex-col border-r border-white/10 bg-ink px-5 py-7 text-white lg:flex">
        <Brand />
        <div className="mt-10 flex flex-1 flex-col">{links}</div>
        <UserPanel login={usuario?.login ?? ""} onLogout={() => void sair()} />
      </aside>

      <header className="sticky top-0 z-30 flex h-17 items-center justify-between border-b border-slate-200/80 bg-canvas/90 px-4 backdrop-blur lg:hidden">
        <Brand compact />
        <button className="icon-button" onClick={() => setMenuAberto(true)} aria-label="Abrir menu">
          <Menu className="size-5" />
        </button>
      </header>

      {menuAberto && (
        <div className="fixed inset-0 z-50 lg:hidden">
          <button className="absolute inset-0 bg-slate-950/40 backdrop-blur-sm" onClick={() => setMenuAberto(false)} aria-label="Fechar menu" />
          <aside className="animate-drawer absolute inset-y-0 right-0 flex w-[min(21rem,88vw)] flex-col bg-ink p-5 text-white shadow-2xl">
            <div className="mb-8 flex items-center justify-between">
              <Brand />
              <button className="rounded-xl p-2 text-slate-300 hover:bg-white/10" onClick={() => setMenuAberto(false)}>
                <X className="size-5" />
              </button>
            </div>
            {links}
            <UserPanel login={usuario?.login ?? ""} onLogout={() => void sair()} />
          </aside>
        </div>
      )}

      <main className="min-h-screen lg:pl-72">
        <div className="mx-auto max-w-[1480px] px-4 py-7 sm:px-7 lg:px-10 lg:py-10">
          <Outlet />
        </div>
      </main>
    </div>
  );
}

function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <div className="flex items-center gap-3">
      <BrandIcon className="size-10 text-lg" />
      {!compact && (
        <div>
          <div className="font-display text-xl font-semibold tracking-tight">AgendarX</div>
          <div className="text-[11px] uppercase tracking-[0.2em] text-teal-200/70">Relações vivas</div>
        </div>
      )}
      {compact && <span className="font-display text-xl font-semibold text-ink">AgendarX</span>}
    </div>
  );
}

function UserPanel({ login, onLogout }: { login: string; onLogout: () => void }) {
  return (
    <div className="mt-6 flex items-center gap-3 rounded-2xl border border-white/10 bg-white/5 p-3">
      <AdminIcon className="size-9 text-sm" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{login}</p>
        <p className="text-xs text-slate-400">Sessão protegida</p>
      </div>
      <button className="rounded-xl p-2 text-slate-400 transition hover:bg-white/10 hover:text-white" onClick={onLogout} title="Sair">
        <LogOut className="size-4" />
      </button>
    </div>
  );
}
