CREATE TABLE parametro_busca_novo (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (tipo IN ('NOME', 'CPF', 'CNPJ', 'EMAIL', 'TELEFONE', 'TERMO', 'PROCESSO')),
    valor TEXT NOT NULL CHECK (length(trim(valor)) > 0),
    provider TEXT NOT NULL DEFAULT 'SEARXNG'
        CHECK (provider IN ('SEARXNG', 'QUERIDO_DIARIO', 'INLABS', 'OPENALEX', 'DATAJUD')),
    ativo INTEGER NOT NULL DEFAULT 1 CHECK (ativo IN (0, 1)),
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    UNIQUE (pessoa_id, tipo, valor, provider)
);

INSERT INTO parametro_busca_novo SELECT * FROM parametro_busca;

DROP TABLE parametro_busca;

ALTER TABLE parametro_busca_novo RENAME TO parametro_busca;

CREATE INDEX idx_parametro_busca_pessoa_ativo
    ON parametro_busca(pessoa_id, ativo);

CREATE TABLE historico_busca_publica_novo (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL,
    fonte TEXT NOT NULL,
    parametro_utilizado TEXT NOT NULL,
    titulo_resultado TEXT NOT NULL,
    snippet TEXT,
    url_origem TEXT NOT NULL,
    anexo_dossie_id INTEGER,
    data_captura DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, provider TEXT NOT NULL DEFAULT 'SEARXNG'
        CHECK (provider IN ('SEARXNG', 'QUERIDO_DIARIO', 'INLABS', 'OPENALEX', 'DATAJUD')), data_publicacao TEXT, detalhes TEXT,
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (anexo_dossie_id) REFERENCES anexo_dossie(id)
        ON UPDATE CASCADE ON DELETE SET NULL,
    UNIQUE (pessoa_id, url_origem)
);

INSERT INTO historico_busca_publica_novo SELECT * FROM historico_busca_publica;

DROP TABLE historico_busca_publica;

ALTER TABLE historico_busca_publica_novo RENAME TO historico_busca_publica;

CREATE INDEX idx_historico_busca_pessoa_data
    ON historico_busca_publica(pessoa_id, data_captura DESC);

CREATE INDEX idx_historico_busca_anexo
    ON historico_busca_publica(anexo_dossie_id);
