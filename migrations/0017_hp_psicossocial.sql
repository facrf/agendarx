ALTER TABLE pessoa ADD COLUMN classificacao_risco TEXT NOT NULL DEFAULT 'NAO_CLASSIFICADO'
    CHECK (classificacao_risco IN ('NAO_CLASSIFICADO', 'SEM_RISCO', 'MANIPULATIVO', 'PATOLOGICO', 'MISTO'));
ALTER TABLE pessoa ADD COLUMN toxicidade REAL NOT NULL DEFAULT 0
    CHECK (toxicidade >= 0 AND toxicidade <= 1 AND (
        (classificacao_risco IN ('NAO_CLASSIFICADO', 'SEM_RISCO') AND toxicidade = 0)
        OR (classificacao_risco IN ('MANIPULATIVO', 'PATOLOGICO', 'MISTO') AND toxicidade > 0)
    ));

CREATE TABLE hp_psicossocial_configuracao (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    versao INTEGER NOT NULL DEFAULT 1 CHECK (versao > 0),
    parametros_json TEXT NOT NULL,
    atualizado_em TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    atualizado_por INTEGER REFERENCES usuario(id) ON DELETE SET NULL
);

INSERT INTO hp_psicossocial_configuracao (id, parametros_json) VALUES (1, '{
    "ativo":true,"toxicidade_min":0.10,"toxicidade_max":0.50,
    "hp_base":1.0,"hp_min":0.05,"fator_segundo_grau":0.20,
    "pesos_vinculo":{"família":1.0,"familia":1.0,"profissional":0.50},"peso_padrao":0.50,
    "faixas_aura":[
        {"min":0.0,"nome":"Estável","cor_hex":"#22C55E","pulsante":false},
        {"min":0.10,"nome":"Elevado","cor_hex":"#EAB308","pulsante":false},
        {"min":0.25,"nome":"Observação","cor_hex":"#F97316","pulsante":false},
        {"min":0.40,"nome":"Crítico","cor_hex":"#EF4444","pulsante":true}
    ],
    "faixas_vitalidade":[
        {"min":0.0,"nome":"Impactada","cor_hex":"#EF4444","pulsante":false},
        {"min":0.25,"nome":"Baixa","cor_hex":"#F97316","pulsante":false},
        {"min":0.50,"nome":"Atenção","cor_hex":"#EAB308","pulsante":false},
        {"min":0.75,"nome":"Preservada","cor_hex":"#22C55E","pulsante":false}
    ]
}');
