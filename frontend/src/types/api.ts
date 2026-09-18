export interface UsuarioSessao {
  id: number;
  login: string;
  perfil: "admin" | "usuario";
  tem_icone: boolean;
  icone_atualizado_em: string | null;
}

export interface LoginResponse {
  token: string;
  token_tipo: "Bearer";
  expira_em: number;
  usuario: UsuarioSessao;
}

export interface CredenciaisPayload {
  login: string;
  senha_atual: string;
  nova_senha?: string;
}

export interface Categoria {
  id: number;
  nome_categoria: string;
  cor_hex: string;
}

export interface TipoMeioContato {
  id: number;
  nome_tipo: string;
}

export interface Contato {
  id: number;
  pessoa_id: number;
  tipo_contato_id: number;
  valor: string;
}

export interface ContatoPayload {
  id?: number;
  tipo_contato_id: number;
  valor: string;
}

export interface PessoaResumo {
  id: number;
  nome: string;
  categoria_id: number | null;
  descricao: string | null;
  nome_categoria: string | null;
  cor_hex: string | null;
  tem_foto: boolean;
  pessoa_juridica: boolean;
  data_cadastro: string;
  etiquetas: string;
  favorito: boolean;
  classificacao_risco: ClassificacaoRisco;
  toxicidade: number;
}

export interface PessoaDetalhe extends PessoaResumo {
  contatos: Contato[];
  psicossocial: IndicadoresPsicossociais;
  risco_registro: RegistroRisco;
}

export interface PessoaPayload {
  nome: string;
  categoria_id: number | null;
  descricao: string | null;
  pessoa_juridica: boolean;
  contatos?: ContatoPayload[];
  classificacao_risco?: ClassificacaoRisco;
  toxicidade?: number;
  risco_justificativa?: string;
  risco_revisado_em?: string;
}

export interface RegistroRisco {
  classificacao_risco: ClassificacaoRisco;
  toxicidade: number;
  justificativa: string;
  revisado_em: string | null;
}

export interface HistoricoRisco {
  id: number;
  autor_login: string;
  registrado_em: string;
  anterior: RegistroRisco | null;
  novo: RegistroRisco;
}

export interface PreviaRisco {
  pessoa: IndicadoresPsicossociais;
  versao_configuracao: number;
  alteracoes: { pessoa_id: number; nome: string; hp_antes: number | null; hp_depois: number; aura_antes: string | null; aura_depois: string }[];
}

export interface AnexoDossie {
  id: number;
  pessoa_id: number;
  nome_arquivo: string;
  mime_type: string;
  tamanho_bytes: number;
  data_upload: string;
  url_stream: string;
  url_download: string;
  url_thumbnail: string | null;
}

export interface PessoaVinculo {
  id: number;
  pessoa_origem_id: number;
  pessoa_destino_id: number;
  tipo_vinculo: string;
  descricao: string | null;
  data_criacao: string;
}

export interface AnexoVinculo {
  id: number;
  vinculo_id: number;
  nome_arquivo: string;
  mime_type: string;
  tamanho_bytes: number;
  data_upload: string;
  url_stream: string;
  url_download: string;
  url_thumbnail: string | null;
}

export interface VinculoPayload {
  pessoa_origem_id: number;
  pessoa_destino_id: number;
  tipo_vinculo: string;
  descricao: string | null;
}

export interface GrafoNode extends IndicadoresPsicossociais {
  id: number;
  label: string;
  color: string;
  foto_url: string | null;
  categoria: string | null;
  pessoa_juridica: boolean;
  descricao: string | null;
  contatos: GrafoContato[];
  classificacao_risco: ClassificacaoRisco;
  toxicidade: number;
}

export interface GrafoContato {
  tipo: string;
  valor: string;
}

export interface GrafoEdge {
  id: number;
  source: number;
  target: number;
  label: string;
  descricao: string | null;
  data_criacao: string;
}

