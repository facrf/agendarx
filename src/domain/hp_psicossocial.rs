use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaixaIndicador {
    pub min: f64,
    pub nome: String,
    pub cor_hex: String,
    #[serde(default)]
    pub pulsante: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametrosHp {
    pub ativo: bool,
    pub toxicidade_min: f64,
    pub toxicidade_max: f64,
    pub hp_base: f64,
    pub hp_min: f64,
    pub fator_segundo_grau: f64,
    pub pesos_vinculo: BTreeMap<String, f64>,
    pub peso_padrao: f64,
    pub faixas_aura: Vec<FaixaIndicador>,
    pub faixas_vitalidade: Vec<FaixaIndicador>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigHpPsicossocial {
    pub versao: i64,
    #[serde(flatten)]
    pub parametros: ParametrosHp,
}

fn faixa(min: f64, nome: &str, cor: &str, pulsante: bool) -> FaixaIndicador {
    FaixaIndicador {
        min,
        nome: nome.into(),
        cor_hex: cor.into(),
        pulsante,
    }
}

impl Default for ParametrosHp {
    fn default() -> Self {
        Self {
            ativo: true,
            toxicidade_min: 0.10,
            toxicidade_max: 0.50,
            hp_base: 1.0,
            hp_min: 0.05,
            fator_segundo_grau: 0.20,
            pesos_vinculo: BTreeMap::from([
                ("família".into(), 1.0),
                ("familia".into(), 1.0),
                ("profissional".into(), 0.50),
            ]),
            peso_padrao: 0.50,
            faixas_aura: vec![
                faixa(0.0, "Estável", "#22C55E", false),
                faixa(0.10, "Elevado", "#EAB308", false),
                faixa(0.25, "Observação", "#F97316", false),
                faixa(0.40, "Crítico", "#EF4444", true),
            ],
            faixas_vitalidade: vec![
                faixa(0.0, "Impactada", "#EF4444", false),
                faixa(0.25, "Baixa", "#F97316", false),
                faixa(0.50, "Atenção", "#EAB308", false),
                faixa(0.75, "Preservada", "#22C55E", false),
            ],
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NoPsicossocial {
    pub pessoa_id: i64,
    pub nome: String,
    pub classificacao_risco: String,
    pub toxicidade: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArestaPsicossocial {
    pub vinculo_id: i64,
    pub origem_id: i64,
    pub destino_id: i64,
    pub tipo_vinculo: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContribuicaoHp {
    pub fonte_id: i64,
    pub fonte_nome: String,
    pub alvo_direto_id: i64,
    pub alvo_direto_nome: String,
    pub vinculo_id: i64,
    pub tipo_vinculo: String,
    pub toxicidade_fonte: f64,
    pub peso: f64,
    pub grau: u8,
    pub penalidade: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndicadoresPsicossociais {
    pub hp: f64,
    pub hp_percentual: f64,
    pub hp_base: f64,
    pub hp_min: f64,
    pub penalidade_direta: f64,
    pub penalidade_propria: f64,
    pub penalidade_residual: f64,
    pub aura_nome: String,
    pub aura_cor_hex: String,
    pub aura_pulsante: bool,
    pub vitalidade_nome: String,
    pub vitalidade_cor_hex: String,
    pub calculo_ativo: bool,
    pub configuracao_versao: i64,
    pub contribuicoes: Vec<ContribuicaoHp>,
}

#[derive(Debug, Clone)]
pub struct AtualizacaoHp {
    pub pessoa_id: i64,
    pub indicadores: IndicadoresPsicossociais,
}

#[derive(Debug, thiserror::Error)]
pub enum ErroHp {
    #[error("configuração inválida: {0}")]
    ConfiguracaoInvalida(String),
    #[error("perfil #{pessoa_id} inválido: {motivo}")]
    PerfilInvalido { pessoa_id: i64, motivo: String },
    #[error("grafo inválido: {0}")]
    GrafoInvalido(String),
}

pub fn normalizar_tipo(tipo: &str) -> String {
    tipo.trim().to_lowercase()
}

impl ParametrosHp {
    pub fn validar_e_normalizar(&mut self) -> Result<(), ErroHp> {
        let invalida = |msg: &str| ErroHp::ConfiguracaoInvalida(msg.into());
        let fracao = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        if !fracao(self.toxicidade_min)
            || self.toxicidade_min == 0.0
            || !fracao(self.toxicidade_max)
            || self.toxicidade_min > self.toxicidade_max
        {
            return Err(invalida(
                "limites de intensidade devem obedecer 0 < mínimo <= máximo <= 1",
            ));
        }
        if !fracao(self.hp_min)
            || self.hp_min == 0.0
            || !fracao(self.hp_base)
            || self.hp_min > self.hp_base
        {
            return Err(invalida("HP deve obedecer 0 < piso <= base <= 1"));
        }
        if !fracao(self.fator_segundo_grau) || !fracao(self.peso_padrao) {
            return Err(invalida(
                "fator residual e peso padrão devem estar entre 0 e 1",
            ));
        }
        let mut pesos = BTreeMap::new();
        for (tipo, peso) in &self.pesos_vinculo {
            let chave = normalizar_tipo(tipo);
            if chave.is_empty()
                || chave.chars().count() > 120
                || !fracao(*peso)
                || pesos.insert(chave, *peso).is_some()
            {
                return Err(invalida(
                    "tipos/pesos inválidos ou repetidos após normalização",
                ));
            }
        }
        self.pesos_vinculo = pesos;
        for faixas in [&mut self.faixas_aura, &mut self.faixas_vitalidade] {
            if faixas.is_empty() || faixas.len() > 32 || faixas[0].min != 0.0 {
                return Err(invalida(
                    "cada indicador deve começar em zero e ter de 1 a 32 faixas",
                ));
            }
            let mut anterior = -1.0;
            for faixa in faixas {
                if !fracao(faixa.min)
                    || faixa.min <= anterior
                    || faixa.nome.trim().is_empty()
                    || faixa.nome.chars().count() > 80
                    || faixa.cor_hex.len() != 7
                    || !faixa.cor_hex.starts_with('#')
                    || !faixa.cor_hex[1..].bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(invalida(
                        "faixas devem ter limiares crescentes, nome e cor #RRGGBB",
                    ));
                }
                anterior = faixa.min;
                faixa.nome = faixa.nome.trim().into();
                faixa.cor_hex = faixa.cor_hex.to_uppercase();
            }
        }
        Ok(())
    }
}

pub fn validar_perfil(
    id: i64,
    classificacao: &str,
    t: f64,
    p: &ParametrosHp,
) -> Result<(), ErroHp> {
    let valido = t.is_finite()
        && match classificacao {
            "NAO_CLASSIFICADO" | "SEM_RISCO" => t == 0.0,
            "MANIPULATIVO" | "PATOLOGICO" | "MISTO" => {
                t >= p.toxicidade_min && t <= p.toxicidade_max
            }
            _ => false,
        };
    if !valido {
        return Err(ErroHp::PerfilInvalido {
            pessoa_id: id,
            motivo: format!(
                "classificação/intensidade incompatível; risco ativo exige intensidade entre {} e {}",
                p.toxicidade_min, p.toxicidade_max
            ),
        });
    }
    Ok(())
}

fn selecionar_faixa(faixas: &[FaixaIndicador], valor: f64) -> &FaixaIndicador {
    faixas
        .iter()
        .rev()
        .find(|f| valor >= f.min)
        .unwrap_or(&faixas[0])
}

/// Recalcula desde o HP base, descontando T próprio e exposições recebidas.
pub fn calcular_hp_psicossocial(
    nos: &[NoPsicossocial],
    arestas: &[ArestaPsicossocial],
    config: &ConfigHpPsicossocial,
) -> Result<Vec<AtualizacaoHp>, ErroHp> {
    let mut p = config.parametros.clone();
    p.validar_e_normalizar()?;
    if config.versao < 1 {
        return Err(ErroHp::ConfiguracaoInvalida(
            "versão deve ser positiva".into(),
        ));
    }
    let mut por_id = BTreeMap::new();
    for no in nos {
        validar_perfil(no.pessoa_id, &no.classificacao_risco, no.toxicidade, &p)?;
        if no.pessoa_id <= 0 || por_id.insert(no.pessoa_id, no).is_some() {
            return Err(ErroHp::GrafoInvalido(
                "IDs de pessoa inválidos ou repetidos".into(),
            ));
        }
    }
    let mut vizinhos: BTreeMap<i64, BTreeSet<i64>> =
        por_id.keys().map(|id| (*id, BTreeSet::new())).collect();
    let mut ordenadas = BTreeMap::new();
    let mut unicas = BTreeSet::new();
    for e in arestas {
        if e.vinculo_id <= 0
            || e.origem_id == e.destino_id
            || e.tipo_vinculo.trim().is_empty()
            || !por_id.contains_key(&e.origem_id)
            || !por_id.contains_key(&e.destino_id)
            || ordenadas.insert(e.vinculo_id, e).is_some()
            || !unicas.insert((e.origem_id, e.destino_id, &e.tipo_vinculo))
        {
            return Err(ErroHp::GrafoInvalido(
                "aresta inválida, duplicada ou com ponta inexistente".into(),
            ));
        }
        vizinhos.get_mut(&e.origem_id).unwrap().insert(e.destino_id);
        vizinhos.get_mut(&e.destino_id).unwrap().insert(e.origem_id);
    }
    let mut recebidas: BTreeMap<i64, Vec<ContribuicaoHp>> =
        por_id.keys().map(|id| (*id, Vec::new())).collect();
    if p.ativo {
        for e in ordenadas.values() {
            let a = por_id[&e.origem_id];
            let b = por_id[&e.destino_id];
            let peso = *p
                .pesos_vinculo
                .get(&normalizar_tipo(&e.tipo_vinculo))
                .unwrap_or(&p.peso_padrao);
            let penalidade = a.toxicidade * peso;
            if penalidade == 0.0 {
                continue;
            }
            let contribuicao = ContribuicaoHp {
                fonte_id: a.pessoa_id,
                fonte_nome: a.nome.clone(),
                alvo_direto_id: b.pessoa_id,
                alvo_direto_nome: b.nome.clone(),
                vinculo_id: e.vinculo_id,
                tipo_vinculo: e.tipo_vinculo.clone(),
                toxicidade_fonte: a.toxicidade,
                peso,
                grau: 1,
                penalidade,
            };
            recebidas
                .get_mut(&b.pessoa_id)
                .unwrap()
                .push(contribuicao.clone());
            if p.fator_segundo_grau == 0.0 {
                continue;
            }
            for c in &vizinhos[&b.pessoa_id] {
                if *c == a.pessoa_id || *c == b.pessoa_id || vizinhos[&a.pessoa_id].contains(c) {
                    continue;
                }
                let mut residual = contribuicao.clone();
                residual.grau = 2;
                residual.penalidade *= p.fator_segundo_grau;
                recebidas.get_mut(c).unwrap().push(residual);
            }
        }
    }
    Ok(por_id
        .into_iter()
        .map(|(id, no)| {
            let contribuicoes = recebidas.remove(&id).unwrap();
            let direta: f64 = contribuicoes
                .iter()
                .filter(|c| c.grau == 1)
                .map(|c| c.penalidade)
                .sum();
            let residual: f64 = contribuicoes
                .iter()
                .filter(|c| c.grau == 2)
                .map(|c| c.penalidade)
                .sum();
            let propria = if p.ativo { no.toxicidade } else { 0.0 };
            let hp = (p.hp_base - propria - direta - residual).clamp(p.hp_min, p.hp_base);
            let aura = selecionar_faixa(&p.faixas_aura, no.toxicidade);
            let vitalidade = selecionar_faixa(&p.faixas_vitalidade, hp);
            AtualizacaoHp {
                pessoa_id: id,
                indicadores: IndicadoresPsicossociais {
                    hp,
                    hp_percentual: hp * 100.0,
                    hp_base: p.hp_base,
                    hp_min: p.hp_min,
                    penalidade_direta: direta,
                    penalidade_propria: propria,
                    penalidade_residual: residual,
                    aura_nome: aura.nome.clone(),
                    aura_cor_hex: aura.cor_hex.clone(),
                    aura_pulsante: aura.pulsante,
                    vitalidade_nome: vitalidade.nome.clone(),
                    vitalidade_cor_hex: vitalidade.cor_hex.clone(),
                    calculo_ativo: p.ativo,
                    configuracao_versao: config.versao,
                    contribuicoes,
                },
            }
        })
        .collect())
}
