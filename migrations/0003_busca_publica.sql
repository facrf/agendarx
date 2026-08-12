PRAGMA foreign_keys = ON;

CREATE TABLE parametro_busca (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (
        tipo IN ('NOME', 'CPF', 'CNPJ', 'EMAIL', 'TELEFONE', 'TERMO')
    ),
    valor TEXT NOT NULL CHECK (length(trim(valor)) > 0),
    ativo INTEGER NOT NULL DEFAULT 1 CHECK (ativo IN (0, 1)),
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    UNIQUE (pessoa_id, tipo, valor)
);

CREATE TABLE historico_busca_publica (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL,
    fonte TEXT NOT NULL,
    parametro_utilizado TEXT NOT NULL,
    titulo_resultado TEXT NOT NULL,
    snippet TEXT,
    url_origem TEXT NOT NULL,
    anexo_dossie_id INTEGER,
    data_captura DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (anexo_dossie_id) REFERENCES anexo_dossie(id)
        ON UPDATE CASCADE ON DELETE SET NULL,
    UNIQUE (pessoa_id, url_origem)
);

CREATE INDEX idx_parametro_busca_pessoa_ativo
    ON parametro_busca(pessoa_id, ativo);
CREATE INDEX idx_historico_busca_pessoa_data
    ON historico_busca_publica(pessoa_id, data_captura DESC);
CREATE INDEX idx_historico_busca_anexo
    ON historico_busca_publica(anexo_dossie_id);
