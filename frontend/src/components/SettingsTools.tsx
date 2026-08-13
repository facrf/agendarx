import {
  BellOff,
  BellRing,
  ContactRound,
  Download,
  Eye,
  EyeOff,
  FileArchive,
  ImageIcon,
  LockKeyhole,
  RefreshCcw,
  Save,
  ShieldCheck,
  Trash2,
  Upload,
  UserRound,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ChangeEvent, FormEvent } from "react";
import { useAuth } from "../contexts/AuthContext";
import { useToast } from "../contexts/ToastContext";
import { api, apiUrl, errorMessage } from "../services/api";
import type { IdentidadeVisual, ImportacaoContatosResultado } from "../types/api";
import { AdminIcon } from "./AdminIcon";
import { BrandIcon, refreshBranding } from "./BrandIcon";
import { NOTIFICACOES_TAREFAS_KEY } from "./TaskReminderWatcher";
import { Button } from "./ui";

export function TaskNotificationManager() {
  const supported = "Notification" in window;
  const [enabled, setEnabled] = useState(() => supported
    && Notification.permission === "granted"
    && localStorage.getItem(NOTIFICACOES_TAREFAS_KEY) === "true");
  const { notify } = useToast();

  const enable = async () => {
    if (!supported) return;
    const permission = await Notification.requestPermission();
    if (permission !== "granted") {
      localStorage.removeItem(NOTIFICACOES_TAREFAS_KEY);
      setEnabled(false);
      return notify("O navegador não autorizou as notificações", "erro");
    }
    localStorage.setItem(NOTIFICACOES_TAREFAS_KEY, "true");
    setEnabled(true);
    notify("Notificações de tarefas ativadas");
  };

  const disable = () => {
    localStorage.removeItem(NOTIFICACOES_TAREFAS_KEY);
    setEnabled(false);
    notify("Notificações do navegador desativadas");
  };

  return (
    <section className="panel overflow-hidden">
      <header className="flex items-center gap-3 border-b border-slate-100 p-5 sm:p-6">
        <div className="grid size-11 place-items-center rounded-2xl bg-amber-50 text-amber-700"><BellRing className="size-5" /></div>
        <div><h2 className="font-display text-xl font-semibold">Lembretes de tarefas</h2><p className="text-sm text-slate-500">Avisos dentro do sistema e no navegador.</p></div>
      </header>
      <div className="p-5 sm:p-6">
        {!supported ? (
          <p className="rounded-xl bg-slate-50 p-3 text-sm text-slate-500">Este navegador não oferece notificações do sistema. Os lembretes internos continuarão funcionando.</p>
        ) : (
          <>
            <p className="text-sm leading-6 text-slate-600">Os avisos internos ficam sempre ativos. A permissão abaixo acrescenta uma notificação do sistema mesmo enquanto você estiver em outra aba.</p>
            <Button className="mt-4" type="button" variant={enabled ? "secondary" : "primary"} onClick={() => enabled ? disable() : void enable()}>
              {enabled ? <BellOff className="size-4" /> : <BellRing className="size-4" />}
              {enabled ? "Desativar no navegador" : "Ativar no navegador"}
            </Button>
          </>
        )}
      </div>
    </section>
  );
}

