PRAGMA foreign_keys = ON;

CREATE TABLE anexo_vinculo (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vinculo_id INTEGER NOT NULL,
    nome_arquivo TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    conteudo_blob BLOB NOT NULL,
    tamanho_bytes INTEGER NOT NULL CHECK (tamanho_bytes >= 0),
    data_upload DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (vinculo_id) REFERENCES pessoa_vinculo(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE INDEX idx_anexo_vinculo_vinculo
    ON anexo_vinculo(vinculo_id, data_upload DESC);

CREATE TABLE identidade_visual (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    icone_blob BLOB NOT NULL,
    mime_type TEXT NOT NULL,
    atualizado_em DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
