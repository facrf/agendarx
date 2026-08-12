export interface UsuarioSessao {
  id: number;
  login: string;
}

export interface LoginResponse {
  token: string;
  token_tipo: "Bearer";
  expira_em: number;
  usuario: UsuarioSessao;
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
}

export interface PessoaVinculo {
  id: number;
  pessoa_origem_id: number;
  pessoa_destino_id: number;
  tipo_vinculo: string;
  descricao: string | null;
  data_criacao: string;
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

export type TipoParametroBusca =
  | "NOME"
  | "CPF"
  | "CNPJ"
  | "EMAIL"
  | "TELEFONE"
  | "TERMO";

export interface ParametroBusca {
  id: number;
  pessoa_id: number;
  tipo: TipoParametroBusca;
  valor: string;
  ativo: boolean;
}

export interface ParametroBuscaPayload {
  tipo: TipoParametroBusca;
  valor: string;
  ativo?: boolean;
}

export interface HistoricoBuscaPublica {
  id: number;
  pessoa_id: number;
  fonte: string;
  parametro_utilizado: string;
  titulo_resultado: string;
  snippet: string | null;
  url_origem: string;
  anexo_dossie_id: number | null;
  url_pdf: string | null;
  data_captura: string;
}

export interface VarreduraPublicaResponse {
  parametros_processados: number;
  resultados_encontrados: number;
  novos_achados: number;
  pdfs_arquivados: number;
  avisos: string[];
}
