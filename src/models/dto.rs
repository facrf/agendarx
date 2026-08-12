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
    pub ativo: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct HistoricoBuscaResponse {
    pub id: i64,
    pub pessoa_id: i64,
    pub fonte: String,
    pub parametro_utilizado: String,
    pub titulo_resultado: String,
    pub snippet: Option<String>,
    pub url_origem: String,
    pub anexo_dossie_id: Option<i64>,
    pub url_pdf: Option<String>,
    pub data_captura: String,
}

#[derive(Debug, Serialize)]
pub struct VarreduraResponse {
    pub parametros_processados: usize,
    pub resultados_encontrados: usize,
    pub novos_achados: usize,
    pub pdfs_arquivados: usize,
    pub avisos: Vec<String>,
}
