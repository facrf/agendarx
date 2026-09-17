use super::hp_psicossocial::*;

fn no(id: i64, t: f64) -> NoPsicossocial {
    NoPsicossocial {
        pessoa_id: id,
        nome: format!("Pessoa {id}"),
        classificacao_risco: if t == 0.0 {
            "NAO_CLASSIFICADO"
        } else {
            "MANIPULATIVO"
        }
        .into(),
        toxicidade: t,
    }
}
fn aresta(id: i64, a: i64, b: i64, tipo: &str) -> ArestaPsicossocial {
    ArestaPsicossocial {
        vinculo_id: id,
        origem_id: a,
        destino_id: b,
        tipo_vinculo: tipo.into(),
    }
}
fn config() -> ConfigHpPsicossocial {
    ConfigHpPsicossocial {
        versao: 1,
        parametros: ParametrosHp::default(),
    }
}
fn estado(v: &[AtualizacaoHp], id: i64) -> &IndicadoresPsicossociais {
    &v.iter().find(|a| a.pessoa_id == id).unwrap().indicadores
}
fn perto(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn acumula_fontes_limita_hp_e_propaga_penalidade_original() {
    let nos = [no(1, 0.5), no(2, 0.0), no(3, 0.0), no(4, 0.5)];
    let mut edges = vec![
        aresta(1, 1, 2, "Família"),
        aresta(2, 4, 2, "Família"),
        aresta(3, 2, 3, "Profissional"),
    ];
    let v = calcular_hp_psicossocial(&nos, &edges, &config()).unwrap();
    perto(estado(&v, 2).hp, 0.05);
    perto(estado(&v, 2).penalidade_direta, 1.0);
    perto(estado(&v, 3).hp, 0.8);
    assert_eq!(estado(&v, 2).aura_nome, "Estável");
    assert_eq!(estado(&v, 2).vitalidade_nome, "Impactada");
    assert_eq!(estado(&v, 1).aura_nome, "Crítico");
    perto(estado(&v, 1).hp, 0.9);
    edges.remove(1);
    let restored = calcular_hp_psicossocial(&nos, &edges, &config()).unwrap();
    perto(estado(&restored, 2).hp, 0.5);
    perto(estado(&restored, 3).hp, 0.9);
}

#[test]
fn residual_usa_vizinhanca_nos_dois_sentidos_e_nao_duplica_vinculos() {
    let nos = [no(1, 0.3), no(2, 0.0), no(3, 0.0), no(4, 0.0)];
    let edges = [
        aresta(1, 1, 2, " família "),
        aresta(2, 3, 2, "Amizade"),
        aresta(3, 2, 3, "Profissional"),
        aresta(4, 3, 4, "Família"),
    ];
    let v = calcular_hp_psicossocial(&nos, &edges, &config()).unwrap();
    perto(estado(&v, 2).hp, 0.7);
    perto(estado(&v, 3).hp, 0.94);
    perto(estado(&v, 4).hp, 1.0);
    assert_eq!(estado(&v, 3).contribuicoes.len(), 1);
    assert_eq!(estado(&v, 3).contribuicoes[0].grau, 2);
}

#[test]
fn triangulos_nao_devolvem_residual_nem_somam_residual_a_alvo_direto() {
    let nos = [no(1, 0.3), no(2, 0.0), no(3, 0.0)];
    let edges = [
        aresta(1, 1, 2, "Família"),
        aresta(2, 2, 3, "Família"),
        aresta(3, 1, 3, "Profissional"),
    ];
    let v = calcular_hp_psicossocial(&nos, &edges, &config()).unwrap();
    perto(estado(&v, 1).hp, 1.0);
    perto(estado(&v, 3).hp, 0.85);
    perto(estado(&v, 3).penalidade_residual, 0.0);
    let cycle = [
        edges[0].clone(),
        edges[1].clone(),
        aresta(3, 3, 1, "Família"),
    ];
    let v = calcular_hp_psicossocial(&nos, &cycle, &config()).unwrap();
    perto(estado(&v, 3).hp, 1.0);
}

#[test]
fn caminhos_e_tipos_distintos_acumulam_e_ordem_nao_altera_resultado() {
    let mut nos = vec![no(1, 0.3), no(2, 0.0), no(3, 0.0), no(4, 0.0)];
    let mut edges = vec![
        aresta(1, 1, 2, "Família"),
        aresta(2, 1, 2, "Profissional"),
        aresta(3, 1, 4, "Profissional"),
        aresta(4, 2, 3, "Amizade"),
        aresta(5, 4, 3, "Amizade"),
    ];
    let first = calcular_hp_psicossocial(&nos, &edges, &config()).unwrap();
    perto(estado(&first, 3).penalidade_residual, 0.12);
    nos.reverse();
    edges.reverse();
    let second = calcular_hp_psicossocial(&nos, &edges, &config()).unwrap();
    assert_eq!(
        serde_json::to_value(estado(&first, 3)).unwrap(),
        serde_json::to_value(estado(&second, 3)).unwrap()
    );
}

#[test]
fn parametros_desativacao_e_cores_sao_independentes() {
    let nos = [no(1, 0.4), no(2, 0.0), no(3, 0.0)];
    let edges = [aresta(1, 1, 2, "Desconhecido"), aresta(2, 2, 3, "Família")];
    let mut cfg = config();
    cfg.parametros.peso_padrao = 0.25;
    cfg.parametros.fator_segundo_grau = 0.0;
    let v = calcular_hp_psicossocial(&nos, &edges, &cfg).unwrap();
    perto(estado(&v, 2).hp, 0.9);
    perto(estado(&v, 3).hp, 1.0);
    cfg.parametros.ativo = false;
    let v = calcular_hp_psicossocial(&nos, &edges, &cfg).unwrap();
    perto(estado(&v, 2).hp, 1.0);
    assert!(estado(&v, 2).contribuicoes.is_empty());
    assert_eq!(estado(&v, 1).aura_nome, "Crítico");
    assert!(estado(&v, 1).aura_pulsante);
}

#[test]
fn valida_limites_grafo_e_faixas() {
    let mut cfg = config();
    for invalid in [f64::NAN, f64::INFINITY, -0.1, 50.0] {
        cfg.parametros.peso_padrao = invalid;
        assert!(calcular_hp_psicossocial(&[], &[], &cfg).is_err());
    }
    let cfg = config();
    for t in [0.09, 0.51, f64::NAN] {
        assert!(calcular_hp_psicossocial(&[no(1, t)], &[], &cfg).is_err());
    }
    for t in [0.10, 0.50] {
        assert!(calcular_hp_psicossocial(&[no(1, t)], &[], &cfg).is_ok());
    }
    assert!(calcular_hp_psicossocial(&[no(1, 0.0), no(1, 0.0)], &[], &cfg).is_err());
    assert!(calcular_hp_psicossocial(&[no(1, 0.0)], &[aresta(1, 1, 2, "Família")], &cfg).is_err());
    assert!(calcular_hp_psicossocial(&[no(1, 0.0)], &[aresta(1, 1, 1, "Família")], &cfg).is_err());
    let mut cfg = cfg;
    cfg.parametros.faixas_aura[1].min = 0.0;
    assert!(calcular_hp_psicossocial(&[], &[], &cfg).is_err());
}
