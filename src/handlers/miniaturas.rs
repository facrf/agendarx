use std::io::Cursor;

use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use bytes::Bytes;
use image::{ExtendedColorType, ImageEncoder, ImageReader, Limits, codecs::webp::WebPEncoder};

use crate::error::AppError;

const LADO_MAXIMO: u32 = 512;
const DIMENSAO_FONTE_MAXIMA: u32 = 20_000;
const MEMORIA_DECODIFICACAO_MAXIMA: u64 = 256 * 1024 * 1024;

pub struct MiniaturaGerada {
    pub conteudo: Vec<u8>,
    pub largura: u32,
    pub altura: u32,
}

pub fn mime_suportado(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/bmp"
            | "image/gif"
            | "image/jpeg"
            | "image/jpg"
            | "image/pjpeg"
            | "image/png"
            | "image/tiff"
            | "image/webp"
            | "image/vnd.microsoft.icon"
            | "image/x-bmp"
            | "image/x-icon"
            | "image/x-png"
            | "image/x-tiff"
    )
}

pub async fn gerar(conteudo: Bytes) -> Result<Option<MiniaturaGerada>, AppError> {
    tokio::task::spawn_blocking(move || gerar_sincrona(&conteudo))
        .await
        .map_err(|_| AppError::interno("falha ao processar miniatura"))
}

fn gerar_sincrona(conteudo: &[u8]) -> Option<MiniaturaGerada> {
    let mut leitor = ImageReader::new(Cursor::new(conteudo))
        .with_guessed_format()
        .ok()?;
    let mut limites = Limits::default();
    limites.max_image_width = Some(DIMENSAO_FONTE_MAXIMA);
    limites.max_image_height = Some(DIMENSAO_FONTE_MAXIMA);
    limites.max_alloc = Some(MEMORIA_DECODIFICACAO_MAXIMA);
    leitor.limits(limites);

    let imagem = leitor.decode().ok()?;
    let miniatura = imagem.thumbnail(LADO_MAXIMO, LADO_MAXIMO).to_rgba8();
    let (largura, altura) = miniatura.dimensions();
    let mut conteudo_webp = Vec::new();
    WebPEncoder::new_lossless(&mut conteudo_webp)
        .write_image(
            miniatura.as_raw(),
            largura,
            altura,
            ExtendedColorType::Rgba8,
        )
        .ok()?;

    Some(MiniaturaGerada {
        conteudo: conteudo_webp,
        largura,
        altura,
    })
}

pub fn responder(conteudo: Vec<u8>) -> Response {
    let tamanho = conteudo.len();
    let mut response = Response::new(Body::from(conteudo));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&tamanho.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        // O cache persistente fica no SQLite. O navegador não deve conservar
        // miniaturas sensíveis depois que a sessão for revogada.
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

    use axum::http::header;

    use super::{LADO_MAXIMO, gerar_sincrona, mime_suportado, responder};

    #[test]
    fn gera_webp_limitado_preservando_proporcao() {
        let imagem = RgbImage::from_pixel(1_200, 600, Rgb([20, 120, 180]));
        let mut png = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(imagem)
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();

        let miniatura = gerar_sincrona(png.get_ref()).unwrap();

        assert_eq!((miniatura.largura, miniatura.altura), (LADO_MAXIMO, 256));
        assert_eq!(
            infer::get(&miniatura.conteudo).map(|tipo| tipo.mime_type()),
            Some("image/webp")
        );
    }

    #[test]
    fn restringe_formatos_decodificados() {
        assert!(mime_suportado("image/jpeg"));
        assert!(mime_suportado("image/jpg"));
        assert!(mime_suportado("image/png"));
        assert!(!mime_suportado("image/svg+xml"));
        assert!(!mime_suportado("application/pdf"));
    }

    #[test]
    fn resposta_nao_persiste_midia_sensivel_no_navegador() {
        let resposta = responder(vec![1, 2, 3]);

        assert_eq!(resposta.headers()[header::CONTENT_TYPE], "image/webp");
        assert_eq!(
            resposta.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
    }
}
