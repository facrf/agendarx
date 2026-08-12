import {
  AlertTriangle,
  DatabaseZap,
  ExternalLink,
  FileSearch,
  FileText,
  History,
  Plus,
  Power,
  Radar,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import type { FormEvent } from "react";
import { useToast } from "../contexts/ToastContext";
import { api, apiUrl, errorMessage } from "../services/api";
import type {
  HistoricoBuscaPublica,
  ParametroBusca,
  TipoParametroBusca,
  VarreduraPublicaResponse,
} from "../types/api";
import { formatDate } from "../utils/format";
import { Button, EmptyState, Modal, Spinner, cn } from "./ui";

const TIPOS: Array<{ valor: TipoParametroBusca; rotulo: string }> = [
  { valor: "NOME", rotulo: "Nome" },
  { valor: "CPF", rotulo: "CPF" },
  { valor: "CNPJ", rotulo: "CNPJ" },
  { valor: "EMAIL", rotulo: "E-mail" },
  { valor: "TELEFONE", rotulo: "Telefone" },
  { valor: "TERMO", rotulo: "Termo livre" },
];

export function OSINTTab({ pessoaId }: { pessoaId: number }) {
  const [parametros, setParametros] = useState<ParametroBusca[]>([]);
  const [historico, setHistorico] = useState<HistoricoBuscaPublica[]>([]);
  const [tipo, setTipo] = useState<TipoParametroBusca>("NOME");
  const [valor, setValor] = useState("");
  const [carregando, setCarregando] = useState(true);
  const [salvando, setSalvando] = useState(false);
  const [varrendo, setVarrendo] = useState(false);
  const [resultado, setResultado] = useState<VarreduraPublicaResponse | null>(null);
  const [pdfAberto, setPdfAberto] = useState<HistoricoBuscaPublica | null>(null);
  const { notify } = useToast();

  const carregar = useCallback(async () => {
    try {
      const [parametrosData, historicoData] = await Promise.all([
        api.get<ParametroBusca[]>(`/api/osint/parametros/${pessoaId}`),
        api.get<HistoricoBuscaPublica[]>(`/api/osint/historico/${pessoaId}`),
      ]);
      setParametros(parametrosData);
      setHistorico(historicoData);
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setCarregando(false);
    }
  }, [pessoaId, notify]);

  useEffect(() => {
    void carregar();
  }, [carregar]);

  const adicionar = async (event: FormEvent) => {
    event.preventDefault();
    if (!valor.trim()) return;
    setSalvando(true);
    try {
      const criado = await api.post<ParametroBusca>(`/api/osint/parametros/${pessoaId}`, {
        tipo,
        valor: valor.trim(),
        ativo: true,
      });
      setParametros((atuais) => [criado, ...atuais]);
      setValor("");
      notify("Parâmetro de pesquisa adicionado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const alternarParametro = async (parametro: ParametroBusca) => {
    try {
      const atualizado = await api.put<ParametroBusca>(
        `/api/osint/parametros/item/${parametro.id}`,
        { tipo: parametro.tipo, valor: parametro.valor, ativo: !parametro.ativo },
      );
      setParametros((atuais) =>
        atuais.map((item) => (item.id === atualizado.id ? atualizado : item)),
      );
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const excluirParametro = async (parametro: ParametroBusca) => {
    if (!window.confirm(`Remover o parâmetro “${parametro.valor}”?`)) return;
    try {
      await api.delete(`/api/osint/parametros/item/${parametro.id}`);
      setParametros((atuais) => atuais.filter((item) => item.id !== parametro.id));
      notify("Parâmetro removido");
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  };

  const executarVarredura = async () => {
    setVarrendo(true);
    setResultado(null);
    try {
      const resumo = await api.post<VarreduraPublicaResponse>(
        `/api/osint/varrer/${pessoaId}`,
      );
      setResultado(resumo);
      setHistorico(await api.get(`/api/osint/historico/${pessoaId}`));
      notify(
        resumo.novos_achados > 0
          ? `${resumo.novos_achados} novo(s) achado(s) arquivado(s)`
          : "Varredura concluída sem novos achados",
      );
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setVarrendo(false);
    }
  };

  if (carregando) return <Spinner label="Carregando pesquisa pública" />;

  const ativos = parametros.filter((parametro) => parametro.ativo).length;

  return (
    <div className="space-y-7">
      <section className="rounded-3xl border border-teal-100 bg-teal-50/70 p-5">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex items-start gap-3">
            <div className="grid size-11 shrink-0 place-items-center rounded-2xl bg-white text-teal-700 shadow-sm">
              <ShieldCheck className="size-5" />
            </div>
            <div>
              <h2 className="font-display text-xl font-semibold text-slate-950">Pesquisa em fontes públicas</h2>
              <p className="mt-1 max-w-2xl text-sm leading-6 text-slate-600">
                Consulte somente dados de acesso público e mantenha uma finalidade legítima. Identificadores como CPF e CNPJ exigem cuidado especial no tratamento.
              </p>
            </div>
          </div>
          <Button
            type="button"
            loading={varrendo}
            disabled={ativos === 0}
            onClick={() => void executarVarredura()}
          >
            <Radar className="size-4" />
            {varrendo ? "Consultando fontes…" : "Iniciar varredura"}
          </Button>
        </div>
      </section>

      {resultado && <ResumoVarredura resultado={resultado} />}

      <div className="grid gap-6 xl:grid-cols-[22rem_1fr]">
        <section>
          <div className="mb-4">
            <p className="eyebrow">Critérios</p>
            <h2 className="font-display text-xl font-semibold">Parâmetros de busca</h2>
            <p className="mt-1 text-sm text-slate-500">{ativos} de {parametros.length} ativo(s)</p>
          </div>

          <form className="rounded-2xl border border-slate-200 bg-slate-50 p-4" onSubmit={adicionar}>
            <label className="field-label" htmlFor="tipo-parametro-osint">Tipo</label>
            <select
              id="tipo-parametro-osint"
              className="field"
              value={tipo}
              onChange={(event) => setTipo(event.target.value as TipoParametroBusca)}
            >
              {TIPOS.map((item) => <option key={item.valor} value={item.valor}>{item.rotulo}</option>)}
            </select>
            <label className="field-label mt-3" htmlFor="valor-parametro-osint">Valor pesquisado</label>
            <input
              id="valor-parametro-osint"
              className="field"
              value={valor}
              maxLength={512}
              placeholder={placeholderPara(tipo)}
              onChange={(event) => setValor(event.target.value)}
            />
            <Button className="mt-3 w-full" type="submit" loading={salvando} disabled={!valor.trim()}>
              <Plus className="size-4" /> Adicionar parâmetro
            </Button>
          </form>

          <div className="mt-4 space-y-2">
            {parametros.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-slate-200 p-5 text-center text-sm text-slate-400">
                Nenhum parâmetro configurado.
              </div>
            ) : parametros.map((parametro) => (
              <article
                key={parametro.id}
                className={cn(
                  "flex items-center gap-3 rounded-2xl border p-3 transition",
                  parametro.ativo ? "border-teal-100 bg-white" : "border-slate-100 bg-slate-50 opacity-65",
                )}
              >
                <button
                  type="button"
                  className={cn(
                    "grid size-9 shrink-0 place-items-center rounded-xl transition",
                    parametro.ativo ? "bg-teal-100 text-teal-800" : "bg-slate-200 text-slate-500",
                  )}
                  title={parametro.ativo ? "Desativar parâmetro" : "Ativar parâmetro"}
                  aria-label={parametro.ativo ? "Desativar parâmetro" : "Ativar parâmetro"}
                  onClick={() => void alternarParametro(parametro)}
                >
                  <Power className="size-4" />
                </button>
                <div className="min-w-0 flex-1">
                  <p className="text-[0.65rem] font-bold uppercase tracking-wider text-slate-400">{parametro.tipo}</p>
                  <p className="truncate text-sm font-medium text-slate-800" title={parametro.valor}>{parametro.valor}</p>
                </div>
                <button
                  type="button"
                  className="rounded-lg p-2 text-slate-400 transition hover:bg-rose-50 hover:text-rose-600"
                  title="Excluir parâmetro"
                  aria-label="Excluir parâmetro"
                  onClick={() => void excluirParametro(parametro)}
                >
                  <Trash2 className="size-4" />
                </button>
              </article>
            ))}
          </div>
        </section>

        <section>
          <div className="mb-4 flex items-end justify-between gap-3">
            <div>
              <p className="eyebrow">Arquivamento</p>
              <h2 className="font-display text-xl font-semibold">Linha do tempo de achados</h2>
            </div>
            <span className="chip"><History className="size-3.5" /> {historico.length}</span>
          </div>

          {historico.length === 0 ? (
            <EmptyState
              icon={<FileSearch className="size-7" />}
              title="Nenhum achado registrado"
              description="Configure os parâmetros e execute uma varredura. Resultados já arquivados não serão duplicados."
            />
          ) : (
            <div className="relative space-y-4 before:absolute before:bottom-5 before:left-[0.68rem] before:top-5 before:w-px before:bg-slate-200">
              {historico.map((item) => (
                <article key={item.id} className="relative pl-8">
                  <span className="absolute left-1.5 top-5 z-[1] size-3 rounded-full border-[3px] border-white bg-teal-600 shadow-sm" />
                  <div className="rounded-2xl border border-slate-100 bg-white p-4 shadow-sm transition hover:border-teal-100 hover:shadow-md sm:p-5">
                    <div className="flex flex-wrap items-center gap-2 text-xs text-slate-400">
                      <span className="font-semibold text-teal-700">{item.fonte}</span>
                      <span aria-hidden="true">•</span>
                      <time dateTime={item.data_captura}>{formatDate(item.data_captura, true)}</time>
                      <span className="chip ml-auto">{item.parametro_utilizado}</span>
                    </div>
                    <h3 className="mt-3 text-base font-semibold leading-6 text-slate-900">{item.titulo_resultado}</h3>
                    {item.snippet && <p className="mt-2 whitespace-pre-line text-sm leading-6 text-slate-600">{item.snippet}</p>}
                    <div className="mt-4 flex flex-wrap gap-2">
                      <a
                        className="btn btn-secondary"
                        href={item.url_origem}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        <ExternalLink className="size-4" /> Abrir fonte original
                      </a>
                      {item.url_pdf && (
                        <Button type="button" variant="secondary" onClick={() => setPdfAberto(item)}>
                          <FileText className="size-4" /> Visualizar PDF do Dossiê
                        </Button>
                      )}
                    </div>
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      </div>

      <Modal
        open={Boolean(pdfAberto)}
        onClose={() => setPdfAberto(null)}
        title={pdfAberto?.titulo_resultado || "PDF arquivado"}
        className="max-w-6xl"
      >
        {pdfAberto?.url_pdf && (
          <iframe
            className="h-[72vh] w-full rounded-xl border border-slate-200 bg-slate-100"
            src={apiUrl(pdfAberto.url_pdf)}
            title={`PDF: ${pdfAberto.titulo_resultado}`}
          />
        )}
      </Modal>
    </div>
  );
}

function ResumoVarredura({ resultado }: { resultado: VarreduraPublicaResponse }) {
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-4">
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Metrica rotulo="Parâmetros" valor={resultado.parametros_processados} />
        <Metrica rotulo="Resultados lidos" valor={resultado.resultados_encontrados} />
        <Metrica rotulo="Novos achados" valor={resultado.novos_achados} destaque />
        <Metrica rotulo="PDFs arquivados" valor={resultado.pdfs_arquivados} />
      </div>
      {resultado.avisos.length > 0 && (
        <details className="mt-4 rounded-xl bg-amber-50 p-3 text-sm text-amber-900">
          <summary className="flex cursor-pointer list-none items-center gap-2 font-semibold">
            <AlertTriangle className="size-4" /> {resultado.avisos.length} aviso(s) da varredura
          </summary>
          <ul className="mt-2 list-disc space-y-1 pl-6 text-xs leading-5">
            {resultado.avisos.map((aviso, indice) => <li key={`${indice}-${aviso}`}>{aviso}</li>)}
          </ul>
        </details>
      )}
    </section>
  );
}

function Metrica({ rotulo, valor, destaque = false }: { rotulo: string; valor: number; destaque?: boolean }) {
  return (
    <div className={cn("rounded-xl p-3", destaque ? "bg-teal-50" : "bg-slate-50")}>
      <div className="flex items-center gap-2 text-slate-400"><DatabaseZap className="size-3.5" /><span className="text-xs font-medium">{rotulo}</span></div>
      <p className={cn("mt-1 font-display text-2xl font-semibold", destaque ? "text-teal-800" : "text-slate-900")}>{valor}</p>
    </div>
  );
}

function placeholderPara(tipo: TipoParametroBusca): string {
  switch (tipo) {
    case "CPF": return "000.000.000-00";
    case "CNPJ": return "00.000.000/0000-00";
    case "EMAIL": return "nome@exemplo.com";
    case "TELEFONE": return "+55 11 99999-0000";
    case "TERMO": return "Termo, organização ou contexto";
    default: return "Nome completo";
  }
}
