use std::collections::HashMap;

use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
    routing::{get, post},
};

use crate::{AppState, error::AppError, models::ImportacaoContatosResponse};

const MAX_PESSOAS_IMPORTACAO: usize = 10_000;
const MAX_CONTATOS_POR_PESSOA: usize = 100;

pub fn rotas() -> Router<AppState> {
    Router::new()
        .route("/contatos/importar", post(importar_contatos))
        .route("/contatos/exportar/{formato}", get(exportar_contatos))
}

async fn importar_contatos(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ImportacaoContatosResponse>, AppError> {
    let mut arquivo = None;
    while let Some(campo) = multipart
        .next_field()
        .await
        .map_err(|erro| AppError::BadRequest(format!("multipart inválido: {erro}")))?
    {
        if campo.name() != Some("arquivo") {
            continue;
        }
        let nome = campo.file_name().unwrap_or("contatos.csv").to_owned();
        let conteudo = campo
            .bytes()
            .await
            .map_err(|erro| AppError::BadRequest(format!("falha no upload: {erro}")))?;
        arquivo = Some((nome, conteudo));
        break;
    }

    let (nome_arquivo, conteudo) = arquivo.ok_or_else(|| {
        AppError::BadRequest("envie o arquivo no campo multipart 'arquivo'".to_owned())
    })?;
    if conteudo.is_empty() {
        return Err(AppError::BadRequest(
            "o arquivo de contatos está vazio".to_owned(),
        ));
    }
    if conteudo.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }

    let texto = String::from_utf8_lossy(&conteudo);
    let mut avisos = Vec::new();
    if texto.contains('\u{fffd}') {
        avisos.push(
            "alguns caracteres inválidos foram substituídos; prefira arquivos UTF-8".to_owned(),
        );
    }
    let parece_vcard = nome_arquivo.to_ascii_lowercase().ends_with(".vcf")
        || texto.to_ascii_uppercase().contains("BEGIN:VCARD");
    let (pessoas, registros_ignorados) = if parece_vcard {
        parse_vcards(&texto, &mut avisos)?
    } else {
        parse_csv_contatos(&texto, &mut avisos)?
    };

    if pessoas.is_empty() {
        return Err(AppError::BadRequest(
            "nenhum contato válido foi encontrado no arquivo".to_owned(),
        ));
    }
    if pessoas.len() > MAX_PESSOAS_IMPORTACAO {
        return Err(AppError::BadRequest(format!(
            "o arquivo excede o limite de {MAX_PESSOAS_IMPORTACAO} pessoas por importação"
        )));
    }

    let (pessoas_importadas, contatos_importados) =
        persistir_contatos(&state, pessoas, &mut avisos).await?;
    Ok(Json(ImportacaoContatosResponse {
        pessoas_importadas,
        contatos_importados,
        registros_ignorados,
        avisos,
    }))
}

async fn exportar_contatos(
    State(state): State<AppState>,
    Path(formato): Path<String>,
) -> Result<Response, AppError> {
    let linhas = sqlx::query_as::<_, LinhaExportacao>(
        "SELECT p.id AS pessoa_id, p.nome, c.nome_categoria AS categoria, \
                t.nome_tipo AS tipo, co.valor \
         FROM pessoa p \
         LEFT JOIN categoria_pessoa c ON c.id = p.categoria_id \
         LEFT JOIN contato co ON co.pessoa_id = p.id \
         LEFT JOIN tipo_meio_contato t ON t.id = co.tipo_contato_id \
         ORDER BY p.nome COLLATE NOCASE, p.id, co.id",
    )
    .fetch_all(&state.pool)
    .await?;

    match formato.to_ascii_lowercase().as_str() {
        "csv" => resposta_download(
            exportar_csv(&linhas),
            "text/csv; charset=utf-8",
            "agendarx-contatos.csv",
        ),
        "vcf" | "vcard" => resposta_download(
            exportar_vcard(&linhas),
            "text/vcard; charset=utf-8",
            "agendarx-contatos.vcf",
        ),
        _ => Err(AppError::BadRequest(
            "formato inválido; use csv ou vcf".to_owned(),
        )),
    }
}