export interface Etiqueta { id: number; nome: string; cor_hex: string }
export interface PessoaLixeira { id: number; nome: string; excluida_em: string }
export interface AuditoriaItem { id: number; usuario_login: string; acao: string; recurso: string; status_http: number; data_evento: string }
export interface PosicaoGrafo { pessoa_id: number; x: number; y: number }
export interface BackupInfo {
  id: number;
  nome_arquivo: string;
  tamanho_bytes: number;
  automatico: boolean;
  data_criacao: string;
  tipo: "manual" | "automatico" | "seguranca";
  sha256: string;
  integridade_ok: boolean;
  versao_app: string | null;
  schema_versao: number | null;
}

export interface BackupConfiguracao {
  ativo: boolean;
  horario: string;
  manter_diarios: number;
  manter_semanais: number;
  manter_mensais: number;
  ultima_execucao_em: string | null;
  ultima_tentativa_em: string | null;
  ultimo_erro: string | null;
  proxima_execucao_em: string | null;
  max_upload_bytes: number;
}

export interface RestauracaoPrevia {
  token: string;
  expira_em: string;
  criado_em: string | null;
  versao_app: string | null;
  schema_versao: number;
  schema_atual: number;
  tamanho_bytes: number;
  sha256: string;
  pessoas: number;
  usuarios: number;
  anexos: number;
  criptografado: boolean;
  avisos: string[];
}

export interface GrafoResponse {
  nodes: GrafoNode[];
  edges: GrafoEdge[];
  hp_configuracao?: ConfigHpPsicossocial;
}

export type ClassificacaoRisco = "NAO_CLASSIFICADO" | "SEM_RISCO" | "MANIPULATIVO" | "PATOLOGICO" | "MISTO";

export interface FaixaIndicador {
  min: number;
  nome: string;
  cor_hex: string;
  pulsante: boolean;
}

export interface ConfigHpPsicossocial {
  versao: number;
  ativo: boolean;
  toxicidade_min: number;
  toxicidade_max: number;
  hp_base: number;
  hp_min: number;
  fator_segundo_grau: number;
  pesos_vinculo: Record<string, number>;
  peso_padrao: number;
  faixas_aura: FaixaIndicador[];
  faixas_vitalidade: FaixaIndicador[];
}

export interface ContribuicaoHp {
  fonte_id: number;
  fonte_nome: string;
  alvo_direto_id: number;
  alvo_direto_nome: string;
  vinculo_id: number;
  tipo_vinculo: string;
  toxicidade_fonte: number;
  peso: number;
  grau: 1 | 2;
  penalidade: number;
}

export interface IndicadoresPsicossociais {
  hp: number;
  hp_percentual: number;
  hp_base: number;
  hp_min: number;
  penalidade_direta: number;
  penalidade_propria: number;
  penalidade_residual: number;
  aura_nome: string;
  aura_cor_hex: string;
  aura_pulsante: boolean;
  vitalidade_nome: string;
  vitalidade_cor_hex: string;
  calculo_ativo: boolean;
  configuracao_versao: number;
  contribuicoes: ContribuicaoHp[];
}

export interface IdentidadeVisual {
  tem_icone: boolean;
  atualizado_em: string | null;
}

export type StatusTarefa = "PENDENTE" | "EM_ANDAMENTO" | "CONCLUIDA";
export type PrioridadeTarefa = "BAIXA" | "NORMAL" | "ALTA";
export type RecorrenciaTarefa = "NENHUMA" | "DIARIA" | "SEMANAL" | "MENSAL";

export interface PessoaTarefaResumo {
  id: number;
  nome: string;
  cor_hex: string | null;
  tem_foto: boolean;
  pessoa_juridica: boolean;
}

export interface TarefaCalendario {
  id: number;
  titulo: string;
  descricao: string | null;
  inicio_em: string;
  fim_em: string | null;
  dia_inteiro: boolean;
  status: StatusTarefa;
  prioridade: PrioridadeTarefa;
  cor_hex: string;
  serie_id: string | null;
  recorrencia: RecorrenciaTarefa;
  recorrencia_fim_em: string | null;
  total_ocorrencias: number;
  lembrete_minutos: number | null;
  pessoas: PessoaTarefaResumo[];
  anexos: AnexoTarefaCalendario[];
  data_criacao: string;
  data_atualizacao: string;
}

