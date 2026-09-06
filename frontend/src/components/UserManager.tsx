import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import { Button, Spinner } from "./ui";

interface Account {
  id: number;
  login: string;
  perfil: "admin" | "usuario";
}

export function UserManager() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [login, setLogin] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Account["perfil"]>("usuario");
  const { notify } = useToast();

  useEffect(() => {
    api.get<Account[]>("/api/configuracoes/admin/usuarios")
      .then(setAccounts)
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setLoading(false));
  }, [notify]);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (saving) return;
    setSaving(true);
    try {
      const account = await api.post<Account>("/api/configuracoes/admin/usuarios", {
        login: login.trim(), senha: password, perfil: role,
      });
      setAccounts((items) => [...items, account].sort((a, b) => a.login.localeCompare(b.login, "pt-BR")));
      setLogin("");
      setPassword("");
      setRole("usuario");
      notify("Conta cadastrada. O novo usuário já pode entrar com suas credenciais.");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally { setSaving(false); }
  };

  return (
    <section className="panel p-5 sm:p-6">
      <h2 className="font-display text-xl font-semibold">Administradores e usuários</h2>
      <p className="mt-2 text-sm text-slate-500">Usuários podem editar pessoas, dossiês e vínculos compartilhados e têm calendário próprio. Administradores também gerenciam contas e configurações globais.</p>
      <form className="mt-5 space-y-3" onSubmit={submit}>
        <label className="block"><span className="field-label">Login da nova conta</span><input className="field" autoComplete="off" required minLength={3} maxLength={64} value={login} onChange={(event) => setLogin(event.target.value)} /></label>
        <label className="block"><span className="field-label">Senha inicial</span><input className="field" type="password" autoComplete="new-password" required minLength={8} maxLength={1024} value={password} onChange={(event) => setPassword(event.target.value)} /></label>
        <label className="block"><span className="field-label">Perfil de acesso</span><select className="field" value={role} onChange={(event) => setRole(event.target.value as Account["perfil"])}><option value="usuario">Usuário</option><option value="admin">Administrador</option></select></label>
        <Button type="submit" loading={saving}>Cadastrar conta</Button>
      </form>
      {loading ? <Spinner label="Carregando contas" /> : <ul className="mt-5 divide-y divide-slate-100">{accounts.map((account) => <li className="flex justify-between gap-3 py-3 text-sm" key={account.id}><span>{account.login}</span><span className="text-slate-500">{account.perfil === "admin" ? "Administrador" : "Usuário"}</span></li>)}</ul>}
    </section>
  );
}