async fn persistir_contatos(
    state: &AppState,
    pessoas: Vec<ContatoImportado>,
    avisos: &mut Vec<String>,
) -> Result<(usize, usize), AppError> {
    let mut tx = state.pool.begin().await?;
    let tipos_existentes = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, nome_tipo FROM tipo_meio_contato ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    let categorias_existentes = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, nome_categoria FROM categoria_pessoa ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut tipos: HashMap<String, i64> = tipos_existentes
        .into_iter()
        .map(|(id, nome)| (chave_texto(&nome), id))
        .collect();
    let mut categorias: HashMap<String, i64> = categorias_existentes
        .into_iter()
        .map(|(id, nome)| (chave_texto(&nome), id))
        .collect();

    let mut pessoas_importadas = 0;
    let mut contatos_importados = 0;
    for mut pessoa in pessoas {
        pessoa.nome = pessoa.nome.trim().chars().take(255).collect();
        if pessoa.nome.is_empty() {
            continue;
        }
        if pessoa.campos.len() > MAX_CONTATOS_POR_PESSOA {
            avisos.push(format!(
                "{}: somente os primeiros {MAX_CONTATOS_POR_PESSOA} meios foram importados",
                pessoa.nome
            ));
            pessoa.campos.truncate(MAX_CONTATOS_POR_PESSOA);
        }

        let categoria_id = if let Some(nome_categoria) = pessoa
            .categoria
            .as_deref()
            .map(str::trim)
            .filter(|nome| !nome.is_empty())
        {
            let nome_categoria: String = nome_categoria.chars().take(100).collect();
            let chave = chave_texto(&nome_categoria);
            if let Some(id) = categorias.get(&chave) {
                Some(*id)
            } else {
                let id: i64 = sqlx::query_scalar(
                    "INSERT INTO categoria_pessoa (nome_categoria, cor_hex) VALUES (?, '#64748B') RETURNING id",
                )
                .bind(&nome_categoria)
                .fetch_one(&mut *tx)
                .await?;
                categorias.insert(chave, id);
                Some(id)
            }
        } else {
            None
        };

        let pessoa_id: i64 = sqlx::query_scalar(
            "INSERT INTO pessoa (nome, categoria_id) VALUES (?, ?) RETURNING id",
        )
        .bind(&pessoa.nome)
        .bind(categoria_id)
        .fetch_one(&mut *tx)
        .await?;
        pessoas_importadas += 1;

        for campo in pessoa.campos {
            let nome_tipo: String = campo.tipo.trim().chars().take(100).collect();
            let valor: String = campo.valor.trim().chars().take(2_048).collect();
            if nome_tipo.is_empty() || valor.is_empty() {
                continue;
            }
            let chave = chave_texto(&nome_tipo);
            let tipo_id = if let Some(id) = tipos.get(&chave) {
                *id
            } else {
                let id: i64 = sqlx::query_scalar(
                    "INSERT INTO tipo_meio_contato (nome_tipo) VALUES (?) RETURNING id",
                )
                .bind(&nome_tipo)
                .fetch_one(&mut *tx)
                .await?;
                tipos.insert(chave, id);
                id
            };
            sqlx::query("INSERT INTO contato (pessoa_id, tipo_contato_id, valor) VALUES (?, ?, ?)")
                .bind(pessoa_id)
                .bind(tipo_id)
                .bind(valor)
                .execute(&mut *tx)
                .await?;
            contatos_importados += 1;
        }
    }

    tx.commit().await?;
    Ok((pessoas_importadas, contatos_importados))
}