export interface AnexoTarefaCalendario {
  id: number;
  tarefa_id: number;
  nome_arquivo: string;
  mime_type: string;
  tamanho_bytes: number;
  data_upload: string;
  url_stream: string;
  url_download: string;
  url_thumbnail: string | null;
}

export interface TarefaCalendarioPayload {
  titulo: string;
  descricao: string | null;
  inicio_em: string;
  fim_em: string | null;
  dia_inteiro: boolean;
  status: StatusTarefa;
  prioridade: PrioridadeTarefa;
  cor_hex: string;
  pessoas_ids: number[];
  recorrencia: RecorrenciaTarefa;
  recorrencia_fim_em: string | null;
  lembrete_minutos: number | null;
}

export interface HistoricoTarefa {
  id: number;
  tarefa_id: number;
  tipo: "CRIADA" | "ATUALIZADA" | "MOVIDA" | "STATUS_ALTERADO" | "ANEXO_ADICIONADO" | "ANEXO_EXCLUIDO";
  descricao: string;
  data_evento: string;
}

export interface ArmazenamentoTarefas {
  usado_bytes: number;
  limite_usuario_bytes: number;
  limite_tarefa_bytes: number;
  max_arquivo_bytes: number;
  anexos_total: number;
}

export interface ConsumoUsuarioAdmin {
  id: number;
  login: string;
  tarefas_total: number;
  anexos_tarefas_total: number;
  armazenamento_tarefas_bytes: number;
}

export interface DiagnosticoArmazenamento {
  banco_bytes: number;
  dossie_bytes: number;
  vinculos_bytes: number;
  tarefas_bytes: number;
  midia_total_bytes: number;
  anexos_total: number;
  pessoas_total: number;
  limite_usuario_tarefas_bytes: number;
  max_arquivo_bytes: number;
  usuarios: ConsumoUsuarioAdmin[];
}

export interface ImportacaoContatosResultado {
  pessoas_importadas: number;
  contatos_importados: number;
  registros_ignorados: number;
  avisos: string[];
}

export type TipoParametroBusca =
  | "NOME"
  | "CPF"
  | "CNPJ"
  | "EMAIL"
  | "TELEFONE"
  | "TERMO"
  | "PROCESSO";

export type FontePesquisaPublica =
  | "SEARXNG"
  | "QUERIDO_DIARIO"
  | "INLABS"
  | "OPENALEX"
  | "DATAJUD";

export interface ParametroBusca {
  id: number;
  pessoa_id: number;
  tipo: TipoParametroBusca;
  valor: string;
  provider: FontePesquisaPublica;
  ativo: boolean;
}

export interface ParametroBuscaPayload {
  tipo: TipoParametroBusca;
  valor: string;
  provider?: FontePesquisaPublica;
  ativo?: boolean;
}

export interface HistoricoBuscaPublica {
  id: number;
  pessoa_id: number;
  fonte: string;
  provider: FontePesquisaPublica;
  parametro_utilizado: string;
  titulo_resultado: string;
  snippet: string | null;
  url_origem: string;
  anexo_dossie_id: number | null;
  url_pdf: string | null;
  data_publicacao: string | null;
  detalhes: string | null;
  data_captura: string;
}

export interface HistoricoBuscaPaginado {
  itens: HistoricoBuscaPublica[];
  total: number;
  pagina: number;
  por_pagina: number;
  total_paginas: number;
}

export interface VarreduraPublicaResponse {
  situacao: "concluida" | "parcial" | "inconclusiva";
  parametros_processados: number;
  parametros_inconclusivos: number;
  resultados_encontrados: number;
  novos_achados: number;
  pdfs_arquivados: number;
  fontes_indisponiveis: number;
  avisos: string[];
}