export function BrandingManager() {
  const [identidade, setIdentidade] = useState<IdentidadeVisual | null>(null);
  const [salvando, setSalvando] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const { notify } = useToast();

  useEffect(() => {
    api.get<IdentidadeVisual>("/api/configuracoes/identidade")
      .then(setIdentidade)
      .catch((error) => notify(errorMessage(error), "erro"));
  }, [notify]);

  const enviar = async (event: ChangeEvent<HTMLInputElement>) => {
    const arquivo = event.target.files?.[0];
    if (!arquivo) return;
    if (arquivo.size > 2 * 1024 * 1024) {
      event.target.value = "";
      return notify("O ícone deve ter no máximo 2 MB", "erro");
    }
    setSalvando(true);
    try {
      const atualizada = await api.put<IdentidadeVisual>(
        "/api/configuracoes/icone",
        arquivo,
        arquivo.type || "image/png",
      );
      setIdentidade(atualizada);
      refreshBranding();
      notify("Ícone e favicon atualizados");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
      event.target.value = "";
    }
  };

  const restaurar = async () => {
    if (!window.confirm("Restaurar o ícone padrão do AgendarX?")) return;
    setSalvando(true);
    try {
      await api.delete("/api/configuracoes/icone");
      setIdentidade({ tem_icone: false, atualizado_em: null });
      refreshBranding();
      notify("Ícone padrão restaurado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  return (
    <section className="panel overflow-hidden">
      <header className="flex items-center gap-3 border-b border-slate-100 p-5 sm:p-6">
        <div className="grid size-11 place-items-center rounded-2xl bg-orange-50 text-orange-700"><ImageIcon className="size-5" /></div>
        <div><h2 className="font-display text-xl font-semibold">Ícone do sistema</h2><p className="text-sm text-slate-500">Usado ao lado do nome e como favicon.</p></div>
      </header>
      <div className="flex flex-col gap-5 p-5 sm:flex-row sm:items-center sm:p-6">
        <BrandIcon className="size-24 text-4xl" />
        <div className="min-w-0 flex-1">
          <p className="text-sm leading-6 text-slate-600">Envie PNG, JPEG, WebP, GIF ou ICO. Para melhor resultado, use uma imagem quadrada de até 2 MB.</p>
          <p className="mt-1 text-xs text-slate-400">{identidade?.tem_icone ? "Ícone personalizado ativo" : "Ícone padrão ativo"}</p>
          <div className="mt-4 flex flex-wrap gap-2">
            <input ref={inputRef} className="sr-only" type="file" accept="image/*,.ico" onChange={enviar} />
            <Button type="button" loading={salvando} onClick={() => inputRef.current?.click()}><Upload className="size-4" /> Trocar ícone</Button>
            {identidade?.tem_icone && <Button type="button" variant="secondary" disabled={salvando} onClick={() => void restaurar()}><Trash2 className="size-4" /> Restaurar padrão</Button>}
          </div>
        </div>
      </div>
    </section>
  );
}

export function AdminCredentialsManager() {
  const { usuario, atualizarCredenciais, verificarSessao } = useAuth();
  const [login, setLogin] = useState(usuario?.login || "");
  const [senhaAtual, setSenhaAtual] = useState("");
  const [novaSenha, setNovaSenha] = useState("");
  const [confirmacao, setConfirmacao] = useState("");
  const [mostrarSenhas, setMostrarSenhas] = useState(false);
  const [salvando, setSalvando] = useState(false);
  const [salvandoIcone, setSalvandoIcone] = useState(false);
  const iconeInputRef = useRef<HTMLInputElement>(null);
  const { notify } = useToast();

  const salvar = async (event: FormEvent) => {
    event.preventDefault();
    const loginNormalizado = login.trim();
    if (loginNormalizado.length < 3) {
      return notify("O usuário deve possuir ao menos 3 caracteres", "erro");
    }
    if (!senhaAtual) return notify("Informe a senha atual", "erro");
    if (novaSenha && novaSenha.length < 8) {
      return notify("A nova senha deve possuir ao menos 8 caracteres", "erro");
    }
    if (novaSenha !== confirmacao) {
      return notify("A confirmação da nova senha não confere", "erro");
    }
    if (loginNormalizado === usuario?.login && !novaSenha) {
      return notify("Altere o usuário ou informe uma nova senha", "erro");
    }

    setSalvando(true);
    try {
      await atualizarCredenciais({
        login: loginNormalizado,
        senha_atual: senhaAtual,
        ...(novaSenha ? { nova_senha: novaSenha } : {}),
      });
      notify("Credenciais atualizadas. Entre novamente.");
    } catch (error) {
      notify(errorMessage(error), "erro");
      setSalvando(false);
    }
  };

  const enviarIcone = async (event: ChangeEvent<HTMLInputElement>) => {
    const arquivo = event.target.files?.[0];
    if (!arquivo) return;
    if (arquivo.size > 2 * 1024 * 1024) {
      event.target.value = "";
      return notify("O ícone deve ter no máximo 2 MB", "erro");
    }
    setSalvandoIcone(true);
    try {
      await api.put("/api/auth/icone", arquivo, arquivo.type || "image/png");
      await verificarSessao();
      notify("Ícone do administrador atualizado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvandoIcone(false);
      event.target.value = "";
    }
  };

  const restaurarIcone = async () => {
    if (!window.confirm("Restaurar o ícone do administrador para as iniciais?")) return;
    setSalvandoIcone(true);
    try {
      await api.delete("/api/auth/icone");
      await verificarSessao();
      notify("Ícone do administrador restaurado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvandoIcone(false);
    }
  };

  return (
    <section className="panel overflow-hidden">
      <header className="flex items-center gap-3 border-b border-slate-100 p-5 sm:p-6">
        <div className="grid size-11 place-items-center rounded-2xl bg-violet-50 text-violet-700"><ShieldCheck className="size-5" /></div>
        <div><h2 className="font-display text-xl font-semibold">Administrador</h2><p className="text-sm text-slate-500">Altere o usuário e a senha de acesso.</p></div>
      </header>
      <form className="space-y-4 p-5 sm:p-6" onSubmit={salvar}>
        <div className="flex flex-col gap-4 rounded-2xl border border-slate-100 bg-slate-50/70 p-4 sm:flex-row sm:items-center">
          <AdminIcon className="size-18 text-xl shadow-sm ring-4 ring-white" />
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold text-slate-800">Ícone do administrador</p>
            <p className="mt-1 text-xs leading-5 text-slate-500">Aparece no painel da sessão. Use uma imagem quadrada de até 2 MB.</p>
            <input ref={iconeInputRef} className="sr-only" type="file" accept="image/*,.ico" onChange={enviarIcone} />
            <div className="mt-3 flex flex-wrap gap-2">
              <Button type="button" loading={salvandoIcone} onClick={() => iconeInputRef.current?.click()}><Upload className="size-4" /> Trocar ícone</Button>
              {usuario?.tem_icone && <Button type="button" variant="secondary" disabled={salvandoIcone} onClick={() => void restaurarIcone()}><Trash2 className="size-4" /> Usar iniciais</Button>}
            </div>
          </div>
        </div>
        <div>
          <label className="field-label" htmlFor="admin-login">Usuário administrador</label>
          <div className="relative">
            <UserRound className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
            <input id="admin-login" className="field pl-10" autoComplete="username" minLength={3} maxLength={64} required value={login} onChange={(event) => setLogin(event.target.value)} />
          </div>
        </div>
        <div>
          <label className="field-label" htmlFor="admin-current-password">Senha atual</label>
          <PasswordField id="admin-current-password" value={senhaAtual} onChange={setSenhaAtual} visible={mostrarSenhas} autoComplete="current-password" required />
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className="field-label" htmlFor="admin-new-password">Nova senha</label>
            <PasswordField id="admin-new-password" value={novaSenha} onChange={setNovaSenha} visible={mostrarSenhas} autoComplete="new-password" placeholder="Deixe em branco para manter" />
          </div>
          <div>
            <label className="field-label" htmlFor="admin-confirm-password">Confirmar nova senha</label>
            <PasswordField id="admin-confirm-password" value={confirmacao} onChange={setConfirmacao} visible={mostrarSenhas} autoComplete="new-password" placeholder="Repita a nova senha" />
          </div>
        </div>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <button type="button" className="inline-flex items-center gap-2 text-xs font-medium text-slate-500 hover:text-slate-800" onClick={() => setMostrarSenhas((value) => !value)}>
            {mostrarSenhas ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            {mostrarSenhas ? "Ocultar senhas" : "Mostrar senhas"}
          </button>
          <Button type="submit" loading={salvando}><Save className="size-4" /> Salvar credenciais</Button>
        </div>
        <p className="rounded-xl bg-amber-50 px-3 py-2 text-xs leading-5 text-amber-800">Ao salvar, todas as sessões serão encerradas e será necessário entrar novamente.</p>
      </form>
    </section>
  );
}

function PasswordField({ id, value, onChange, visible, autoComplete, placeholder, required = false }: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  visible: boolean;
  autoComplete: string;
  placeholder?: string;
  required?: boolean;
}) {
  return (
    <div className="relative">
      <LockKeyhole className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
      <input id={id} className="field pl-10" type={visible ? "text" : "password"} autoComplete={autoComplete} required={required} maxLength={1024} value={value} placeholder={placeholder} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

export function ContactTransferManager() {
  const [importando, setImportando] = useState(false);
  const [resultado, setResultado] = useState<ImportacaoContatosResultado | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const { notify } = useToast();

  const importar = async (event: ChangeEvent<HTMLInputElement>) => {
    const arquivo = event.target.files?.[0];
    if (!arquivo) return;
    const form = new FormData();
    form.append("arquivo", arquivo);
    setImportando(true);
    setResultado(null);
    try {
      const resposta = await api.post<ImportacaoContatosResultado>(
        "/api/configuracoes/contatos/importar",
        form,
      );
      setResultado(resposta);
      notify(`${resposta.pessoas_importadas} contato(s) importado(s)`);
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setImportando(false);
      event.target.value = "";
    }
  };

  return (
    <section className="panel overflow-hidden xl:col-span-2">
      <header className="flex items-center gap-3 border-b border-slate-100 p-5 sm:p-6">
        <div className="grid size-11 place-items-center rounded-2xl bg-emerald-50 text-emerald-700"><ContactRound className="size-5" /></div>
        <div><h2 className="font-display text-xl font-semibold">Importar e exportar contatos</h2><p className="text-sm text-slate-500">Migre agendas sem depender de integrações externas.</p></div>
      </header>
      <div className="grid gap-5 p-5 sm:p-6 lg:grid-cols-2">
        <div className="rounded-2xl border border-dashed border-teal-300 bg-teal-50/60 p-5">
          <div className="flex items-start gap-3">
            <Upload className="mt-0.5 size-5 shrink-0 text-teal-700" />
            <div><h3 className="font-semibold text-slate-800">Importar agenda</h3><p className="mt-1 text-sm leading-6 text-slate-500">Aceita vCard/VCF, CSV do Google Contacts, Outlook e CSV genérico em UTF-8.</p></div>
          </div>
          <input ref={inputRef} className="sr-only" type="file" accept=".csv,.vcf,text/csv,text/vcard,text/x-vcard" onChange={importar} />
          <Button className="mt-4" type="button" loading={importando} onClick={() => inputRef.current?.click()}><FileArchive className="size-4" /> Selecionar arquivo</Button>
          {resultado && (
            <div className="mt-4 rounded-xl border border-emerald-200 bg-white p-3 text-sm text-emerald-800">
              <p className="font-semibold">{resultado.pessoas_importadas} pessoa(s) e {resultado.contatos_importados} meio(s) importados.</p>
              {resultado.registros_ignorados > 0 && <p className="mt-1">{resultado.registros_ignorados} registro(s) sem nome foram ignorados.</p>}
              {resultado.avisos.length > 0 && <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-amber-700">{resultado.avisos.map((aviso, index) => <li key={`${index}-${aviso}`}>{aviso}</li>)}</ul>}
            </div>
          )}
        </div>

        <div className="rounded-2xl border border-slate-200 bg-slate-50 p-5">
          <div className="flex items-start gap-3">
            <Download className="mt-0.5 size-5 shrink-0 text-sky-700" />
            <div><h3 className="font-semibold text-slate-800">Exportar agenda</h3><p className="mt-1 text-sm leading-6 text-slate-500">CSV preserva categorias e tipos do AgendarX; vCard oferece maior compatibilidade com celulares e serviços de contatos.</p></div>
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <a className="btn btn-secondary" href={apiUrl("/api/configuracoes/contatos/exportar/csv")} download><Download className="size-4" /> Exportar CSV</a>
            <a className="btn btn-secondary" href={apiUrl("/api/configuracoes/contatos/exportar/vcf")} download><RefreshCcw className="size-4" /> Exportar vCard</a>
          </div>
        </div>
      </div>
    </section>
  );
}