fn parse_csv_contatos(
    texto: &str,
    avisos: &mut Vec<String>,
) -> Result<(Vec<ContatoImportado>, usize), AppError> {
    let delimitador = detectar_delimitador(texto);
    let linhas = parse_csv(texto.trim_start_matches('\u{feff}'), delimitador)?;
    let Some(cabecalho) = linhas.first() else {
        return Ok((Vec::new(), 0));
    };
    let cabecalhos: Vec<String> = cabecalho
        .iter()
        .map(|valor| normalizar_cabecalho(valor))
        .collect();
    let indice_nome = localizar_coluna(
        &cabecalhos,
        &[
            "nome",
            "name",
            "nome completo",
            "full name",
            "display name",
            "nome de exibicao",
        ],
    );
    let indice_nome_proprio = localizar_coluna(&cabecalhos, &["given name", "first name"]);
    let indice_sobrenome = localizar_coluna(&cabecalhos, &["family name", "last name", "surname"]);
    let indice_id = localizar_coluna(&cabecalhos, &["id", "agendarx id"]);
    let indice_categoria = localizar_coluna(
        &cabecalhos,
        &[
            "categoria",
            "category",
            "categories",
            "group membership",
            "labels",
        ],
    );
    let indice_tipo = localizar_coluna(&cabecalhos, &["tipo", "tipo de contato", "contact type"]);
    let indice_valor = localizar_coluna(&cabecalhos, &["valor", "value", "contact value"]);

    if indice_nome.is_none() && indice_nome_proprio.is_none() && indice_sobrenome.is_none() {
        return Err(AppError::BadRequest(
            "CSV sem coluna de nome reconhecida".to_owned(),
        ));
    }

    let mut pessoas = Vec::<ContatoImportado>::new();
    let mut grupos = HashMap::<String, usize>::new();
    let mut ignorados = 0;
    for (numero, linha) in linhas.iter().enumerate().skip(1) {
        if linha.iter().all(|valor| valor.trim().is_empty()) {
            continue;
        }
        let nome = indice_nome
            .and_then(|indice| linha.get(indice))
            .map(|valor| valor.trim().to_owned())
            .filter(|valor| !valor.is_empty())
            .unwrap_or_else(|| {
                let proprio = indice_nome_proprio
                    .and_then(|indice| linha.get(indice))
                    .map(|valor| valor.trim())
                    .unwrap_or_default();
                let sobrenome = indice_sobrenome
                    .and_then(|indice| linha.get(indice))
                    .map(|valor| valor.trim())
                    .unwrap_or_default();
                format!("{proprio} {sobrenome}").trim().to_owned()
            });
        if nome.is_empty() {
            ignorados += 1;
            if avisos.len() < 20 {
                avisos.push(format!(
                    "linha {} ignorada por não possuir nome",
                    numero + 1
                ));
            }
            continue;
        }

        let chave_grupo = indice_id
            .and_then(|indice| linha.get(indice))
            .map(|valor| valor.trim())
            .filter(|valor| !valor.is_empty())
            .map(|valor| format!("id:{valor}"))
            .unwrap_or_else(|| format!("linha:{numero}"));
        let pessoa_indice = if let Some(indice) = grupos.get(&chave_grupo) {
            *indice
        } else {
            let categoria = indice_categoria
                .and_then(|indice| linha.get(indice))
                .map(|valor| limpar_categoria_importada(valor))
                .filter(|valor| !valor.is_empty());
            pessoas.push(ContatoImportado {
                nome,
                categoria,
                campos: Vec::new(),
            });
            let indice = pessoas.len() - 1;
            grupos.insert(chave_grupo, indice);
            indice
        };

        if let (Some(tipo_idx), Some(valor_idx)) = (indice_tipo, indice_valor) {
            let tipo = linha.get(tipo_idx).map(String::as_str).unwrap_or_default();
            let valor = linha.get(valor_idx).map(String::as_str).unwrap_or_default();
            adicionar_campo(&mut pessoas[pessoa_indice], tipo, valor);
            continue;
        }

        for (indice, cabecalho) in cabecalhos.iter().enumerate() {
            let Some(tipo) = tipo_por_cabecalho(cabecalho) else {
                continue;
            };
            let valor = linha.get(indice).map(String::as_str).unwrap_or_default();
            for item in valor.split(" ::: ") {
                adicionar_campo(&mut pessoas[pessoa_indice], tipo, item);
            }
        }
    }

    Ok((pessoas, ignorados))
}

