import {
  CalendarCheck,
  CalendarDays,
  Check,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  Clock3,
  Edit3,
  GripVertical,
  History,
  ListFilter,
  Paperclip,
  Plus,
  Repeat2,
  Search,
  Trash2,
  UserRound,
  UsersRound,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, DragEvent, FormEvent } from "react";
import { Link, useSearchParams } from "react-router-dom";
import {
  TaskAttachmentEditor,
  taskFileKey,
} from "../components/TaskAttachmentEditor";
import type { TaskUploadState } from "../components/TaskAttachmentEditor";
import { Button, EmptyState, Modal, PageHeader, Spinner, cn } from "../components/ui";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import { formatBytes } from "../utils/format";
import type {
  PessoaResumo,
  PrioridadeTarefa,
  StatusTarefa,
  TarefaCalendario,
  TarefaCalendarioPayload,
  AnexoTarefaCalendario,
  ArmazenamentoTarefas,
  HistoricoTarefa,
  RecorrenciaTarefa,
} from "../types/api";

const DIAS_SEMANA = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
const CORES = ["#13716D", "#2563EB", "#7C3AED", "#DB2777", "#E7654F", "#D97706"];
const TIPO_ARRASTE_TAREFA = "application/x-agendarx-tarefa";

interface TarefaFormState {
  titulo: string;
  descricao: string;
  data: string;
  inicioHora: string;
  fimHora: string;
  diaInteiro: boolean;
  status: StatusTarefa;
  prioridade: PrioridadeTarefa;
  corHex: string;
  pessoasIds: number[];
  recorrencia: RecorrenciaTarefa;
  recorrenciaFim: string;
  lembreteMinutos: string;
}

