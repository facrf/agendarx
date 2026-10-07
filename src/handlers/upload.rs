use axum::extract::multipart::Field;
use bytes::{Bytes, BytesMut};

use crate::error::AppError;

/// Interrompe o recebimento antes de acumular um arquivo acima do limite.
pub(crate) async fn ler_campo(mut campo: Field<'_>, limite: usize) -> Result<Bytes, AppError> {
    let mut dados = BytesMut::new();
    while let Some(chunk) = campo.chunk().await? {
        if chunk.len() > limite.saturating_sub(dados.len()) {
            return Err(AppError::PayloadTooLarge);
        }
        dados.extend_from_slice(&chunk);
    }
    Ok(dados.freeze())
}