fn parse_vcards(
    texto: &str,
    avisos: &mut Vec<String>,
) -> Result<(Vec<ContatoImportado>, usize), AppError> {
    let linhas = desdobrar_linhas_vcard(texto.trim_start_matches('\u{feff}'));
    let mut pessoas = Vec::new();
    let mut atual: Option<ContatoImportado> = None;
    let mut nome_estruturado: Option<String> = None;
    let mut ignorados = 0;

    for linha in linhas {
        if linha.eq_ignore_ascii_case("BEGIN:VCARD") {
            atual = Some(ContatoImportado::default());
            nome_estruturado = None;
            continue;
        }
        if linha.eq_ignore_ascii_case("END:VCARD") {
            if let Some(mut pessoa) = atual.take() {
                if pessoa.nome.trim().is_empty() {
                    pessoa.nome = nome_estruturado.take().unwrap_or_default();
                }
                if pessoa.nome.trim().is_empty() {
                    ignorados += 1;
                    if avisos.len() < 20 {
                        avisos.push("vCard ignorado por não possuir FN ou N".to_owned());
                    }
                } else {
                    pessoas.push(pessoa);
                }
            }
            continue;
        }
        let Some(pessoa) = atual.as_mut() else {
            continue;
        };
        let Some((propriedade, valor_bruto)) = linha.split_once(':') else {
            continue;
        };
        let nome_propriedade = propriedade
            .split(';')
            .next()
            .unwrap_or_default()
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let valor = decodificar_valor_vcard(valor_bruto, propriedade);
        match nome_propriedade.as_str() {
            "FN" => pessoa.nome = valor.trim().to_owned(),
            "N" => {
                let partes: Vec<&str> = valor.split(';').collect();
                let sobrenome = partes.first().copied().unwrap_or_default().trim();
                let nome = partes.get(1).copied().unwrap_or_default().trim();
                nome_estruturado = Some(format!("{nome} {sobrenome}").trim().to_owned());
            }
            "EMAIL" => adicionar_campo(pessoa, "E-mail", &valor),
            "TEL" => adicionar_campo(pessoa, tipo_telefone_vcard(propriedade), &valor),
            "URL" => adicionar_campo(pessoa, "Site", &valor),
            "ADR" => adicionar_campo(
                pessoa,
                "Endereço",
                &valor
                    .split(';')
                    .filter(|parte| !parte.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            "ORG" => adicionar_campo(pessoa, "Organização", &valor.replace(';', " · ")),
            "NOTE" => adicionar_campo(pessoa, "Observação", &valor),
            "CATEGORIES" => {
                pessoa.categoria = valor.split(',').next().map(str::trim).map(str::to_owned)
            }
            nome if nome.starts_with("X-AGENDARX-") => {
                let tipo = nome.trim_start_matches("X-AGENDARX-").replace('-', " ");
                adicionar_campo(pessoa, &capitalizar(&tipo), &valor);
            }
            _ => {}
        }
    }

    if atual.is_some() {
        return Err(AppError::BadRequest(
            "vCard incompleto: END:VCARD não encontrado".to_owned(),
        ));
    }
    Ok((pessoas, ignorados))
}

fn exportar_csv(linhas: &[LinhaExportacao]) -> Vec<u8> {
    let mut saida = String::from("\u{feff}ID,Nome,Categoria,Tipo,Valor\r\n");
    for linha in linhas {
        let valores = [
            linha.pessoa_id.to_string(),
            linha.nome.clone(),
            linha.categoria.clone().unwrap_or_default(),
            linha.tipo.clone().unwrap_or_default(),
            linha.valor.clone().unwrap_or_default(),
        ];
        saida.push_str(
            &valores
                .iter()
                .map(|valor| escapar_csv(valor))
                .collect::<Vec<_>>()
                .join(","),
        );
        saida.push_str("\r\n");
    }
    saida.into_bytes()
}

fn exportar_vcard(linhas: &[LinhaExportacao]) -> Vec<u8> {
    let mut saida = String::new();
    let mut indice = 0;
    while indice < linhas.len() {
        let pessoa_id = linhas[indice].pessoa_id;
        let pessoa = &linhas[indice];
        saida.push_str("BEGIN:VCARD\r\nVERSION:4.0\r\n");
        saida.push_str(&format!("FN:{}\r\n", escapar_vcard(&pessoa.nome)));
        saida.push_str(&format!("N:{};;;;\r\n", escapar_vcard(&pessoa.nome)));
        if let Some(categoria) = pessoa.categoria.as_deref() {
            saida.push_str(&format!("CATEGORIES:{}\r\n", escapar_vcard(categoria)));
        }
        while indice < linhas.len() && linhas[indice].pessoa_id == pessoa_id {
            let linha = &linhas[indice];
            if let (Some(tipo), Some(valor)) = (linha.tipo.as_deref(), linha.valor.as_deref()) {
                let normalizado = chave_texto(tipo);
                let propriedade = if normalizado.contains("mail") {
                    "EMAIL".to_owned()
                } else if normalizado.contains("telefone")
                    || normalizado.contains("celular")
                    || normalizado.contains("phone")
                    || normalizado.contains("whatsapp")
                {
                    format!("TEL;TYPE={}", slug_vcard(tipo))
                } else if normalizado.contains("site")
                    || normalizado.contains("url")
                    || normalizado.contains("web")
                {
                    "URL".to_owned()
                } else if normalizado.contains("endereco") || normalizado.contains("address") {
                    "ADR".to_owned()
                } else if normalizado.contains("organizacao") || normalizado.contains("company") {
                    "ORG".to_owned()
                } else if normalizado.contains("observacao") || normalizado.contains("note") {
                    "NOTE".to_owned()
                } else {
                    format!("X-AGENDARX-{}", slug_vcard(tipo))
                };
                if propriedade == "ADR" {
                    saida.push_str(&format!("ADR:;;{};;;;\r\n", escapar_vcard(valor)));
                } else {
                    saida.push_str(&format!("{propriedade}:{}\r\n", escapar_vcard(valor)));
                }
            }
            indice += 1;
        }
        saida.push_str("END:VCARD\r\n");
    }
    saida.into_bytes()
}

fn resposta_download(
    conteudo: Vec<u8>,
    mime_type: &'static str,
    nome_arquivo: &'static str,
) -> Result<Response, AppError> {
    let mut resposta = Response::new(Body::from(conteudo));
    *resposta.status_mut() = StatusCode::OK;
    resposta
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime_type));
    resposta.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static(match nome_arquivo {
            "agendarx-contatos.csv" => "attachment; filename=\"agendarx-contatos.csv\"",
            _ => "attachment; filename=\"agendarx-contatos.vcf\"",
        }),
    );
    resposta.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(resposta)
}

