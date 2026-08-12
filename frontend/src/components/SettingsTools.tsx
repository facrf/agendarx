import {
  ContactRound,
  Download,
  FileArchive,
  ImageIcon,
  RefreshCcw,
  Trash2,
  Upload,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ChangeEvent } from "react";
import { useToast } from "../contexts/ToastContext";
import { api, apiUrl, errorMessage } from "../services/api";
import type { IdentidadeVisual, ImportacaoContatosResultado } from "../types/api";
import { BrandIcon, refreshBranding } from "./BrandIcon";
import { Button } from "./ui";

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
            <input ref={inputRef} className="sr-only" type="file" accept="image/png,image/jpeg,image/webp,image/gif,image/x-icon,image/vnd.microsoft.icon" onChange={enviar} />
            <Button type="button" loading={salvando} onClick={() => inputRef.current?.click()}><Upload className="size-4" /> Trocar ícone</Button>
            {identidade?.tem_icone && <Button type="button" variant="secondary" disabled={salvando} onClick={() => void restaurar()}><Trash2 className="size-4" /> Restaurar padrão</Button>}
          </div>
        </div>
      </div>
    </section>
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
