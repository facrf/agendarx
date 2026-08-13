export interface UsuarioSessao {
  id: number;
  login: string;
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
  nome_categoria: string | null;
  cor_hex: string | null;
  tem_foto: boolean;
  data_cadastro: string;
}

export interface PessoaDetalhe extends PessoaResumo {
  contatos: Contato[];
}

export interface PessoaPayload {
  nome: string;
  categoria_id: number | null;
  contatos?: ContatoPayload[];
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

export interface GrafoNode {
  id: number;
  label: string;
  color: string;
  foto_url: string | null;
  categoria: string | null;
}

export interface GrafoEdge {
  id: number;
  source: number;
  target: number;
  label: string;
  descricao: string | null;
}

export interface GrafoResponse {
  nodes: GrafoNode[];
  edges: GrafoEdge[];
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
  | "TERMO";

export type FontePesquisaPublica =
  | "SEARXNG"
  | "QUERIDO_DIARIO"
  | "INLABS"
  | "OPENALEX";

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
