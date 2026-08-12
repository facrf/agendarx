import { Eye, EyeOff, Fingerprint, LockKeyhole, Network, UserRound } from "lucide-react";
import { useState } from "react";
import type { FormEvent } from "react";
import { Navigate, useLocation, useNavigate } from "react-router-dom";
import { Button } from "../components/ui";
import { useAuth } from "../contexts/AuthContext";
import { errorMessage } from "../services/api";

export function LoginPage() {
  const [login, setLogin] = useState("");
  const [senha, setSenha] = useState("");
  const [mostrarSenha, setMostrarSenha] = useState(false);
  const [erro, setErro] = useState("");
  const [enviando, setEnviando] = useState(false);
  const auth = useAuth();
  const navigate = useNavigate();
  const location = useLocation();

  if (!auth.carregando && auth.usuario) return <Navigate to="/pessoas" replace />;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setErro("");
    setEnviando(true);
    try {
      await auth.login(login, senha);
      const from = (location.state as { from?: string } | null)?.from || "/pessoas";
      navigate(from, { replace: true });
    } catch (error) {
      setErro(errorMessage(error));
    } finally {
      setEnviando(false);
    }
  };

  return (
    <main className="relative grid min-h-screen overflow-hidden bg-ink lg:grid-cols-[1.15fr_0.85fr]">
      <section className="relative hidden overflow-hidden p-12 lg:flex lg:flex-col lg:justify-between">
        <div className="absolute -left-24 top-1/4 size-96 rounded-full bg-teal-500/10 blur-3xl" />
        <div className="absolute -right-20 bottom-0 size-[30rem] rounded-full bg-coral/15 blur-3xl" />
        <div className="relative z-10 flex items-center gap-3 text-white">
          <div className="grid size-12 place-items-center rounded-2xl bg-coral text-xl font-bold shadow-xl shadow-coral/20">A</div>
          <div>
            <p className="font-display text-2xl font-semibold">AgendarX</p>
            <p className="text-xs uppercase tracking-[0.24em] text-teal-100/60">Relações vivas</p>
          </div>
        </div>

        <div className="relative z-10 max-w-2xl pb-10">
          <p className="mb-5 inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-teal-100">
            <Network className="size-3.5" /> Pessoas, contexto e conexões
          </p>
          <h1 className="font-display text-5xl font-semibold leading-[1.08] tracking-tight text-white xl:text-6xl">
            Histórias humanas merecem mais que uma lista de contatos.
          </h1>
          <p className="mt-6 max-w-xl text-lg leading-8 text-slate-300">
            Reúna contatos, mídias e vínculos em um espaço privado para compreender melhor sua rede.
          </p>
          <div className="mt-10 grid max-w-xl grid-cols-3 gap-3">
            {["Perfis completos", "Dossiê multimídia", "Mapa interativo"].map((item, index) => (
              <div key={item} className="rounded-2xl border border-white/10 bg-white/5 p-4 backdrop-blur">
                <span className="font-display text-2xl font-semibold text-coral">0{index + 1}</span>
                <p className="mt-2 text-sm text-slate-300">{item}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="relative flex min-h-screen items-center justify-center bg-canvas px-5 py-10 sm:px-10 lg:rounded-l-[3rem]">
        <div className="w-full max-w-md">
          <div className="mb-10 flex items-center gap-3 lg:hidden">
            <div className="grid size-11 place-items-center rounded-2xl bg-coral font-bold text-white">A</div>
            <span className="font-display text-2xl font-semibold text-ink">AgendarX</span>
          </div>
          <p className="eyebrow">Área protegida</p>
          <h2 className="font-display text-4xl font-semibold tracking-tight text-slate-950">Bem-vindo de volta</h2>
          <p className="mt-3 text-sm leading-6 text-slate-500">Entre com suas credenciais para acessar sua agenda.</p>

          <form className="mt-9 space-y-5" onSubmit={submit}>
            <div>
              <label className="field-label" htmlFor="login">Usuário</label>
              <div className="relative">
                <UserRound className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
                <input
                  className="field pl-10"
                  id="login"
                  autoComplete="username"
                  autoFocus
                  required
                  value={login}
                  onChange={(event) => setLogin(event.target.value)}
                  placeholder="seu usuário"
                />
              </div>
            </div>
            <div>
              <label className="field-label" htmlFor="senha">Senha</label>
              <div className="relative">
                <LockKeyhole className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
                <input
                  className="field px-10"
                  id="senha"
                  type={mostrarSenha ? "text" : "password"}
                  autoComplete="current-password"
                  required
                  value={senha}
                  onChange={(event) => setSenha(event.target.value)}
                  placeholder="••••••••"
                />
                <button
                  type="button"
                  className="absolute right-3 top-1/2 -translate-y-1/2 rounded-lg p-1 text-slate-400 hover:text-slate-700"
                  onClick={() => setMostrarSenha((value) => !value)}
                  aria-label={mostrarSenha ? "Ocultar senha" : "Mostrar senha"}
                >
                  {mostrarSenha ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                </button>
              </div>
            </div>

            {erro && (
              <div role="alert" className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                {erro}
              </div>
            )}

            <Button className="w-full py-3" type="submit" loading={enviando}>
              <Fingerprint className="size-5" /> Entrar com segurança
            </Button>
          </form>

          <p className="mt-8 text-center text-xs leading-5 text-slate-400">
            Sua sessão usa um cookie HttpOnly e pode ser revogada imediatamente.
          </p>
        </div>
      </section>
    </main>
  );
}

