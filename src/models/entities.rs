use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Usuario {
    pub id: i64,
    pub login: String,
    #[serde(skip_serializing)]
    pub senha_hash: String,
    #[serde(skip_serializing)]
    pub icone_admin_blob: Option<Vec<u8>>,
    pub icone_admin_mime_type: Option<String>,
    pub icone_admin_atualizado_em: Option<String>,
    pub data_criacao: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TarefaCalendarioRow {
    pub id: i64,
    pub usuario_id: i64,
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
    pub lembrete_minutos: Option<i64>,
    pub lembrete_dispensado_em: Option<String>,
    pub data_criacao: String,
    pub data_atualizacao: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AnexoTarefaCalendario {
    pub id: i64,
    pub tarefa_id: i64,
    pub nome_arquivo: String,
    pub mime_type: String,
    #[serde(skip_serializing)]
    pub conteudo_blob: Vec<u8>,
    pub tamanho_bytes: i64,
    pub data_upload: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CategoriaPessoa {
    pub id: i64,
    pub nome_categoria: String,
    pub cor_hex: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TipoMeioContato {
    pub id: i64,
    pub nome_tipo: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[allow(dead_code)]
pub struct Pessoa {
    pub id: i64,
    pub nome: String,
    pub categoria_id: Option<i64>,
    pub descricao: Option<String>,
    #[serde(skip_serializing)]
    pub foto_principal: Option<Vec<u8>>,
    pub data_cadastro: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Contato {
    pub id: i64,
    pub pessoa_id: i64,
    pub tipo_contato_id: i64,
    pub valor: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AnexoDossie {
    pub id: i64,
    pub pessoa_id: i64,
    pub nome_arquivo: String,
    pub mime_type: String,
    #[serde(skip_serializing)]
    pub conteudo_blob: Vec<u8>,
    pub tamanho_bytes: i64,
    pub data_upload: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PessoaVinculo {
    pub id: i64,
    pub pessoa_origem_id: i64,
    pub pessoa_destino_id: i64,
    pub tipo_vinculo: String,
    pub descricao: Option<String>,
    pub data_criacao: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AnexoVinculo {
    pub id: i64,
    pub vinculo_id: i64,
    pub nome_arquivo: String,
    pub mime_type: String,
    #[serde(skip_serializing)]
    pub conteudo_blob: Vec<u8>,
    pub tamanho_bytes: i64,
    pub data_upload: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ParametroBusca {
    pub id: i64,
    pub pessoa_id: i64,
    pub tipo: String,
    pub valor: String,
    pub provider: String,
    pub ativo: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct HistoricoBuscaPublica {
    pub id: i64,
    pub pessoa_id: i64,
    pub fonte: String,
    pub provider: String,
    pub parametro_utilizado: String,
    pub titulo_resultado: String,
    pub snippet: Option<String>,
    pub url_origem: String,
    pub anexo_dossie_id: Option<i64>,
    pub data_publicacao: Option<String>,
    pub detalhes: Option<String>,
    pub data_captura: String,
}
