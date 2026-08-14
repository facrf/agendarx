import {
  AlertTriangle,
  ChevronLeft,
  ChevronRight,
  DatabaseZap,
  ExternalLink,
  FileSearch,
  FileText,
  History,
  Pencil,
  Plus,
  Power,
  Radar,
  Search,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type {
  HistoricoBuscaPublica,
  HistoricoBuscaPaginado,
  FontePesquisaPublica,
  ParametroBusca,
  TipoParametroBusca,
  VarreduraPublicaResponse,
} from "../types/api";
import { formatDate } from "../utils/format";
import { AttachmentPreviewModal } from "./AttachmentPreview";
import { Button, EmptyState, Spinner, cn } from "./ui";

const TIPOS: Array<{ valor: TipoParametroBusca; rotulo: string }> = [
  { valor: "NOME", rotulo: "Nome" },
  { valor: "CPF", rotulo: "CPF" },
  { valor: "CNPJ", rotulo: "CNPJ" },
  { valor: "EMAIL", rotulo: "E-mail" },
  { valor: "TELEFONE", rotulo: "Telefone" },
  { valor: "TERMO", rotulo: "Termo livre" },
];

const PROVIDERS: Array<{
  valor: FontePesquisaPublica;
  rotulo: string;
  descricao: string;
}> = [
  { valor: "SEARXNG", rotulo: "SearXNG", descricao: "Busca geral na web." },
  { valor: "QUERIDO_DIARIO", rotulo: "Querido Diário", descricao: "Diários oficiais municipais brasileiros." },
  { valor: "INLABS", rotulo: "INLABS / DOU", descricao: "Publicações recentes do Diário Oficial da União." },
  { valor: "OPENALEX", rotulo: "OpenAlex", descricao: "Literatura e citações acadêmicas." },
];

type QuantidadePorPagina = 0 | 10 | 50 | 100;

function providerLabel(provider: FontePesquisaPublica): string {
  return PROVIDERS.find((item) => item.valor === provider)?.rotulo ?? provider;
}

export function OSINTTab({ pessoaId }: { pessoaId: number }) {
  const [parametros, setParametros] = useState<ParametroBusca[]>([]);
  const [historico, setHistorico] = useState<HistoricoBuscaPublica[]>([]);
  const [totalHistorico, setTotalHistorico] = useState(0);
  const [totalPaginas, setTotalPaginas] = useState(0);
  const [pagina, setPagina] = useState(1);
  const [porPagina, setPorPagina] = useState<QuantidadePorPagina>(10);
  const [buscaHistorico, setBuscaHistorico] = useState("");
  const [buscaAplicada, setBuscaAplicada] = useState("");
  const [tipo, setTipo] = useState<TipoParametroBusca>("NOME");
  const [provider, setProvider] = useState<FontePesquisaPublica>("SEARXNG");
  const [valor, setValor] = useState("");
  const [editandoId, setEditandoId] = useState<number | null>(null);
  const [carregando, setCarregando] = useState(true);
  const [carregandoHistorico, setCarregandoHistorico] = useState(true);
  const [salvando, setSalvando] = useState(false);
  const [varrendo, setVarrendo] = useState(false);
  const [resultado, setResultado] = useState<VarreduraPublicaResponse | null>(null);
  const [pdfAberto, setPdfAberto] = useState<HistoricoBuscaPublica | null>(null);
  const [excluindoAchadoId, setExcluindoAchadoId] = useState<number | null>(null);
  const consultaHistoricoAtual = useRef(0);
  const { notify } = useToast();

  const carregarParametros = useCallback(async () => {
    try {
      setParametros(await api.get<ParametroBusca[]>(`/api/osint/parametros/${pessoaId}`));
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setCarregando(false);
    }
  }, [pessoaId, notify]);

  useEffect(() => {
    void carregarParametros();
  }, [carregarParametros]);

  const carregarHistorico = useCallback(async () => {
    const idConsulta = ++consultaHistoricoAtual.current;
    setCarregandoHistorico(true);
    const query = new URLSearchParams({
      pagina: String(pagina),
      por_pagina: String(porPagina),
    });
    if (buscaAplicada) query.set("busca", buscaAplicada);
    try {
      const data = await api.get<HistoricoBuscaPaginado>(
        `/api/osint/historico/${pessoaId}?${query.toString()}`,
      );
      if (idConsulta !== consultaHistoricoAtual.current) return;
      if (data.total_paginas > 0 && pagina > data.total_paginas && porPagina !== 0) {
        setPagina(data.total_paginas);
        return;
      }
      setHistorico(data.itens);
      setTotalHistorico(data.total);
      setTotalPaginas(data.total_paginas);
    } catch (error) {
      if (idConsulta === consultaHistoricoAtual.current) notify(errorMessage(error), "erro");
    } finally {
      if (idConsulta === consultaHistoricoAtual.current) setCarregandoHistorico(false);
    }
  }, [buscaAplicada, pagina, pessoaId, porPagina, notify]);

  useEffect(() => {
    void carregarHistorico();
  }, [carregarHistorico]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      setPagina(1);
      setBuscaAplicada(buscaHistorico.trim());
    }, 300);
    return () => window.clearTimeout(timeout);
  }, [buscaHistorico]);

  const salvarParametro = async (event: FormEvent) => {
    event.preventDefault();
    if (!valor.trim()) return;
    setSalvando(true);
    try {
      const atual = parametros.find((item) => item.id === editandoId);
      const payload = {
        tipo,
        valor: valor.trim(),
        provider,
        ativo: atual?.ativo ?? true,
      };
      if (editandoId !== null) {
        const atualizado = await api.put<ParametroBusca>(
          `/api/osint/parametros/item/${editandoId}`,
          payload,
        );
        setParametros((atuais) =>
          atuais.map((item) => (item.id === atualizado.id ? atualizado : item)),
        );
        notify("Pesquisa atualizada");
      } else {
        const criado = await api.post<ParametroBusca>(
          `/api/osint/parametros/${pessoaId}`,
          payload,
        );
        setParametros((atuais) => [criado, ...atuais]);
        notify("Parâmetro de pesquisa adicionado");
      }
      cancelarEdicao();
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const editarParametro = (parametro: ParametroBusca) => {
    setEditandoId(parametro.id);
    setTipo(parametro.tipo);
    setProvider(parametro.provider);
    setValor(parametro.valor);
  };

  const cancelarEdicao = () => {
    setEditandoId(null);
    setTipo("NOME");
    setProvider("SEARXNG");
    setValor("");
  };

  const alternarParametro = async (parametro: ParametroBusca) => {
    try {
      const atualizado = await api.put<ParametroBusca>(
        `/api/osint/parametros/item/${parametro.id}`,
        { tipo: parametro.tipo, valor: parametro.valor, provider: parametro.provider, ativo: !parametro.ativo },
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
      if (editandoId === parametro.id) cancelarEdicao();
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
      if (pagina === 1) await carregarHistorico();
      else setPagina(1);
      if (resumo.situacao === "inconclusiva") {
        notify("Varredura inconclusiva: consulte as fontes indisponíveis", "erro");
      } else if (resumo.novos_achados > 0) {
        notify(`${resumo.novos_achados} novo(s) achado(s) arquivado(s)`);
      } else if (resumo.situacao === "parcial") {
        notify("Varredura parcial concluída sem novos achados");
      } else {
        notify("Varredura concluída sem novos achados");
      }
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setVarrendo(false);
    }
  };

  const excluirAchado = async (achado: HistoricoBuscaPublica) => {
    if (!window.confirm(
      `Excluir “${achado.titulo_resultado}” da linha do tempo? O PDF arquivado, se houver, será mantido no dossiê.`,
    )) return;
    setExcluindoAchadoId(achado.id);
    try {
      await api.delete(`/api/osint/historico/item/${achado.id}`);
      if (pdfAberto?.id === achado.id) setPdfAberto(null);
      notify("Achado removido da linha do tempo");
      if (historico.length === 1 && pagina > 1) setPagina((atual) => atual - 1);
      else await carregarHistorico();
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setExcluindoAchadoId(null);
    }
  };

  if (carregando) return <Spinner label="Carregando pesquisa pública" />;

  const ativos = parametros.filter((parametro) => parametro.ativo).length;
  const primeiroItem = totalHistorico === 0
    ? 0
    : porPagina === 0 ? 1 : (pagina - 1) * porPagina + 1;
  const ultimoItem = porPagina === 0
    ? totalHistorico
    : Math.min(pagina * porPagina, totalHistorico);

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

          <form className="rounded-2xl border border-slate-200 bg-slate-50 p-4" onSubmit={salvarParametro}>
            <label className="field-label" htmlFor="provider-parametro-osint">Fonte de pesquisa</label>
            <select
              id="provider-parametro-osint"
              className="field"
              value={provider}
              onChange={(event) => setProvider(event.target.value as FontePesquisaPublica)}
            >
              {PROVIDERS.map((item) => <option key={item.valor} value={item.valor}>{item.rotulo}</option>)}
            </select>
            <p className="mt-1.5 text-xs leading-5 text-slate-500">
              {PROVIDERS.find((item) => item.valor === provider)?.descricao}
            </p>
            <label className="field-label mt-3" htmlFor="tipo-parametro-osint">Tipo</label>
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
            <div className="mt-3 flex gap-2">
              <Button className="flex-1" type="submit" loading={salvando} disabled={!valor.trim()}>
                {editandoId === null ? <Plus className="size-4" /> : <Pencil className="size-4" />}
                {editandoId === null ? "Adicionar parâmetro" : "Salvar alterações"}
              </Button>
              {editandoId !== null && (
                <Button type="button" variant="secondary" title="Cancelar edição" onClick={cancelarEdicao}>
                  <X className="size-4" />
                </Button>
              )}
            </div>
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
                  <div className="flex flex-wrap items-center gap-1.5">
                    <span className="text-[0.65rem] font-bold uppercase tracking-wider text-slate-400">{parametro.tipo}</span>
                    <span className="rounded-full bg-sky-50 px-2 py-0.5 text-[0.62rem] font-semibold text-sky-700">
                      {providerLabel(parametro.provider)}
                    </span>
                  </div>
                  <p className="truncate text-sm font-medium text-slate-800" title={parametro.valor}>{parametro.valor}</p>
                </div>
                <button
                  type="button"
                  className="rounded-lg p-2 text-slate-400 transition hover:bg-teal-50 hover:text-teal-700"
                  title="Editar pesquisa"
                  aria-label="Editar pesquisa"
                  onClick={() => editarParametro(parametro)}
                >
                  <Pencil className="size-4" />
                </button>
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

        <section className="min-w-0">
          <div className="mb-4 flex items-end justify-between gap-3">
            <div>
              <p className="eyebrow">Arquivamento</p>
              <h2 className="font-display text-xl font-semibold">Linha do tempo de achados</h2>
            </div>
            <span className="chip shrink-0"><History className="size-3.5" /> {totalHistorico}</span>
          </div>

          <div className="mb-5 grid gap-3 rounded-2xl border border-slate-200 bg-slate-50 p-3 sm:grid-cols-[minmax(0,1fr)_auto]">
            <label className="relative block">
              <span className="sr-only">Pesquisar na linha do tempo</span>
              <Search className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
              <input
                className="field pl-10 pr-10"
                type="search"
                value={buscaHistorico}
                maxLength={200}
                placeholder="Pesquisar título, fonte, trecho ou parâmetro…"
                onChange={(event) => setBuscaHistorico(event.target.value)}
              />
              {buscaHistorico && (
                <button
                  type="button"
                  className="absolute right-2 top-1/2 grid size-8 -translate-y-1/2 place-items-center rounded-lg text-slate-400 hover:bg-white hover:text-slate-700"
                  onClick={() => setBuscaHistorico("")}
                  aria-label="Limpar pesquisa"
                >
                  <X className="size-4" />
                </button>
              )}
            </label>
            <label className="flex items-center gap-2 text-sm font-medium text-slate-600">
              <span className="whitespace-nowrap">Por página</span>
              <select
                className="field min-w-24"
                value={porPagina}
                onChange={(event) => {
                  setPorPagina(Number(event.target.value) as QuantidadePorPagina);
                  setPagina(1);
                }}
              >
                <option value={10}>10</option>
                <option value={50}>50</option>
                <option value={100}>100</option>
                <option value={0}>Todos</option>
              </select>
            </label>
          </div>

          {carregandoHistorico ? (
            <Spinner label="Consultando a linha do tempo" />
          ) : historico.length === 0 ? (
            <EmptyState
              icon={<FileSearch className="size-7" />}
              title={buscaAplicada ? "Nenhum achado encontrado" : "Nenhum achado registrado"}
              description={buscaAplicada
                ? "Tente outro termo ou limpe a pesquisa para voltar a exibir toda a linha do tempo."
                : "Configure os parâmetros e execute uma varredura. Resultados já arquivados não serão duplicados."}
            />
          ) : (
            <>
              <div className="relative space-y-4 before:absolute before:bottom-5 before:left-[0.68rem] before:top-5 before:w-px before:bg-slate-200">
                {historico.map((item) => (
                  <article key={item.id} className="relative pl-8">
                    <span className="absolute left-1.5 top-5 z-[1] size-3 rounded-full border-[3px] border-white bg-teal-600 shadow-sm" />
                    <div className="rounded-2xl border border-slate-100 bg-white p-4 shadow-sm transition hover:border-teal-100 hover:shadow-md sm:p-5">
                      <div className="flex flex-wrap items-center gap-2 text-xs text-slate-400">
                        <span className="rounded-full bg-teal-50 px-2 py-1 font-semibold text-teal-800">
                          {providerLabel(item.provider)}
                        </span>
                        <span className="font-semibold text-slate-600">{item.fonte}</span>
                        <span aria-hidden="true">•</span>
                        <time dateTime={item.data_captura}>{formatDate(item.data_captura, true)}</time>
                        <span className="chip sm:ml-auto">{item.parametro_utilizado}</span>
                      </div>
                      <h3 className="mt-3 break-words text-base font-semibold leading-6 text-slate-900">{item.titulo_resultado}</h3>
                      {item.data_publicacao && (
                        <p className="mt-1 text-xs font-medium text-slate-500">
                          Publicado em: {formatDate(item.data_publicacao)}
                        </p>
                      )}
                      {item.snippet && <p className="mt-2 whitespace-pre-line break-words text-sm leading-6 text-slate-600">{item.snippet}</p>}
                      {item.detalhes && (
                        <p className="mt-3 whitespace-pre-line break-words rounded-xl bg-slate-50 p-3 text-xs leading-5 text-slate-600">
                          {item.detalhes}
                        </p>
                      )}
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
                        <Button
                          className="sm:ml-auto"
                          type="button"
                          variant="danger"
                          loading={excluindoAchadoId === item.id}
                          disabled={excluindoAchadoId !== null}
                          onClick={() => void excluirAchado(item)}
                        >
                          <Trash2 className="size-4" /> Excluir resultado
                        </Button>
                      </div>
                    </div>
                  </article>
                ))}
              </div>

              <nav className="mt-5 flex flex-col gap-3 rounded-2xl border border-slate-200 bg-white p-3 sm:flex-row sm:items-center sm:justify-between" aria-label="Paginação da linha do tempo">
                <p className="text-center text-xs text-slate-500 sm:text-left" aria-live="polite">
                  Exibindo <strong>{primeiroItem}–{ultimoItem}</strong> de <strong>{totalHistorico}</strong> resultado(s)
                </p>
                {porPagina !== 0 && totalPaginas > 1 && (
                  <div className="flex items-center justify-center gap-2">
                    <Button type="button" variant="secondary" disabled={pagina <= 1} onClick={() => setPagina((atual) => atual - 1)}>
                      <ChevronLeft className="size-4" /> Anterior
                    </Button>
                    <span className="min-w-20 text-center text-xs font-semibold text-slate-600">
                      {pagina} de {totalPaginas}
                    </span>
                    <Button type="button" variant="secondary" disabled={pagina >= totalPaginas} onClick={() => setPagina((atual) => atual + 1)}>
                      Próxima <ChevronRight className="size-4" />
                    </Button>
                  </div>
                )}
              </nav>
            </>
          )}
        </section>
      </div>

      <AttachmentPreviewModal
        attachment={pdfAberto?.url_pdf && pdfAberto.anexo_dossie_id ? {
          nome_arquivo: `${pdfAberto.titulo_resultado}.pdf`,
          mime_type: "application/pdf",
          url_stream: pdfAberto.url_pdf,
          url_download: `/api/dossie/anexos/${pdfAberto.anexo_dossie_id}/download`,
        } : null}
        onClose={() => setPdfAberto(null)}
      />
    </div>
  );
}

function ResumoVarredura({ resultado }: { resultado: VarreduraPublicaResponse }) {
  const situacoes = {
    concluida: {
      titulo: "Varredura concluída",
      descricao: "As fontes consultadas responderam normalmente.",
      classe: "border-emerald-200 bg-emerald-50 text-emerald-900",
    },
    parcial: {
      titulo: "Varredura parcial",
      descricao: "A pesquisa encontrou respostas, mas uma ou mais fontes ficaram indisponíveis.",
      classe: "border-amber-200 bg-amber-50 text-amber-900",
    },
    inconclusiva: {
      titulo: "Varredura inconclusiva",
      descricao: "As fontes falharam; a ausência de resultados não confirma que não existam achados.",
      classe: "border-rose-200 bg-rose-50 text-rose-900",
    },
  } as const;
  const situacao = situacoes[resultado.situacao];

  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-4">
      <div className={cn("mb-4 rounded-xl border p-3", situacao.classe)}>
        <div className="flex items-start gap-2">
          {resultado.situacao === "concluida" ? (
            <ShieldCheck className="mt-0.5 size-4 shrink-0" />
          ) : (
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          )}
          <div>
            <p className="text-sm font-semibold">{situacao.titulo}</p>
            <p className="mt-0.5 text-xs leading-5 opacity-80">{situacao.descricao}</p>
          </div>
        </div>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <Metrica rotulo="Parâmetros" valor={resultado.parametros_processados} />
        <Metrica rotulo="Inconclusivos" valor={resultado.parametros_inconclusivos} />
        <Metrica rotulo="Resultados lidos" valor={resultado.resultados_encontrados} />
        <Metrica rotulo="Novos achados" valor={resultado.novos_achados} destaque />
        <Metrica rotulo="PDFs arquivados" valor={resultado.pdfs_arquivados} />
        <Metrica rotulo="Fontes indisponíveis" valor={resultado.fontes_indisponiveis} />
      </div>
      {resultado.avisos.length > 0 && (
        <details
          className="mt-4 rounded-xl bg-amber-50 p-3 text-sm text-amber-900"
          open={resultado.situacao === "inconclusiva"}
        >
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