fn adicionar_campo(pessoa: &mut ContatoImportado, tipo: &str, valor: &str) {
    let tipo = tipo.trim();
    let valor = valor.trim();
    if tipo.is_empty() || valor.is_empty() {
        return;
    }
    if pessoa
        .campos
        .iter()
        .any(|campo| chave_texto(&campo.tipo) == chave_texto(tipo) && campo.valor == valor)
    {
        return;
    }
    pessoa.campos.push(CampoImportado {
        tipo: tipo.to_owned(),
        valor: valor.to_owned(),
    });
}

fn detectar_delimitador(texto: &str) -> char {
    let primeira = texto
        .lines()
        .find(|linha| !linha.trim().is_empty())
        .unwrap_or_default();
    let mut aspas = false;
    let mut virgulas = 0;
    let mut ponto_virgulas = 0;
    for caractere in primeira.chars() {
        match caractere {
            '"' => aspas = !aspas,
            ',' if !aspas => virgulas += 1,
            ';' if !aspas => ponto_virgulas += 1,
            _ => {}
        }
    }
    if ponto_virgulas > virgulas { ';' } else { ',' }
}

fn parse_csv(texto: &str, delimitador: char) -> Result<Vec<Vec<String>>, AppError> {
    let mut linhas = Vec::new();
    let mut linha = Vec::new();
    let mut campo = String::new();
    let mut aspas = false;
    let mut caracteres = texto.chars().peekable();
    while let Some(caractere) = caracteres.next() {
        if aspas {
            if caractere == '"' {
                if caracteres.peek() == Some(&'"') {
                    campo.push('"');
                    caracteres.next();
                } else {
                    aspas = false;
                }
            } else {
                campo.push(caractere);
            }
            continue;
        }
        match caractere {
            '"' if campo.is_empty() => aspas = true,
            valor if valor == delimitador => {
                linha.push(std::mem::take(&mut campo));
            }
            '\n' => {
                linha.push(std::mem::take(&mut campo));
                linhas.push(std::mem::take(&mut linha));
            }
            '\r' if caracteres.peek() == Some(&'\n') => {}
            '\r' => {
                linha.push(std::mem::take(&mut campo));
                linhas.push(std::mem::take(&mut linha));
            }
            _ => campo.push(caractere),
        }
    }
    if aspas {
        return Err(AppError::BadRequest(
            "CSV inválido: campo entre aspas não foi encerrado".to_owned(),
        ));
    }
    if !campo.is_empty() || !linha.is_empty() {
        linha.push(campo);
        linhas.push(linha);
    }
    Ok(linhas)
}

fn localizar_coluna(cabecalhos: &[String], nomes: &[&str]) -> Option<usize> {
    cabecalhos
        .iter()
        .position(|cabecalho| nomes.iter().any(|nome| cabecalho == nome))
}

