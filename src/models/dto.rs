use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::Contato;

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub login: String,
    pub senha: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub token_tipo: &'static str,
    pub expira_em: i64,
    pub usuario: UsuarioSessao,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsuarioSessao {
    pub id: i64,
    pub login: String,
    pub tem_icone: bool,
    pub icone_atualizado_em: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TarefaCalendarioInput {
    pub titulo: String,
    pub descricao: Option<String>,
    pub inicio_em: String,
    pub fim_em: Option<String>,
    pub dia_inteiro: bool,
    pub status: String,
    pub prioridade: String,
    pub cor_hex: String,
    #[serde(default)]
    pub pessoas_ids: Vec<i64>,
    #[serde(default = "recorrencia_padrao")]
    pub recorrencia: String,
    pub recorrencia_fim_em: Option<String>,
    pub lembrete_minutos: Option<i64>,
}

fn recorrencia_padrao() -> String {
    "NENHUMA".to_owned()
}

#[derive(Debug, Deserialize)]
pub struct CalendarioFiltro {
    pub inicio: Option<String>,
    pub fim: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TarefaCalendarioDataInput {
    pub inicio_em: String,
    pub fim_em: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TarefaCalendarioStatusInput {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PessoaTarefaResumo {
    pub id: i64,
    pub nome: String,
    pub cor_hex: Option<String>,
    pub tem_foto: bool,
}

#[derive(Debug, Serialize)]
pub struct TarefaCalendarioResponse {
    pub id: i64,
    pub titulo: String,
    pub descricao: Option<String>,
    pub inicio_em: String,
    pub fim_em: Option<String>,
    pub dia_inteiro: bool,
    pub status: String,
    pub prioridade: String,
    pub cor_hex: String,
    pub serie_id: Option<String>,
    pub recorrencia: String,
    pub recorrencia_fim_em: Option<String>,
    pub total_ocorrencias: i64,
    pub lembrete_minutos: Option<i64>,
    pub pessoas: Vec<PessoaTarefaResumo>,
    pub anexos: Vec<AnexoTarefaResumo>,
    pub data_criacao: String,
    pub data_atualizacao: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct HistoricoTarefaResponse {
    pub id: i64,
    pub tarefa_id: i64,
    pub tipo: String,
    pub descricao: String,
    pub data_evento: String,
}

#[derive(Debug, Serialize)]
pub struct ArmazenamentoTarefasResponse {
    pub usado_bytes: i64,
    pub limite_usuario_bytes: i64,
    pub limite_tarefa_bytes: i64,
    pub max_arquivo_bytes: i64,
    pub anexos_total: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnexoTarefaResumo {
    pub id: i64,
    pub tarefa_id: i64,
    pub nome_arquivo: String,
    pub mime_type: String,
    pub tamanho_bytes: i64,
    pub data_upload: String,
    pub url_stream: String,
    pub url_download: String,
    pub url_thumbnail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CredenciaisInput {
    pub login: String,
    pub senha_atual: String,
    pub nova_senha: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CategoriaInput {
    pub nome_categoria: String,
    pub cor_hex: String,
}

#[derive(Debug, Deserialize)]
pub struct TipoMeioContatoInput {
    pub nome_tipo: String,
}

#[derive(Debug, Deserialize)]
pub struct PessoaInput {
    pub nome: String,
    pub categoria_id: Option<i64>,
    #[serde(default)]
    pub contatos: Vec<ContatoInput>,
}

#[derive(Debug, Deserialize)]
pub struct PessoaUpdateInput {
    pub nome: String,
    pub categoria_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ContatoInput {
    pub tipo_contato_id: i64,
    pub valor: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PessoaResumo {
    pub id: i64,
    pub nome: String,
    pub categoria_id: Option<i64>,
    pub nome_categoria: Option<String>,
    pub cor_hex: Option<String>,
    pub tem_foto: bool,
    pub data_cadastro: String,
}

#[derive(Debug, Serialize)]
pub struct PessoaDetalhe {
    #[serde(flatten)]
    pub pessoa: PessoaResumo,
    pub contatos: Vec<Contato>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AnexoResumo {
    pub id: i64,
    pub pessoa_id: i64,
    pub nome_arquivo: String,
    pub mime_type: String,
    pub tamanho_bytes: i64,
    pub data_upload: String,
    pub url_stream: String,
    pub url_download: String,
    pub url_thumbnail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnexoNomeInput {
    pub nome_arquivo: String,
}

#[derive(Debug, Deserialize)]
pub struct VinculoInput {
    pub pessoa_origem_id: i64,
    pub pessoa_destino_id: i64,
    pub tipo_vinculo: String,
    pub descricao: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnexoVinculoResumo {
    pub id: i64,
    pub vinculo_id: i64,
    pub nome_arquivo: String,
    pub mime_type: String,
    pub tamanho_bytes: i64,
    pub data_upload: String,
    pub url_stream: String,
    pub url_download: String,
    pub url_thumbnail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IdentidadeVisualResponse {
    pub tem_icone: bool,
    pub atualizado_em: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportacaoContatosResponse {
    pub pessoas_importadas: usize,
    pub contatos_importados: usize,
    pub registros_ignorados: usize,
    pub avisos: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GrafoResponse {
    pub nodes: Vec<GrafoNode>,
    pub edges: Vec<GrafoEdge>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct GrafoNode {
    pub id: i64,
    pub label: String,
    pub color: String,
    pub foto_url: Option<String>,
    pub categoria: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct GrafoEdge {
    pub id: i64,
    pub source: i64,
    pub target: i64,
    pub label: String,
    pub descricao: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MensagemResponse {
    pub mensagem: String,
}

#[derive(Debug, Deserialize)]
pub struct ParametroBuscaInput {
    pub tipo: String,
    pub valor: String,
    #[serde(default = "provider_busca_padrao")]
    pub provider: String,
    pub ativo: Option<bool>,
}

fn provider_busca_padrao() -> String {
    "SEARXNG".to_owned()
}

#[derive(Debug, Serialize)]
pub struct HistoricoBuscaResponse {
    pub id: i64,
    pub pessoa_id: i64,
    pub fonte: String,
    pub provider: String,
    pub parametro_utilizado: String,
    pub titulo_resultado: String,
    pub snippet: Option<String>,
    pub url_origem: String,
    pub anexo_dossie_id: Option<i64>,
    pub url_pdf: Option<String>,
    pub data_publicacao: Option<String>,
    pub detalhes: Option<String>,
    pub data_captura: String,
}

#[derive(Debug, Serialize)]
pub struct VarreduraResponse {
    pub situacao: String,
    pub parametros_processados: usize,
    pub parametros_inconclusivos: usize,
    pub resultados_encontrados: usize,
    pub novos_achados: usize,
    pub pdfs_arquivados: usize,
    pub fontes_indisponiveis: usize,
    pub avisos: Vec<String>,
}
