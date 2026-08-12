PRAGMA foreign_keys = ON;

-- As miniaturas ficam separadas dos anexos originais para que as listagens não
-- carreguem os BLOBs e para permitir recriar o cache sem tocar no arquivo fonte.
CREATE TABLE miniatura_anexo_dossie (
    anexo_id INTEGER PRIMARY KEY,
    conteudo_webp BLOB NOT NULL,
    largura INTEGER NOT NULL CHECK (largura > 0),
    altura INTEGER NOT NULL CHECK (altura > 0),
    tamanho_bytes INTEGER NOT NULL CHECK (tamanho_bytes > 0),
    data_geracao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (anexo_id) REFERENCES anexo_dossie(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE miniatura_anexo_vinculo (
    anexo_id INTEGER PRIMARY KEY,
    conteudo_webp BLOB NOT NULL,
    largura INTEGER NOT NULL CHECK (largura > 0),
    altura INTEGER NOT NULL CHECK (altura > 0),
    tamanho_bytes INTEGER NOT NULL CHECK (tamanho_bytes > 0),
    data_geracao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (anexo_id) REFERENCES anexo_vinculo(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);
