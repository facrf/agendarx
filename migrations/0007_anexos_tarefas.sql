PRAGMA foreign_keys = ON;

CREATE TABLE anexo_tarefa_calendario (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tarefa_id INTEGER NOT NULL,
    nome_arquivo TEXT NOT NULL CHECK (
        length(trim(nome_arquivo)) BETWEEN 1 AND 255
    ),
    mime_type TEXT NOT NULL,
    conteudo_blob BLOB NOT NULL,
    tamanho_bytes INTEGER NOT NULL CHECK (tamanho_bytes > 0),
    data_upload DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tarefa_id) REFERENCES tarefa_calendario(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE INDEX idx_anexo_tarefa_calendario_tarefa
    ON anexo_tarefa_calendario(tarefa_id, data_upload DESC, id DESC);

CREATE TABLE miniatura_anexo_tarefa_calendario (
    anexo_id INTEGER PRIMARY KEY,
    conteudo_webp BLOB NOT NULL,
    largura INTEGER NOT NULL CHECK (largura > 0),
    altura INTEGER NOT NULL CHECK (altura > 0),
    tamanho_bytes INTEGER NOT NULL CHECK (tamanho_bytes > 0),
    data_geracao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (anexo_id) REFERENCES anexo_tarefa_calendario(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);
