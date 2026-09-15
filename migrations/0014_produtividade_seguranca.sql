-- Organização, recuperação e rastreabilidade.
ALTER TABLE pessoa ADD COLUMN excluida_em DATETIME;

CREATE TABLE etiqueta (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nome TEXT NOT NULL COLLATE NOCASE UNIQUE,
    cor_hex TEXT NOT NULL DEFAULT '#64748B' CHECK (
        length(cor_hex) = 7 AND substr(cor_hex, 1, 1) = '#'
    ),
    data_criacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE pessoa_etiqueta (
    pessoa_id INTEGER NOT NULL,
    etiqueta_id INTEGER NOT NULL,
    PRIMARY KEY (pessoa_id, etiqueta_id),
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id) ON DELETE CASCADE,
    FOREIGN KEY (etiqueta_id) REFERENCES etiqueta(id) ON DELETE CASCADE
);

CREATE TABLE pessoa_favorita (
    usuario_id INTEGER NOT NULL,
    pessoa_id INTEGER NOT NULL,
    data_criacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (usuario_id, pessoa_id),
    FOREIGN KEY (usuario_id) REFERENCES usuario(id) ON DELETE CASCADE,
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id) ON DELETE CASCADE
);

CREATE TABLE auditoria (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario_id INTEGER,
    usuario_login TEXT NOT NULL,
    acao TEXT NOT NULL,
    recurso TEXT NOT NULL,
    status_http INTEGER NOT NULL,
    data_evento DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (usuario_id) REFERENCES usuario(id) ON DELETE SET NULL
);

CREATE TABLE posicao_grafo (
    usuario_id INTEGER NOT NULL,
    layout TEXT NOT NULL CHECK (layout IN ('force', 'hierarchical')),
    pessoa_id INTEGER NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    data_atualizacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (usuario_id, layout, pessoa_id),
    FOREIGN KEY (usuario_id) REFERENCES usuario(id) ON DELETE CASCADE,
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id) ON DELETE CASCADE
);

CREATE TABLE backup_registro (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nome_arquivo TEXT NOT NULL,
    tamanho_bytes INTEGER NOT NULL,
    automatico INTEGER NOT NULL DEFAULT 0 CHECK (automatico IN (0, 1)),
    data_criacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_pessoa_ativa_nome ON pessoa(excluida_em, nome);
CREATE INDEX idx_pessoa_etiqueta_etiqueta ON pessoa_etiqueta(etiqueta_id);
CREATE INDEX idx_auditoria_data ON auditoria(data_evento DESC);
CREATE INDEX idx_vinculo_data ON pessoa_vinculo(data_criacao);