fn tipo_por_cabecalho(cabecalho: &str) -> Option<&'static str> {
    let google_valor = cabecalho.ends_with(" - value") || cabecalho.ends_with(" - formatted");
    if (google_valor || ["email", "e-mail", "email address"].contains(&cabecalho))
        && (cabecalho.contains("mail"))
    {
        Some("E-mail")
    } else if (google_valor
        || [
            "telefone",
            "phone",
            "mobile phone",
            "home phone",
            "business phone",
        ]
        .contains(&cabecalho))
        && (cabecalho.contains("phone") || cabecalho.contains("telefone"))
    {
        if cabecalho.contains("mobile") {
            Some("Celular")
        } else {
            Some("Telefone")
        }
    } else if (google_valor || ["website", "web page", "url", "site"].contains(&cabecalho))
        && (cabecalho.contains("website")
            || cabecalho.contains("web page")
            || cabecalho == "url"
            || cabecalho == "site")
    {
        Some("Site")
    } else if (google_valor || ["address", "endereco"].contains(&cabecalho))
        && (cabecalho.contains("address") || cabecalho.contains("endereco"))
    {
        Some("Endereço")
    } else if (google_valor
        || cabecalho.ends_with(" - name")
        || ["organization", "company", "empresa"].contains(&cabecalho))
        && (cabecalho.contains("organization")
            || cabecalho.contains("company")
            || cabecalho.contains("empresa"))
    {
        Some("Organização")
    } else if ["notes", "note", "observacao", "observacoes"].contains(&cabecalho) {
        Some("Observação")
    } else {
        None
    }
}

fn normalizar_cabecalho(valor: &str) -> String {
    valor
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            outro => outro,
        })
        .collect()
}