export function CalendarPage() {
  const hoje = useMemo(() => new Date(), []);
  const [searchParams, setSearchParams] = useSearchParams();
  const tarefaDeepLinkCarregada = useRef<number | null>(null);
  const [mesAtual, setMesAtual] = useState(() => inicioDoMes(hoje));
  const [tarefas, setTarefas] = useState<TarefaCalendario[]>([]);
  const [pessoas, setPessoas] = useState<PessoaResumo[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [modalAberto, setModalAberto] = useState(false);
  const [tarefaEditando, setTarefaEditando] = useState<TarefaCalendario | null>(null);
  const [form, setForm] = useState<TarefaFormState>(() => novoFormulario(chaveData(hoje)));
  const [buscaPessoa, setBuscaPessoa] = useState("");
  const [arquivosPendentes, setArquivosPendentes] = useState<File[]>([]);
  const [tarefaArrastadaId, setTarefaArrastadaId] = useState<number | null>(null);
  const [diaDestino, setDiaDestino] = useState<string | null>(null);
  const [tarefaMovendoId, setTarefaMovendoId] = useState<number | null>(null);
  const [anexoExcluindoId, setAnexoExcluindoId] = useState<number | null>(null);
  const [armazenamento, setArmazenamento] = useState<ArmazenamentoTarefas | null>(null);
  const [uploadStates, setUploadStates] = useState<Record<string, TaskUploadState>>({});
  const [historico, setHistorico] = useState<HistoricoTarefa[]>([]);
  const [carregandoHistorico, setCarregandoHistorico] = useState(false);
  const [tarefaStatusId, setTarefaStatusId] = useState<number | null>(null);
  const [tarefaMoverMobile, setTarefaMoverMobile] = useState<TarefaCalendario | null>(null);
  const [dataMoverMobile, setDataMoverMobile] = useState("");
  const [buscaTarefa, setBuscaTarefa] = useState("");
  const [filtroPessoa, setFiltroPessoa] = useState("");
  const [filtroStatus, setFiltroStatus] = useState("");
  const [filtroPrioridade, setFiltroPrioridade] = useState("");
  const [filtroAnexos, setFiltroAnexos] = useState("");
  const [filtroInicio, setFiltroInicio] = useState("");
  const [filtroFim, setFiltroFim] = useState("");
  const [salvando, setSalvando] = useState(false);
  const { notify } = useToast();

  const diasDoCalendario = useMemo(() => montarDiasDoCalendario(mesAtual), [mesAtual]);

  const carregarArmazenamento = useCallback(async () => {
    try {
      setArmazenamento(await api.get<ArmazenamentoTarefas>("/api/calendario/armazenamento"));
    } catch (error) {
      notify(errorMessage(error), "erro");
    }
  }, [notify]);

  const carregarHistorico = useCallback(async (id: number) => {
    setCarregandoHistorico(true);
    try {
      setHistorico(await api.get<HistoricoTarefa[]>(`/api/calendario/tarefas/${id}/historico`));
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setCarregandoHistorico(false);
    }
  }, [notify]);

  const carregarTarefas = useCallback(async () => {
    const primeiroDia = diasDoCalendario[0];
    const ultimoDia = adicionarDias(diasDoCalendario[diasDoCalendario.length - 1], 2);
    const margemInicial = adicionarDias(primeiroDia, -1);
    const params = new URLSearchParams({
      inicio: new Date(Date.UTC(
        margemInicial.getFullYear(),
        margemInicial.getMonth(),
        margemInicial.getDate(),
      )).toISOString(),
      fim: new Date(Date.UTC(
        ultimoDia.getFullYear(),
        ultimoDia.getMonth(),
        ultimoDia.getDate(),
      )).toISOString(),
    });
    try {
      const data = await api.get<TarefaCalendario[]>(`/api/calendario/tarefas?${params}`);
      setTarefas(data);
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setCarregando(false);
    }
  }, [diasDoCalendario, notify]);

  useEffect(() => {
    void carregarTarefas();
  }, [carregarTarefas]);

  useEffect(() => {
    api.get<PessoaResumo[]>("/api/pessoas")
      .then(setPessoas)
      .catch((error) => notify(errorMessage(error), "erro"));
  }, [notify]);

  useEffect(() => {
    void carregarArmazenamento();
  }, [carregarArmazenamento]);

  const deepLinkId = Number(searchParams.get("tarefa"));
  useEffect(() => {
    if (!Number.isInteger(deepLinkId) || deepLinkId <= 0) return;
    if (tarefaDeepLinkCarregada.current === deepLinkId) return;
    tarefaDeepLinkCarregada.current = deepLinkId;
    api.get<TarefaCalendario>(`/api/calendario/tarefas/${deepLinkId}`)
      .then((tarefa) => {
        setMesAtual(inicioDoMes(dataDaTarefa(tarefa)));
        setTarefaEditando(tarefa);
        setForm(formularioDaTarefa(tarefa));
        setBuscaPessoa("");
        setArquivosPendentes([]);
        setUploadStates({});
        setModalAberto(true);
        void carregarHistorico(tarefa.id);
      })
      .catch((error) => {
        notify(errorMessage(error), "erro");
        setSearchParams({}, { replace: true });
      });
  }, [carregarHistorico, deepLinkId, notify, setSearchParams]);

  const tarefasFiltradas = useMemo(() => {
    const termo = buscaTarefa.trim().toLocaleLowerCase("pt-BR");
    return tarefas.filter((tarefa) => {
      const data = chaveDaTarefa(tarefa);
      const texto = `${tarefa.titulo} ${tarefa.descricao || ""} ${tarefa.pessoas.map((pessoa) => pessoa.nome).join(" ")}`.toLocaleLowerCase("pt-BR");
      return (!termo || texto.includes(termo))
        && (!filtroPessoa || tarefa.pessoas.some((pessoa) => pessoa.id === Number(filtroPessoa)))
        && (!filtroStatus || tarefa.status === filtroStatus)
        && (!filtroPrioridade || tarefa.prioridade === filtroPrioridade)
        && (!filtroAnexos || (filtroAnexos === "COM" ? tarefa.anexos.length > 0 : tarefa.anexos.length === 0))
        && (!filtroInicio || data >= filtroInicio)
        && (!filtroFim || data <= filtroFim);
    });
  }, [buscaTarefa, filtroAnexos, filtroFim, filtroInicio, filtroPessoa, filtroPrioridade, filtroStatus, tarefas]);

  const tarefasPorDia = useMemo(() => {
    const mapa = new Map<string, TarefaCalendario[]>();
    tarefasFiltradas.forEach((tarefa) => {
      const chave = chaveDaTarefa(tarefa);
      const itens = mapa.get(chave) || [];
      itens.push(tarefa);
      mapa.set(chave, itens);
    });
    mapa.forEach((itens) => itens.sort(ordenarTarefas));
    return mapa;
  }, [tarefasFiltradas]);

  const filtrosAtivos = [buscaTarefa, filtroPessoa, filtroStatus, filtroPrioridade, filtroAnexos, filtroInicio, filtroFim]
    .filter(Boolean).length;

  const limparFiltros = () => {
    setBuscaTarefa("");
    setFiltroPessoa("");
    setFiltroStatus("");
    setFiltroPrioridade("");
    setFiltroAnexos("");
    setFiltroInicio("");
    setFiltroFim("");
  };

  const abrirNova = (data = chaveData(new Date())) => {
    setTarefaEditando(null);
    setForm(novoFormulario(data));
    setBuscaPessoa("");
    setArquivosPendentes([]);
    setUploadStates({});
    setHistorico([]);
    setModalAberto(true);
  };

  const abrirEdicao = (tarefa: TarefaCalendario) => {
    setTarefaEditando(tarefa);
    setForm(formularioDaTarefa(tarefa));
    setBuscaPessoa("");
    setArquivosPendentes([]);
    setUploadStates({});
    setModalAberto(true);
    void carregarHistorico(tarefa.id);
  };

  const fecharModal = () => {
    if (salvando) return;
    setModalAberto(false);
    setTarefaEditando(null);
    setArquivosPendentes([]);
    setUploadStates({});
    setHistorico([]);
    tarefaDeepLinkCarregada.current = null;
    if (searchParams.has("tarefa")) setSearchParams({}, { replace: true });
  };

  const salvar = async (event: FormEvent) => {
    event.preventDefault();
    const titulo = form.titulo.trim();
    if (!titulo) return notify("Informe o título da tarefa", "erro");

    const inicioEm = form.diaInteiro
      ? `${form.data}T00:00:00.000Z`
      : dataHoraLocalParaIso(form.data, form.inicioHora);
    const fimEm = !form.diaInteiro && form.fimHora
      ? dataHoraLocalParaIso(form.data, form.fimHora)
      : null;
    if (!inicioEm || (!form.diaInteiro && !form.inicioHora)) {
      return notify("Informe uma data e um horário válidos", "erro");
    }
    if (fimEm && new Date(fimEm) <= new Date(inicioEm)) {
      return notify("O horário de término deve ser posterior ao início", "erro");
    }
    const recorrenciaFimEm = form.recorrencia === "NENHUMA"
      ? null
      : form.recorrenciaFim
        ? `${form.recorrenciaFim}T23:59:59.999Z`
        : null;
    if (form.recorrencia !== "NENHUMA" && !recorrenciaFimEm) {
      return notify("Informe até quando a tarefa deve se repetir", "erro");
    }
    if (recorrenciaFimEm && new Date(recorrenciaFimEm) <= new Date(inicioEm)) {
      return notify("O fim da recorrência deve ser posterior à primeira tarefa", "erro");
    }

    const payload: TarefaCalendarioPayload = {
      titulo,
      descricao: form.descricao.trim() || null,
      inicio_em: inicioEm,
      fim_em: fimEm,
      dia_inteiro: form.diaInteiro,
      status: form.status,
      prioridade: form.prioridade,
      cor_hex: form.corHex,
      pessoas_ids: form.pessoasIds,
      recorrencia: form.recorrencia,
      recorrencia_fim_em: recorrenciaFimEm,
      lembrete_minutos: form.lembreteMinutos === "" ? null : Number(form.lembreteMinutos),
    };

    setSalvando(true);
    try {
      const estavaEditando = Boolean(tarefaEditando);
      let tarefaSalva: TarefaCalendario;
      if (tarefaEditando) {
        tarefaSalva = await api.put<TarefaCalendario>(`/api/calendario/tarefas/${tarefaEditando.id}`, payload);
      } else {
        tarefaSalva = await api.post<TarefaCalendario>("/api/calendario/tarefas", payload);
      }

      const falharam: File[] = [];
      // Upload sequencial evita concentrar vários arquivos grandes na memória do
      // navegador e do servidor ao mesmo tempo.
      for (const arquivo of arquivosPendentes) {
        try {
          await enviarArquivo(tarefaSalva.id, arquivo);
        } catch {
          falharam.push(arquivo);
        }
      }
      await Promise.all([carregarTarefas(), carregarArmazenamento()]);
      if (falharam.length > 0) {
        const atualizada = await api.get<TarefaCalendario>(`/api/calendario/tarefas/${tarefaSalva.id}`);
        setTarefaEditando(atualizada);
        setForm(formularioDaTarefa(atualizada));
        setArquivosPendentes(falharam);
        await carregarHistorico(atualizada.id);
        notify(`Tarefa salva, mas ${falharam.length} anexo(s) precisam ser reenviados`, "erro");
      } else {
        const recorrencias = !estavaEditando && tarefaSalva.total_ocorrencias > 1
          ? ` · ${tarefaSalva.total_ocorrencias} ocorrências criadas`
          : "";
        notify(`${estavaEditando ? "Tarefa atualizada" : "Tarefa agendada"}${recorrencias}`);
        setModalAberto(false);
        setTarefaEditando(null);
        setArquivosPendentes([]);
        setUploadStates({});
        setHistorico([]);
        if (searchParams.has("tarefa")) setSearchParams({}, { replace: true });
      }
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const enviarArquivo = async (tarefaId: number, arquivo: File) => {
    const key = taskFileKey(arquivo);
    setUploadStates((atuais) => ({ ...atuais, [key]: { progress: 0, status: "uploading" } }));
    const dados = new FormData();
    dados.append("arquivo", arquivo);
    try {
      const anexo = await api.upload<AnexoTarefaCalendario>(
        `/api/calendario/tarefas/${tarefaId}/anexos`,
        dados,
        (progress) => setUploadStates((atuais) => ({
          ...atuais,
          [key]: { progress, status: "uploading" },
        })),
      );
      setArquivosPendentes((atuais) => atuais.filter((item) => taskFileKey(item) !== key));
      setUploadStates((atuais) => {
        const proximos = { ...atuais };
        delete proximos[key];
        return proximos;
      });
      return anexo;
    } catch (error) {
      setUploadStates((atuais) => ({
        ...atuais,
        [key]: { progress: atuais[key]?.progress || 0, status: "error", message: errorMessage(error) },
      }));
      throw error;
    }
  };

  const repetirUpload = async (arquivo: File) => {
    if (!tarefaEditando) return;
    setSalvando(true);
    try {
      await enviarArquivo(tarefaEditando.id, arquivo);
      const atualizada = await api.get<TarefaCalendario>(`/api/calendario/tarefas/${tarefaEditando.id}`);
      setTarefaEditando(atualizada);
      setTarefas((atuais) => atuais.map((item) => item.id === atualizada.id ? atualizada : item));
      await Promise.all([carregarArmazenamento(), carregarHistorico(atualizada.id)]);
      notify("Anexo enviado");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const excluir = async () => {
    if (!tarefaEditando || !window.confirm(`Excluir a tarefa “${tarefaEditando.titulo}”?`)) return;
    setSalvando(true);
    try {
      await api.delete(`/api/calendario/tarefas/${tarefaEditando.id}`);
      setModalAberto(false);
      setTarefaEditando(null);
      setArquivosPendentes([]);
      setUploadStates({});
      setHistorico([]);
      tarefaDeepLinkCarregada.current = null;
      if (searchParams.has("tarefa")) setSearchParams({}, { replace: true });
      notify("Tarefa excluída");
      await Promise.all([carregarTarefas(), carregarArmazenamento()]);
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const excluirAnexo = async (anexo: AnexoTarefaCalendario) => {
    if (!window.confirm(`Excluir o anexo “${anexo.nome_arquivo}”?`)) return;
    setAnexoExcluindoId(anexo.id);
    try {
      await api.delete(`/api/calendario/anexos/${anexo.id}`);
      const remover = (tarefa: TarefaCalendario) => ({
        ...tarefa,
        anexos: tarefa.anexos.filter((item) => item.id !== anexo.id),
      });
      setTarefaEditando((atual) => atual ? remover(atual) : atual);
      setTarefas((atuais) => atuais.map((tarefa) => tarefa.id === anexo.tarefa_id ? remover(tarefa) : tarefa));
      await Promise.all([carregarArmazenamento(), carregarHistorico(anexo.tarefa_id)]);
      notify("Anexo excluído");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setAnexoExcluindoId(null);
    }
  };

  const iniciarArraste = (event: DragEvent<HTMLElement>, tarefa: TarefaCalendario) => {
    if (tarefaMovendoId !== null) {
      event.preventDefault();
      return;
    }
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData(TIPO_ARRASTE_TAREFA, String(tarefa.id));
    event.dataTransfer.setData("text/plain", tarefa.titulo);
    setTarefaArrastadaId(tarefa.id);
  };

  const encerrarArraste = () => {
    setTarefaArrastadaId(null);
    setDiaDestino(null);
  };

  const soltarTarefa = (event: DragEvent<HTMLDivElement>, destino: string) => {
    const id = Number(event.dataTransfer.getData(TIPO_ARRASTE_TAREFA));
    if (!Number.isInteger(id) || id <= 0) return;
    event.preventDefault();
    event.stopPropagation();
    encerrarArraste();
    void moverTarefa(id, destino);
  };

  const moverTarefa = async (id: number, destino: string): Promise<boolean> => {
    const tarefa = tarefas.find((item) => item.id === id);
    if (!tarefa || tarefaMovendoId !== null) return false;
    if (chaveDaTarefa(tarefa) === destino) return true;

    const novasDatas = datasAoMoverTarefa(tarefa, destino);
    const otimista = { ...tarefa, ...novasDatas };
    setTarefaMovendoId(id);
    setTarefas((atuais) => atuais.map((item) => item.id === id ? otimista : item));
    try {
      const atualizada = await api.patch<TarefaCalendario>(`/api/calendario/tarefas/${id}/data`, novasDatas);
      setTarefas((atuais) => atuais.map((item) => item.id === id ? atualizada : item));
      setTarefaEditando((atual) => atual?.id === id ? atualizada : atual);
      if (tarefaEditando?.id === id) void carregarHistorico(id);
      notify(`Tarefa movida para ${formatarDataCompleta(dataLocalDaChave(destino))}`);
      return true;
    } catch (error) {
      setTarefas((atuais) => atuais.map((item) => item.id === id ? tarefa : item));
      notify(errorMessage(error), "erro");
      return false;
    } finally {
      setTarefaMovendoId(null);
    }
  };

  const alterarStatusRapido = async (tarefa: TarefaCalendario) => {
    if (tarefaStatusId !== null) return;
    const novoStatus: StatusTarefa = tarefa.status === "CONCLUIDA" ? "PENDENTE" : "CONCLUIDA";
    const otimista = { ...tarefa, status: novoStatus };
    setTarefaStatusId(tarefa.id);
    setTarefas((atuais) => atuais.map((item) => item.id === tarefa.id ? otimista : item));
    try {
      const atualizada = await api.patch<TarefaCalendario>(`/api/calendario/tarefas/${tarefa.id}/status`, { status: novoStatus });
      setTarefas((atuais) => atuais.map((item) => item.id === tarefa.id ? atualizada : item));
      setTarefaEditando((atual) => atual?.id === tarefa.id ? atualizada : atual);
      notify(novoStatus === "CONCLUIDA" ? "Tarefa concluída" : "Tarefa reaberta");
      if (tarefaEditando?.id === tarefa.id) void carregarHistorico(tarefa.id);
    } catch (error) {
      setTarefas((atuais) => atuais.map((item) => item.id === tarefa.id ? tarefa : item));
      notify(errorMessage(error), "erro");
    } finally {
      setTarefaStatusId(null);
    }
  };

  const abrirMoverMobile = (tarefa: TarefaCalendario) => {
    setTarefaMoverMobile(tarefa);
    setDataMoverMobile(chaveDaTarefa(tarefa));
  };

  const confirmarMoverMobile = async (event: FormEvent) => {
    event.preventDefault();
    if (!tarefaMoverMobile || !dataMoverMobile) return;
    if (await moverTarefa(tarefaMoverMobile.id, dataMoverMobile)) {
      setTarefaMoverMobile(null);
    }
  };

  const pessoasFiltradas = useMemo(() => {
    const termo = buscaPessoa.trim().toLocaleLowerCase("pt-BR");
    return pessoas.filter((pessoa) => !termo || pessoa.nome.toLocaleLowerCase("pt-BR").includes(termo));
  }, [buscaPessoa, pessoas]);

  const tarefasVisiveis = tarefasFiltradas.filter((tarefa) => {
    const data = dataDaTarefa(tarefa);
    return data.getMonth() === mesAtual.getMonth() && data.getFullYear() === mesAtual.getFullYear();
  });
  const pendentes = tarefasVisiveis.filter((tarefa) => tarefa.status !== "CONCLUIDA").length;
  const concluidas = tarefasVisiveis.filter((tarefa) => tarefa.status === "CONCLUIDA").length;
  const vinculadas = new Set(tarefasVisiveis.flatMap((tarefa) => tarefa.pessoas.map((pessoa) => pessoa.id))).size;

  if (carregando) return <Spinner label="Organizando o calendário" />;

  return (
    <div>
      <PageHeader
        eyebrow="Planejamento"
        title="Calendário"
        description="Agende tarefas, organize prioridades e mantenha as pessoas relacionadas no contexto de cada compromisso."
        action={<Button type="button" onClick={() => abrirNova()}><Plus className="size-4" /> Nova tarefa</Button>}
      />

      <section className="mb-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <Resumo icon={<Clock3 />} valor={pendentes} rotulo="tarefas pendentes no mês" />
        <Resumo icon={<CheckCircle2 />} valor={concluidas} rotulo="tarefas concluídas no mês" />
        <Resumo icon={<UsersRound />} valor={vinculadas} rotulo="pessoas vinculadas no mês" />
        <Resumo icon={<Paperclip />} valor={armazenamento ? formatBytes(armazenamento.usado_bytes) : "—"} rotulo="armazenamento das tarefas" />
      </section>

      <section className="panel mb-6 p-4 sm:p-5">
        <div className="mb-3 flex items-center justify-between gap-3">
          <div className="flex items-center gap-2"><ListFilter className="size-4 text-teal-700" /><h2 className="text-sm font-semibold text-slate-800">Buscar e filtrar</h2>{filtrosAtivos > 0 && <span className="chip">{filtrosAtivos} ativo(s)</span>}</div>
          {filtrosAtivos > 0 && <button type="button" className="inline-flex items-center gap-1 text-xs font-semibold text-slate-500 hover:text-teal-700" onClick={limparFiltros}><X className="size-3.5" /> Limpar</button>}
        </div>
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <label className="relative sm:col-span-2">
            <span className="sr-only">Buscar tarefa</span>
            <Search className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
            <input className="field pl-10" placeholder="Título, descrição ou pessoa..." value={buscaTarefa} onChange={(event) => setBuscaTarefa(event.target.value)} />
          </label>
          <select className="field" aria-label="Filtrar por pessoa" value={filtroPessoa} onChange={(event) => setFiltroPessoa(event.target.value)}>
            <option value="">Todas as pessoas</option>
            {pessoas.map((pessoa) => <option key={pessoa.id} value={pessoa.id}>{pessoa.nome}</option>)}
          </select>
          <select className="field" aria-label="Filtrar por status" value={filtroStatus} onChange={(event) => setFiltroStatus(event.target.value)}>
            <option value="">Todos os status</option><option value="PENDENTE">Pendentes</option><option value="EM_ANDAMENTO">Em andamento</option><option value="CONCLUIDA">Concluídas</option>
          </select>
          <select className="field" aria-label="Filtrar por prioridade" value={filtroPrioridade} onChange={(event) => setFiltroPrioridade(event.target.value)}>
            <option value="">Todas as prioridades</option><option value="BAIXA">Baixa</option><option value="NORMAL">Normal</option><option value="ALTA">Alta</option>
          </select>
          <select className="field" aria-label="Filtrar por anexos" value={filtroAnexos} onChange={(event) => setFiltroAnexos(event.target.value)}>
            <option value="">Com ou sem anexos</option><option value="COM">Com anexos</option><option value="SEM">Sem anexos</option>
          </select>
          <label><span className="field-label">De</span><input className="field" type="date" value={filtroInicio} onChange={(event) => setFiltroInicio(event.target.value)} /></label>
          <label><span className="field-label">Até</span><input className="field" type="date" min={filtroInicio || undefined} value={filtroFim} onChange={(event) => setFiltroFim(event.target.value)} /></label>
        </div>
      </section>

      <section className="panel overflow-hidden">
        <header className="flex flex-col gap-4 border-b border-slate-100 p-4 sm:flex-row sm:items-center sm:justify-between sm:p-5">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-teal-700">Visão mensal</p>
            <h2 className="mt-1 font-display text-2xl font-semibold capitalize text-slate-950">
              {mesAtual.toLocaleDateString("pt-BR", { month: "long", year: "numeric" })}
            </h2>
            <p className="mt-1 flex items-center gap-1 text-xs text-slate-400"><GripVertical className="size-3.5" /> Arraste uma tarefa para alterar o dia</p>
          </div>
          <div className="flex items-center gap-2">
            <button className="icon-button" type="button" aria-label="Mês anterior" onClick={() => setMesAtual(adicionarMeses(mesAtual, -1))}><ChevronLeft className="size-4" /></button>
            <Button type="button" variant="secondary" onClick={() => setMesAtual(inicioDoMes(new Date()))}>Hoje</Button>
            <button className="icon-button" type="button" aria-label="Próximo mês" onClick={() => setMesAtual(adicionarMeses(mesAtual, 1))}><ChevronRight className="size-4" /></button>
          </div>
        </header>

        <div className="hidden grid-cols-7 border-b border-slate-100 bg-slate-50/70 md:grid">
          {DIAS_SEMANA.map((dia) => <div key={dia} className="px-3 py-2.5 text-center text-xs font-bold uppercase tracking-wider text-slate-400">{dia}</div>)}
        </div>
        <div className="hidden grid-cols-7 md:grid">
          {diasDoCalendario.map((dia) => {
            const chave = chaveData(dia);
            const itens = tarefasPorDia.get(chave) || [];
            const mesmoMes = dia.getMonth() === mesAtual.getMonth();
            const eHoje = chave === chaveData(hoje);
            return (
              <div
                key={chave}
                className={cn(
                  "group min-h-32 border-b border-r border-slate-100 p-2 transition last:border-r-0",
                  !mesmoMes && "bg-slate-50/45",
                  diaDestino === chave && "bg-teal-50 ring-2 ring-inset ring-teal-400",
                )}
                onDragEnter={(event) => {
                  if (!Array.from(event.dataTransfer.types).includes(TIPO_ARRASTE_TAREFA)) return;
                  event.preventDefault();
                  setDiaDestino(chave);
                }}
                onDragOver={(event) => {
                  if (!Array.from(event.dataTransfer.types).includes(TIPO_ARRASTE_TAREFA)) return;
                  event.preventDefault();
                  event.dataTransfer.dropEffect = "move";
                  if (diaDestino !== chave) setDiaDestino(chave);
                }}
                onDrop={(event) => soltarTarefa(event, chave)}
              >
                <div className="mb-1 flex items-center justify-between">
                  <button
                    type="button"
                    className={cn("grid size-8 place-items-center rounded-full text-sm font-semibold", eHoje ? "bg-coral text-white" : mesmoMes ? "text-slate-700 hover:bg-teal-50 hover:text-teal-800" : "text-slate-300")}
                    onClick={() => abrirNova(chave)}
                    aria-label={`Agendar em ${formatarDataCompleta(dia)}`}
                  >
                    {dia.getDate()}
                  </button>
                  <button type="button" className="rounded-lg p-1 text-slate-300 opacity-0 transition hover:bg-teal-50 hover:text-teal-700 group-hover:opacity-100 focus:opacity-100" onClick={() => abrirNova(chave)} aria-label={`Nova tarefa em ${formatarDataCompleta(dia)}`}><Plus className="size-3.5" /></button>
                </div>
                <div className="max-h-24 space-y-1 overflow-y-auto pr-1">
                  {itens.map((tarefa) => (
                    <TarefaChip
                      key={tarefa.id}
                      tarefa={tarefa}
                      dragging={tarefaArrastadaId === tarefa.id}
                      moving={tarefaMovendoId === tarefa.id}
                      changingStatus={tarefaStatusId === tarefa.id}
                      onClick={() => abrirEdicao(tarefa)}
                      onToggleStatus={() => void alterarStatusRapido(tarefa)}
                      onDragStart={(event) => iniciarArraste(event, tarefa)}
                      onDragEnd={encerrarArraste}
                    />
                  ))}
                </div>
              </div>
            );
          })}
        </div>

        <div className="divide-y divide-slate-100 md:hidden">
          {diasDoCalendario
            .filter((dia) => dia.getMonth() === mesAtual.getMonth() && (tarefasPorDia.get(chaveData(dia))?.length || 0) > 0)
            .map((dia) => {
              const itens = tarefasPorDia.get(chaveData(dia)) || [];
              return (
                <section key={chaveData(dia)} className="p-4">
                  <div className="mb-3 flex items-center justify-between">
                    <div><p className="text-xs font-semibold uppercase text-teal-700">{dia.toLocaleDateString("pt-BR", { weekday: "long" })}</p><h3 className="font-display text-lg font-semibold">{dia.toLocaleDateString("pt-BR", { day: "2-digit", month: "long" })}</h3></div>
                    <button className="icon-button" type="button" onClick={() => abrirNova(chaveData(dia))} aria-label={`Nova tarefa em ${formatarDataCompleta(dia)}`}><Plus className="size-4" /></button>
                  </div>
                  <div className="space-y-2">{itens.map((tarefa) => <TarefaMobile key={tarefa.id} tarefa={tarefa} changingStatus={tarefaStatusId === tarefa.id} onClick={() => abrirEdicao(tarefa)} onMove={() => abrirMoverMobile(tarefa)} onToggleStatus={() => void alterarStatusRapido(tarefa)} />)}</div>
                </section>
              );
            })}
          {tarefasVisiveis.length === 0 && (
            <div className="p-4">
              <EmptyState icon={<CalendarCheck className="size-6" />} title={filtrosAtivos > 0 ? "Nenhuma tarefa encontrada" : "Mês livre"} description={filtrosAtivos > 0 ? "Ajuste ou limpe os filtros para voltar a exibir as tarefas." : "Não há tarefas neste mês. Escolha uma data para começar a planejar."} action={filtrosAtivos > 0 ? <Button type="button" variant="secondary" onClick={limparFiltros}><X className="size-4" /> Limpar filtros</Button> : <Button type="button" onClick={() => abrirNova(chaveData(mesAtual))}><Plus className="size-4" /> Agendar tarefa</Button>} />
            </div>
          )}
        </div>
      </section>

      <Modal open={modalAberto} onClose={fecharModal} title={tarefaEditando ? "Editar tarefa" : "Nova tarefa"} className="max-w-3xl">
        <form className="space-y-5" onSubmit={salvar}>
          <div>
            <label className="field-label" htmlFor="tarefa-titulo">Título</label>
            <input id="tarefa-titulo" className="field" autoFocus maxLength={160} required placeholder="Ex.: Retornar ligação" value={form.titulo} onChange={(event) => setForm({ ...form, titulo: event.target.value })} />
          </div>

          <div>
            <label className="field-label" htmlFor="tarefa-descricao">Descrição <span className="font-normal text-slate-400">(opcional)</span></label>
            <textarea id="tarefa-descricao" className="field min-h-24 resize-y" maxLength={5000} placeholder="Anotações, pauta ou contexto importante..." value={form.descricao} onChange={(event) => setForm({ ...form, descricao: event.target.value })} />
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="tarefa-data">Data</label>
              <input id="tarefa-data" className="field" type="date" required value={form.data} onChange={(event) => setForm({ ...form, data: event.target.value })} />
            </div>
            <label className="mt-6 flex min-h-11 items-center gap-3 rounded-xl border border-slate-200 bg-slate-50 px-3.5 text-sm font-medium text-slate-700">
              <input className="size-4 accent-teal-700" type="checkbox" checked={form.diaInteiro} onChange={(event) => setForm({ ...form, diaInteiro: event.target.checked })} />
              Tarefa de dia inteiro
            </label>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="tarefa-recorrencia">Recorrência</label>
              <select id="tarefa-recorrencia" className="field" disabled={Boolean(tarefaEditando)} value={form.recorrencia} onChange={(event) => setForm({ ...form, recorrencia: event.target.value as RecorrenciaTarefa, recorrenciaFim: event.target.value === "NENHUMA" ? "" : form.recorrenciaFim || chaveData(adicionarDias(dataLocalDaChave(form.data), 30)) })}>
                <option value="NENHUMA">Não repetir</option><option value="DIARIA">Diariamente</option><option value="SEMANAL">Semanalmente</option><option value="MENSAL">Mensalmente</option>
              </select>
            </div>
            <div>
              <label className="field-label" htmlFor="tarefa-lembrete">Lembrete</label>
              <select id="tarefa-lembrete" className="field" value={form.lembreteMinutos} onChange={(event) => setForm({ ...form, lembreteMinutos: event.target.value })}>
                <option value="">Sem lembrete</option><option value="0">No horário</option><option value="5">5 minutos antes</option><option value="15">15 minutos antes</option><option value="30">30 minutos antes</option><option value="60">1 hora antes</option><option value="1440">1 dia antes</option><option value="10080">1 semana antes</option>
              </select>
            </div>
          </div>

          {form.recorrencia !== "NENHUMA" && (
            <div className="rounded-2xl border border-violet-100 bg-violet-50 p-4">
              <label className="field-label" htmlFor="tarefa-recorrencia-fim">Repetir até</label>
              <input id="tarefa-recorrencia-fim" className="field bg-white" type="date" disabled={Boolean(tarefaEditando)} required min={chaveData(adicionarDias(dataLocalDaChave(form.data), 1))} max={chaveData(adicionarDias(dataLocalDaChave(form.data), 365))} value={form.recorrenciaFim} onChange={(event) => setForm({ ...form, recorrenciaFim: event.target.value })} />
              <p className="mt-2 flex items-start gap-1.5 text-xs leading-5 text-violet-700"><Repeat2 className="mt-0.5 size-3.5 shrink-0" /> {tarefaEditando ? `Esta ocorrência faz parte de uma série com ${tarefaEditando.total_ocorrencias} tarefa(s). A edição afeta somente esta ocorrência.` : "As ocorrências serão criadas como tarefas independentes por até um ano. Anexos serão vinculados somente à primeira ocorrência."}</p>
            </div>
          )}

          {!form.diaInteiro && (
            <div className="grid gap-4 sm:grid-cols-2">
              <div><label className="field-label" htmlFor="tarefa-inicio">Início</label><input id="tarefa-inicio" className="field" type="time" required value={form.inicioHora} onChange={(event) => setForm({ ...form, inicioHora: event.target.value })} /></div>
              <div><label className="field-label" htmlFor="tarefa-fim">Término <span className="font-normal text-slate-400">(opcional)</span></label><input id="tarefa-fim" className="field" type="time" value={form.fimHora} onChange={(event) => setForm({ ...form, fimHora: event.target.value })} /></div>
            </div>
          )}

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="tarefa-status">Status</label>
              <select id="tarefa-status" className="field" value={form.status} onChange={(event) => setForm({ ...form, status: event.target.value as StatusTarefa })}>
                <option value="PENDENTE">Pendente</option><option value="EM_ANDAMENTO">Em andamento</option><option value="CONCLUIDA">Concluída</option>
              </select>
            </div>
            <div>
              <label className="field-label" htmlFor="tarefa-prioridade">Prioridade</label>
              <select id="tarefa-prioridade" className="field" value={form.prioridade} onChange={(event) => setForm({ ...form, prioridade: event.target.value as PrioridadeTarefa })}>
                <option value="BAIXA">Baixa</option><option value="NORMAL">Normal</option><option value="ALTA">Alta</option>
              </select>
            </div>
          </div>

          <fieldset>
            <legend className="field-label">Cor da tarefa</legend>
            <div className="flex flex-wrap gap-2">
              {CORES.map((cor) => <button key={cor} type="button" className={cn("size-9 rounded-full border-4 transition", form.corHex === cor ? "border-slate-800 scale-110" : "border-white ring-1 ring-slate-200")} style={{ backgroundColor: cor }} onClick={() => setForm({ ...form, corHex: cor })} aria-label={`Usar a cor ${cor}`} aria-pressed={form.corHex === cor} />)}
              <input className="size-9 cursor-pointer rounded-full border-0 bg-transparent p-0" type="color" value={form.corHex} onChange={(event) => setForm({ ...form, corHex: event.target.value.toUpperCase() })} aria-label="Escolher outra cor" />
            </div>
          </fieldset>

          <fieldset>
            <legend className="field-label flex items-center gap-2"><Paperclip className="size-4" /> Arquivos e fotos <span className="font-normal text-slate-400">(opcional)</span></legend>
            <TaskAttachmentEditor
              existing={tarefaEditando?.anexos || []}
              pending={arquivosPendentes}
              disabled={salvando || anexoExcluindoId !== null}
              onPendingChange={setArquivosPendentes}
              onDeleteExisting={(anexo) => void excluirAnexo(anexo)}
              onInvalidFiles={(mensagem) => notify(mensagem, "erro")}
              storage={armazenamento}
              uploadStates={uploadStates}
              onRetry={(arquivo) => void repetirUpload(arquivo)}
            />
          </fieldset>

          <fieldset>
            <legend className="field-label">Pessoas vinculadas <span className="font-normal text-slate-400">(opcional)</span></legend>
            {pessoas.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-slate-200 bg-slate-50 p-4 text-sm text-slate-500">
                Nenhuma pessoa cadastrada. <Link className="font-semibold text-teal-700 hover:underline" to="/pessoas/nova">Cadastrar pessoa</Link>
              </div>
            ) : (
              <div className="overflow-hidden rounded-2xl border border-slate-200">
                <label className="relative block border-b border-slate-100">
                  <span className="sr-only">Buscar pessoa</span>
                  <Search className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
                  <input className="w-full px-4 py-3 pl-10 text-sm outline-none" placeholder="Buscar pessoa para vincular..." value={buscaPessoa} onChange={(event) => setBuscaPessoa(event.target.value)} />
                </label>
                <div className="max-h-48 divide-y divide-slate-100 overflow-y-auto">
                  {pessoasFiltradas.map((pessoa) => {
                    const selecionada = form.pessoasIds.includes(pessoa.id);
                    return (
                      <label key={pessoa.id} className="flex items-center gap-3 px-4 py-2.5 transition hover:bg-slate-50">
                        <input className="size-4 accent-teal-700" type="checkbox" checked={selecionada} onChange={() => setForm({ ...form, pessoasIds: selecionada ? form.pessoasIds.filter((id) => id !== pessoa.id) : [...form.pessoasIds, pessoa.id] })} />
                        <span className="grid size-8 place-items-center rounded-full text-white" style={{ backgroundColor: pessoa.cor_hex || "#86A6A3" }}><UserRound className="size-4" /></span>
                        <span className="min-w-0 flex-1 truncate text-sm font-medium text-slate-700">{pessoa.nome}</span>
                        {selecionada && <span className="text-xs font-semibold text-teal-700">Vinculada</span>}
                      </label>
                    );
                  })}
                  {pessoasFiltradas.length === 0 && <p className="p-4 text-center text-sm text-slate-400">Nenhuma pessoa encontrada.</p>}
                </div>
              </div>
            )}
            {form.pessoasIds.length > 0 && <p className="mt-2 text-xs font-medium text-teal-700">{form.pessoasIds.length} pessoa(s) vinculada(s)</p>}
          </fieldset>

          {tarefaEditando && (
            <details className="rounded-2xl border border-slate-200 bg-slate-50">
              <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-sm font-semibold text-slate-700"><History className="size-4 text-teal-700" /> Histórico da tarefa <span className="chip ml-auto">{historico.length}</span></summary>
              <div className="border-t border-slate-200 px-4 py-3">
                {carregandoHistorico ? <p className="text-xs text-slate-400">Carregando histórico...</p> : historico.length === 0 ? <p className="text-xs text-slate-400">Nenhuma alteração registrada.</p> : <ol className="space-y-3">{historico.map((item) => <li key={item.id} className="flex gap-3 text-xs"><span className="mt-1.5 size-2 shrink-0 rounded-full bg-teal-600" /><div><p className="font-medium text-slate-700">{item.descricao}</p><p className="mt-0.5 text-slate-400">{new Date(item.data_evento).toLocaleString("pt-BR")}</p></div></li>)}</ol>}
              </div>
            </details>
          )}

          <div className="flex flex-col-reverse gap-3 border-t border-slate-100 pt-5 sm:flex-row sm:items-center sm:justify-between">
            <div>{tarefaEditando && <Button type="button" variant="danger" disabled={salvando} onClick={() => void excluir()}><Trash2 className="size-4" /> Excluir tarefa</Button>}</div>
            <div className="flex justify-end gap-2">
              <Button type="button" variant="secondary" disabled={salvando} onClick={fecharModal}>Cancelar</Button>
              <Button type="submit" loading={salvando}>{tarefaEditando ? <Edit3 className="size-4" /> : <CalendarDays className="size-4" />}{tarefaEditando ? "Salvar alterações" : "Agendar tarefa"}</Button>
            </div>
          </div>
        </form>
      </Modal>

      <Modal open={Boolean(tarefaMoverMobile)} onClose={() => setTarefaMoverMobile(null)} title="Mover tarefa" className="max-w-md">
        <form className="space-y-4" onSubmit={confirmarMoverMobile}>
          <p className="text-sm text-slate-500">Escolha a nova data de <strong className="text-slate-800">{tarefaMoverMobile?.titulo}</strong>. O horário e a duração serão preservados.</p>
          <div><label className="field-label" htmlFor="mover-tarefa-data">Nova data</label><input id="mover-tarefa-data" className="field" type="date" required value={dataMoverMobile} onChange={(event) => setDataMoverMobile(event.target.value)} /></div>
          <div className="flex justify-end gap-2"><Button type="button" variant="secondary" onClick={() => setTarefaMoverMobile(null)}>Cancelar</Button><Button type="submit" loading={tarefaMovendoId === tarefaMoverMobile?.id}><CalendarDays className="size-4" /> Mover tarefa</Button></div>
        </form>
      </Modal>
    </div>
  );
}

function Resumo({ icon, valor, rotulo }: { icon: React.ReactNode; valor: number | string; rotulo: string }) {
  return <div className="panel flex items-center gap-4 px-5 py-4"><div className="grid size-11 place-items-center rounded-2xl bg-teal-50 text-teal-700 [&>svg]:size-5">{icon}</div><div><p className="font-display text-2xl font-semibold text-slate-950">{valor}</p><p className="text-xs text-slate-500">{rotulo}</p></div></div>;
}

function TarefaChip({
  tarefa,
  dragging,
  moving,
  changingStatus,
  onClick,
  onToggleStatus,
  onDragStart,
  onDragEnd,
}: {
  tarefa: TarefaCalendario;
  dragging: boolean;
  moving: boolean;
  changingStatus: boolean;
  onClick: () => void;
  onToggleStatus: () => void;
  onDragStart: (event: DragEvent<HTMLElement>) => void;
  onDragEnd: () => void;
}) {
  const style = { borderLeftColor: tarefa.cor_hex, backgroundColor: `${tarefa.cor_hex}12` } as CSSProperties;
  return (
    <article
      draggable={!moving}
      className={cn(
        "flex w-full cursor-grab items-center rounded-lg border-l-[3px] text-left text-[11px] font-semibold transition hover:brightness-95 active:cursor-grabbing",
        tarefa.status === "CONCLUIDA" && "opacity-60",
        dragging && "opacity-35",
        moving && "cursor-wait animate-pulse",
      )}
      style={style}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      title={`${tarefa.titulo} — arraste para mudar o dia`}
    >
      <button type="button" className={cn("min-w-0 flex-1 truncate px-2 py-1 text-left", tarefa.status === "CONCLUIDA" && "line-through")} onClick={onClick} aria-label={`${tarefa.titulo}. Clique para editar.`}>
        {!tarefa.dia_inteiro && <span className="mr-1 font-normal text-slate-500">{formatarHora(tarefa.inicio_em)}</span>}
        <span style={{ color: tarefa.cor_hex }}>{tarefa.titulo}</span>
        {tarefa.pessoas.length > 0 && <span className="ml-1 text-slate-400">· {tarefa.pessoas.length}</span>}
        {tarefa.anexos.length > 0 && <span className="ml-1 text-slate-400">· 📎 {tarefa.anexos.length}</span>}
        {tarefa.recorrencia !== "NENHUMA" && <span className="ml-1 text-slate-400">· ↻</span>}
      </button>
      <button type="button" disabled={changingStatus} className={cn("mr-1 grid size-5 shrink-0 place-items-center rounded text-slate-400 transition hover:bg-white/80 hover:text-emerald-700", tarefa.status === "CONCLUIDA" && "text-emerald-700")} onClick={onToggleStatus} title={tarefa.status === "CONCLUIDA" ? "Reabrir tarefa" : "Concluir tarefa"}><Check className="size-3" /></button>
    </article>
  );
}

function TarefaMobile({ tarefa, changingStatus, onClick, onMove, onToggleStatus }: { tarefa: TarefaCalendario; changingStatus: boolean; onClick: () => void; onMove: () => void; onToggleStatus: () => void }) {
  return (
    <article className="flex w-full items-start gap-3 rounded-2xl border border-slate-100 p-3 text-left transition hover:border-teal-200 hover:bg-teal-50/40">
      <span className="mt-1 size-3 shrink-0 rounded-full" style={{ backgroundColor: tarefa.cor_hex }} />
      <button type="button" className="min-w-0 flex-1" onClick={onClick}><span className={cn("block text-sm font-semibold text-slate-800", tarefa.status === "CONCLUIDA" && "line-through opacity-60")}>{tarefa.titulo}</span><span className="mt-1 block text-xs text-slate-500">{formatarPeriodo(tarefa)}{tarefa.pessoas.length > 0 ? ` · ${tarefa.pessoas.map((pessoa) => pessoa.nome).join(", ")}` : ""}</span></button>
      {tarefa.prioridade === "ALTA" && <CircleAlert className="size-4 shrink-0 text-rose-600" aria-label="Prioridade alta" />}
      <div className="flex shrink-0 gap-1"><button type="button" className="icon-button size-8" onClick={onMove} title="Mover para outra data"><CalendarDays className="size-3.5" /></button><button type="button" disabled={changingStatus} className={cn("icon-button size-8", tarefa.status === "CONCLUIDA" && "text-emerald-700")} onClick={onToggleStatus} title={tarefa.status === "CONCLUIDA" ? "Reabrir" : "Concluir"}><Check className="size-3.5" /></button></div>
    </article>
  );
}

function novoFormulario(data: string): TarefaFormState {
  return { titulo: "", descricao: "", data, inicioHora: "09:00", fimHora: "10:00", diaInteiro: false, status: "PENDENTE", prioridade: "NORMAL", corHex: "#13716D", pessoasIds: [], recorrencia: "NENHUMA", recorrenciaFim: "", lembreteMinutos: "" };
}

function formularioDaTarefa(tarefa: TarefaCalendario): TarefaFormState {
  const inicio = new Date(tarefa.inicio_em);
  const fim = tarefa.fim_em ? new Date(tarefa.fim_em) : null;
  return {
    titulo: tarefa.titulo,
    descricao: tarefa.descricao || "",
    data: tarefa.dia_inteiro ? tarefa.inicio_em.slice(0, 10) : chaveData(inicio),
    inicioHora: tarefa.dia_inteiro ? "09:00" : horaInput(inicio),
    fimHora: !tarefa.dia_inteiro && fim ? horaInput(fim) : "",
    diaInteiro: tarefa.dia_inteiro,
    status: tarefa.status,
    prioridade: tarefa.prioridade,
    corHex: tarefa.cor_hex,
    pessoasIds: tarefa.pessoas.map((pessoa) => pessoa.id),
    recorrencia: tarefa.recorrencia,
    recorrenciaFim: tarefa.recorrencia_fim_em?.slice(0, 10) || "",
    lembreteMinutos: tarefa.lembrete_minutos === null ? "" : String(tarefa.lembrete_minutos),
  };
}

function inicioDoMes(data: Date) { return new Date(data.getFullYear(), data.getMonth(), 1); }
function adicionarMeses(data: Date, quantidade: number) { return new Date(data.getFullYear(), data.getMonth() + quantidade, 1); }
function adicionarDias(data: Date, quantidade: number) { return new Date(data.getFullYear(), data.getMonth(), data.getDate() + quantidade); }
function montarDiasDoCalendario(mes: Date) {
  const primeiro = inicioDoMes(mes);
  const deslocamento = (primeiro.getDay() + 6) % 7;
  const inicio = adicionarDias(primeiro, -deslocamento);
  return Array.from({ length: 42 }, (_, indice) => adicionarDias(inicio, indice));
}
function chaveData(data: Date) { return `${data.getFullYear()}-${String(data.getMonth() + 1).padStart(2, "0")}-${String(data.getDate()).padStart(2, "0")}`; }
function dataDaTarefa(tarefa: TarefaCalendario) { return tarefa.dia_inteiro ? dataLocalDaChave(tarefa.inicio_em.slice(0, 10)) : new Date(tarefa.inicio_em); }
function chaveDaTarefa(tarefa: TarefaCalendario) { return chaveData(dataDaTarefa(tarefa)); }
function dataLocalDaChave(chave: string) { const [ano, mes, dia] = chave.split("-").map(Number); return new Date(ano, mes - 1, dia); }
function formatarDataCompleta(data: Date) { return data.toLocaleDateString("pt-BR", { day: "2-digit", month: "long", year: "numeric" }); }
function formatarHora(valor: string) { return new Date(valor).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" }); }
function horaInput(data: Date) { return `${String(data.getHours()).padStart(2, "0")}:${String(data.getMinutes()).padStart(2, "0")}`; }
function formatarPeriodo(tarefa: TarefaCalendario) { return tarefa.dia_inteiro ? "Dia inteiro" : `${formatarHora(tarefa.inicio_em)}${tarefa.fim_em ? ` – ${formatarHora(tarefa.fim_em)}` : ""}`; }
function dataHoraLocalParaIso(data: string, hora: string) {
  if (!data || !hora) return null;
  const [ano, mes, dia] = data.split("-").map(Number);
  const [horas, minutos] = hora.split(":").map(Number);
  const valor = new Date(ano, mes - 1, dia, horas, minutos);
  return Number.isNaN(valor.getTime()) ? null : valor.toISOString();
}
function ordenarTarefas(a: TarefaCalendario, b: TarefaCalendario) {
  if (a.dia_inteiro !== b.dia_inteiro) return a.dia_inteiro ? -1 : 1;
  return a.inicio_em.localeCompare(b.inicio_em);
}

function datasAoMoverTarefa(tarefa: TarefaCalendario, destino: string) {
  const inicioAnterior = new Date(tarefa.inicio_em);
  const fimAnterior = tarefa.fim_em ? new Date(tarefa.fim_em) : null;
  const duracao = fimAnterior ? fimAnterior.getTime() - inicioAnterior.getTime() : null;

  if (tarefa.dia_inteiro) {
    const inicio = new Date(`${destino}T00:00:00.000Z`);
    return {
      inicio_em: inicio.toISOString(),
      fim_em: duracao === null ? null : new Date(inicio.getTime() + duracao).toISOString(),
    };
  }

  const dataDestino = dataLocalDaChave(destino);
  const inicio = new Date(
    dataDestino.getFullYear(),
    dataDestino.getMonth(),
    dataDestino.getDate(),
    inicioAnterior.getHours(),
    inicioAnterior.getMinutes(),
    inicioAnterior.getSeconds(),
    inicioAnterior.getMilliseconds(),
  );
  return {
    inicio_em: inicio.toISOString(),
    fim_em: duracao === null ? null : new Date(inicio.getTime() + duracao).toISOString(),
  };
}
