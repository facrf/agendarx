import {
  CalendarCheck,
  CalendarDays,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  Clock3,
  Edit3,
  Plus,
  Search,
  Trash2,
  UserRound,
  UsersRound,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { CSSProperties, FormEvent } from "react";
import { Link } from "react-router-dom";
import { Button, EmptyState, Modal, PageHeader, Spinner, cn } from "../components/ui";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type {
  PessoaResumo,
  PrioridadeTarefa,
  StatusTarefa,
  TarefaCalendario,
  TarefaCalendarioPayload,
} from "../types/api";

const DIAS_SEMANA = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
const CORES = ["#13716D", "#2563EB", "#7C3AED", "#DB2777", "#E7654F", "#D97706"];

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
}

export function CalendarPage() {
  const hoje = useMemo(() => new Date(), []);
  const [mesAtual, setMesAtual] = useState(() => inicioDoMes(hoje));
  const [tarefas, setTarefas] = useState<TarefaCalendario[]>([]);
  const [pessoas, setPessoas] = useState<PessoaResumo[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [modalAberto, setModalAberto] = useState(false);
  const [tarefaEditando, setTarefaEditando] = useState<TarefaCalendario | null>(null);
  const [form, setForm] = useState<TarefaFormState>(() => novoFormulario(chaveData(hoje)));
  const [buscaPessoa, setBuscaPessoa] = useState("");
  const [salvando, setSalvando] = useState(false);
  const { notify } = useToast();

  const diasDoCalendario = useMemo(() => montarDiasDoCalendario(mesAtual), [mesAtual]);

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

  const tarefasPorDia = useMemo(() => {
    const mapa = new Map<string, TarefaCalendario[]>();
    tarefas.forEach((tarefa) => {
      const chave = chaveDaTarefa(tarefa);
      const itens = mapa.get(chave) || [];
      itens.push(tarefa);
      mapa.set(chave, itens);
    });
    mapa.forEach((itens) => itens.sort(ordenarTarefas));
    return mapa;
  }, [tarefas]);

  const abrirNova = (data = chaveData(new Date())) => {
    setTarefaEditando(null);
    setForm(novoFormulario(data));
    setBuscaPessoa("");
    setModalAberto(true);
  };

  const abrirEdicao = (tarefa: TarefaCalendario) => {
    setTarefaEditando(tarefa);
    setForm(formularioDaTarefa(tarefa));
    setBuscaPessoa("");
    setModalAberto(true);
  };

  const fecharModal = () => {
    if (salvando) return;
    setModalAberto(false);
    setTarefaEditando(null);
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
    };

    setSalvando(true);
    try {
      if (tarefaEditando) {
        await api.put(`/api/calendario/tarefas/${tarefaEditando.id}`, payload);
        notify("Tarefa atualizada");
      } else {
        await api.post("/api/calendario/tarefas", payload);
        notify("Tarefa agendada");
      }
      setModalAberto(false);
      setTarefaEditando(null);
      await carregarTarefas();
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
      notify("Tarefa excluída");
      await carregarTarefas();
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const pessoasFiltradas = useMemo(() => {
    const termo = buscaPessoa.trim().toLocaleLowerCase("pt-BR");
    return pessoas.filter((pessoa) => !termo || pessoa.nome.toLocaleLowerCase("pt-BR").includes(termo));
  }, [buscaPessoa, pessoas]);

  const tarefasVisiveis = tarefas.filter((tarefa) => {
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

      <section className="mb-6 grid gap-3 sm:grid-cols-3">
        <Resumo icon={<Clock3 />} valor={pendentes} rotulo="tarefas pendentes no mês" />
        <Resumo icon={<CheckCircle2 />} valor={concluidas} rotulo="tarefas concluídas no mês" />
        <Resumo icon={<UsersRound />} valor={vinculadas} rotulo="pessoas vinculadas no mês" />
      </section>

      <section className="panel overflow-hidden">
        <header className="flex flex-col gap-4 border-b border-slate-100 p-4 sm:flex-row sm:items-center sm:justify-between sm:p-5">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-teal-700">Visão mensal</p>
            <h2 className="mt-1 font-display text-2xl font-semibold capitalize text-slate-950">
              {mesAtual.toLocaleDateString("pt-BR", { month: "long", year: "numeric" })}
            </h2>
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
              <div key={chave} className={cn("group min-h-32 border-b border-r border-slate-100 p-2 last:border-r-0", !mesmoMes && "bg-slate-50/45")}>
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
                  {itens.map((tarefa) => <TarefaChip key={tarefa.id} tarefa={tarefa} onClick={() => abrirEdicao(tarefa)} />)}
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
                  <div className="space-y-2">{itens.map((tarefa) => <TarefaMobile key={tarefa.id} tarefa={tarefa} onClick={() => abrirEdicao(tarefa)} />)}</div>
                </section>
              );
            })}
          {tarefasVisiveis.length === 0 && (
            <div className="p-4">
              <EmptyState icon={<CalendarCheck className="size-6" />} title="Mês livre" description="Não há tarefas neste mês. Escolha uma data para começar a planejar." action={<Button type="button" onClick={() => abrirNova(chaveData(mesAtual))}><Plus className="size-4" /> Agendar tarefa</Button>} />
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

          <div className="flex flex-col-reverse gap-3 border-t border-slate-100 pt-5 sm:flex-row sm:items-center sm:justify-between">
            <div>{tarefaEditando && <Button type="button" variant="danger" disabled={salvando} onClick={() => void excluir()}><Trash2 className="size-4" /> Excluir tarefa</Button>}</div>
            <div className="flex justify-end gap-2">
              <Button type="button" variant="secondary" disabled={salvando} onClick={fecharModal}>Cancelar</Button>
              <Button type="submit" loading={salvando}>{tarefaEditando ? <Edit3 className="size-4" /> : <CalendarDays className="size-4" />}{tarefaEditando ? "Salvar alterações" : "Agendar tarefa"}</Button>
            </div>
          </div>
        </form>
      </Modal>
    </div>
  );
}

function Resumo({ icon, valor, rotulo }: { icon: React.ReactNode; valor: number; rotulo: string }) {
  return <div className="panel flex items-center gap-4 px-5 py-4"><div className="grid size-11 place-items-center rounded-2xl bg-teal-50 text-teal-700 [&>svg]:size-5">{icon}</div><div><p className="font-display text-2xl font-semibold text-slate-950">{valor}</p><p className="text-xs text-slate-500">{rotulo}</p></div></div>;
}

function TarefaChip({ tarefa, onClick }: { tarefa: TarefaCalendario; onClick: () => void }) {
  const style = { borderLeftColor: tarefa.cor_hex, backgroundColor: `${tarefa.cor_hex}12` } as CSSProperties;
  return (
    <button type="button" className={cn("block w-full truncate rounded-lg border-l-[3px] px-2 py-1 text-left text-[11px] font-semibold transition hover:brightness-95", tarefa.status === "CONCLUIDA" && "opacity-55 line-through")} style={style} onClick={onClick} title={tarefa.titulo}>
      {!tarefa.dia_inteiro && <span className="mr-1 font-normal text-slate-500">{formatarHora(tarefa.inicio_em)}</span>}
      <span style={{ color: tarefa.cor_hex }}>{tarefa.titulo}</span>
      {tarefa.pessoas.length > 0 && <span className="ml-1 text-slate-400">· {tarefa.pessoas.length}</span>}
    </button>
  );
}

function TarefaMobile({ tarefa, onClick }: { tarefa: TarefaCalendario; onClick: () => void }) {
  return (
    <button type="button" className="flex w-full items-start gap-3 rounded-2xl border border-slate-100 p-3 text-left transition hover:border-teal-200 hover:bg-teal-50/40" onClick={onClick}>
      <span className="mt-1 size-3 shrink-0 rounded-full" style={{ backgroundColor: tarefa.cor_hex }} />
      <span className="min-w-0 flex-1"><span className={cn("block text-sm font-semibold text-slate-800", tarefa.status === "CONCLUIDA" && "line-through opacity-60")}>{tarefa.titulo}</span><span className="mt-1 block text-xs text-slate-500">{formatarPeriodo(tarefa)}{tarefa.pessoas.length > 0 ? ` · ${tarefa.pessoas.map((pessoa) => pessoa.nome).join(", ")}` : ""}</span></span>
      {tarefa.prioridade === "ALTA" && <CircleAlert className="size-4 shrink-0 text-rose-600" aria-label="Prioridade alta" />}
    </button>
  );
}

function novoFormulario(data: string): TarefaFormState {
  return { titulo: "", descricao: "", data, inicioHora: "09:00", fimHora: "10:00", diaInteiro: false, status: "PENDENTE", prioridade: "NORMAL", corHex: "#13716D", pessoasIds: [] };
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