fn chave_texto(valor: &str) -> String {
    normalizar_cabecalho(valor)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn limpar_categoria_importada(valor: &str) -> String {
    valor
        .split(" ::: ")
        .find(|item| !item.trim().is_empty() && !item.trim().eq_ignore_ascii_case("* myContacts"))
        .unwrap_or_default()
        .trim()
        .trim_start_matches('*')
        .trim()
        .to_owned()
}

fn desdobrar_linhas_vcard(texto: &str) -> Vec<String> {
    let mut resultado: Vec<String> = Vec::new();
    for linha in texto.replace("\r\n", "\n").replace('\r', "\n").lines() {
        if let Some(anterior) = resultado.last_mut().filter(|item| item.ends_with('=')) {
            anterior.pop();
            anterior.push_str(linha.trim_start());
        } else if linha.starts_with(' ') || linha.starts_with('\t') {
            if let Some(anterior) = resultado.last_mut() {
                anterior.push_str(linha.trim_start());
            }
        } else {
            resultado.push(linha.to_owned());
        }
    }
    resultado
}

fn decodificar_valor_vcard(valor: &str, propriedade: &str) -> String {
    let bruto = if propriedade
        .to_ascii_uppercase()
        .contains("ENCODING=QUOTED-PRINTABLE")
    {
        decodificar_quoted_printable(valor)
    } else {
        valor.to_owned()
    };
    bruto
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

fn decodificar_quoted_printable(valor: &str) -> String {
    let bytes = valor.as_bytes();
    let mut saida = Vec::with_capacity(bytes.len());
    let mut indice = 0;
    while indice < bytes.len() {
        if bytes[indice] == b'='
            && indice + 2 < bytes.len()
            && let (Some(a), Some(b)) = (hex(bytes[indice + 1]), hex(bytes[indice + 2]))
        {
            saida.push((a << 4) | b);
            indice += 3;
            continue;
        }
        saida.push(bytes[indice]);
        indice += 1;
    }
    String::from_utf8_lossy(&saida).into_owned()
}

fn hex(valor: u8) -> Option<u8> {
    match valor {
        b'0'..=b'9' => Some(valor - b'0'),
        b'a'..=b'f' => Some(valor - b'a' + 10),
        b'A'..=b'F' => Some(valor - b'A' + 10),
        _ => None,
    }
}

fn tipo_telefone_vcard(propriedade: &str) -> &'static str {
    let propriedade = propriedade.to_ascii_uppercase();
    if propriedade.contains("WHATSAPP") {
        "WhatsApp"
    } else if propriedade.contains("CELL") || propriedade.contains("MOBILE") {
        "Celular"
    } else {
        "Telefone"
    }
}

fn capitalizar(valor: &str) -> String {
    let mut caracteres = valor.to_lowercase().chars().collect::<Vec<_>>();
    if let Some(primeiro) = caracteres.first_mut() {
        primeiro.make_ascii_uppercase();
    }
    caracteres.into_iter().collect()
}

fn escapar_csv(valor: &str) -> String {
    let valor_seguro;
    let perigoso = matches!(
        valor.trim_start().chars().next(),
        Some('=' | '+' | '-' | '@')
    );
    let valor = if perigoso {
        valor_seguro = format!("\t{valor}");
        &valor_seguro
    } else {
        valor
    };
    if valor.contains(',')
        || valor.contains('"')
        || valor.contains('\r')
        || valor.contains('\n')
        || valor.contains('\t')
    {
        format!("\"{}\"", valor.replace('"', "\"\""))
    } else {
        valor.to_owned()
    }
}

fn escapar_vcard(valor: &str) -> String {
    valor
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

fn slug_vcard(valor: &str) -> String {
    let slug: String = normalizar_cabecalho(valor)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        "CONTATO".to_owned()
    } else {
        slug
    }
}

#[derive(Default, Debug)]
struct ContatoImportado {
    nome: String,
    categoria: Option<String>,
    campos: Vec<CampoImportado>,
}

#[derive(Debug)]
struct CampoImportado {
    tipo: String,
    valor: String,
}

#[derive(sqlx::FromRow, Debug)]
struct LinhaExportacao {
    pessoa_id: i64,
    nome: String,
    categoria: Option<String>,
    tipo: Option<String>,
    valor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{detectar_delimitador, escapar_csv, parse_csv, parse_csv_contatos, parse_vcards};

    #[test]
    fn interpreta_csv_google_com_aspas_e_multiplos_valores() {
        let csv = "Name,E-mail 1 - Value,Phone 1 - Value\r\n\"Maria, Silva\",maria@example.com,+5511999 ::: +5511888\r\n";
        let (pessoas, ignorados) = parse_csv_contatos(csv, &mut Vec::new()).unwrap();
        assert_eq!(ignorados, 0);
        assert_eq!(pessoas.len(), 1);
        assert_eq!(pessoas[0].nome, "Maria, Silva");
        assert_eq!(pessoas[0].campos.len(), 3);
    }

    #[test]
    fn interpreta_csv_separado_por_ponto_e_virgula() {
        let csv = "Nome;E-mail\nJoão;joao@example.com\n";
        assert_eq!(detectar_delimitador(csv), ';');
        let linhas = parse_csv(csv, ';').unwrap();
        assert_eq!(linhas[1][0], "João");
    }

    #[test]
    fn reagrupa_csv_exportado_pelo_agendarx() {
        let csv = "ID,Nome,Categoria,Tipo,Valor\r\n7,Ana,Amigos,E-mail,ana@example.com\r\n7,Ana,Amigos,Telefone,+5511999\r\n";
        let (pessoas, ignorados) = parse_csv_contatos(csv, &mut Vec::new()).unwrap();
        assert_eq!(ignorados, 0);
        assert_eq!(pessoas.len(), 1);
        assert_eq!(pessoas[0].categoria.as_deref(), Some("Amigos"));
        assert_eq!(pessoas[0].campos.len(), 2);
    }

    #[test]
    fn interpreta_vcard_e_quoted_printable() {
        let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN;ENCODING=QUOTED-PRINTABLE:Jos=C3=A9 Silva\r\nTEL;TYPE=CELL:+5511999\r\nEMAIL:jose@example.com\r\nEND:VCARD\r\n";
        let (pessoas, ignorados) = parse_vcards(vcard, &mut Vec::new()).unwrap();
        assert_eq!(ignorados, 0);
        assert_eq!(pessoas[0].nome, "José Silva");
        assert_eq!(pessoas[0].campos.len(), 2);
    }

    #[test]
    fn escapa_csv_rfc4180() {
        assert_eq!(escapar_csv("Silva, Maria"), "\"Silva, Maria\"");
        assert_eq!(escapar_csv("a\"b"), "\"a\"\"b\"");
        assert_eq!(escapar_csv("=2+2"), "\"\t=2+2\"");
        assert_eq!(escapar_csv("+5511999"), "\"\t+5511999\"");
    }
}
