ALTER TABLE parametro_busca RENAME TO parametro_busca_legado;

CREATE TABLE parametro_busca (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (tipo IN ('NOME', 'CPF', 'CNPJ', 'EMAIL', 'TELEFONE', 'TERMO')),
    valor TEXT NOT NULL CHECK (length(trim(valor)) > 0),
    provider TEXT NOT NULL DEFAULT 'SEARXNG'
        CHECK (provider IN ('SEARXNG', 'QUERIDO_DIARIO', 'INLABS', 'OPENALEX')),
    ativo INTEGER NOT NULL DEFAULT 1 CHECK (ativo IN (0, 1)),
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    UNIQUE (pessoa_id, tipo, valor, provider)
);

INSERT INTO parametro_busca (id, pessoa_id, tipo, valor, provider, ativo)
SELECT id, pessoa_id, tipo, valor, 'SEARXNG', ativo
FROM parametro_busca_legado;

DROP TABLE parametro_busca_legado;

CREATE INDEX idx_parametro_busca_pessoa_ativo
    ON parametro_busca(pessoa_id, ativo);

ALTER TABLE historico_busca_publica
    ADD COLUMN provider TEXT NOT NULL DEFAULT 'SEARXNG'
        CHECK (provider IN ('SEARXNG', 'QUERIDO_DIARIO', 'INLABS', 'OPENALEX'));

ALTER TABLE historico_busca_publica
    ADD COLUMN data_publicacao TEXT;

ALTER TABLE historico_busca_publica
    ADD COLUMN detalhes TEXT;
