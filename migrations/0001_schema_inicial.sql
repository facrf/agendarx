PRAGMA foreign_keys = ON;

CREATE TABLE usuario (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    login TEXT NOT NULL UNIQUE,
    senha_hash TEXT NOT NULL,
    data_criacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE categoria_pessoa (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nome_categoria TEXT NOT NULL,
    cor_hex TEXT NOT NULL CHECK (
        length(cor_hex) = 7 AND substr(cor_hex, 1, 1) = '#'
    )
);

CREATE TABLE tipo_meio_contato (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nome_tipo TEXT NOT NULL
);

CREATE TABLE pessoa (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nome TEXT NOT NULL,
    categoria_id INTEGER,
    foto_principal BLOB,
    data_cadastro DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (categoria_id) REFERENCES categoria_pessoa(id)
        ON UPDATE CASCADE ON DELETE SET NULL
);

CREATE TABLE contato (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL,
    tipo_contato_id INTEGER NOT NULL,
    valor TEXT NOT NULL,
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (tipo_contato_id) REFERENCES tipo_meio_contato(id)
        ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE TABLE anexo_dossie (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL,
    nome_arquivo TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    conteudo_blob BLOB NOT NULL,
    tamanho_bytes INTEGER NOT NULL CHECK (tamanho_bytes >= 0),
    data_upload DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE pessoa_vinculo (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_origem_id INTEGER NOT NULL,
    pessoa_destino_id INTEGER NOT NULL,
    tipo_vinculo TEXT NOT NULL,
    descricao TEXT,
    data_criacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pessoa_origem_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (pessoa_destino_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    CHECK (pessoa_origem_id <> pessoa_destino_id),
    UNIQUE (pessoa_origem_id, pessoa_destino_id, tipo_vinculo)
);

-- O JWT identifica esta sessão. Excluí-la revoga o token imediatamente.
CREATE TABLE sessao (
    id TEXT PRIMARY KEY,
    usuario_id INTEGER NOT NULL,
    expira_em INTEGER NOT NULL,
    data_criacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (usuario_id) REFERENCES usuario(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE INDEX idx_pessoa_nome ON pessoa(nome);
CREATE INDEX idx_pessoa_categoria ON pessoa(categoria_id);
CREATE INDEX idx_contato_pessoa ON contato(pessoa_id);
CREATE INDEX idx_anexo_pessoa ON anexo_dossie(pessoa_id);
CREATE INDEX idx_vinculo_origem ON pessoa_vinculo(pessoa_origem_id);
CREATE INDEX idx_vinculo_destino ON pessoa_vinculo(pessoa_destino_id);
CREATE INDEX idx_sessao_usuario ON sessao(usuario_id);
CREATE INDEX idx_sessao_expiracao ON sessao(expira_em);

INSERT INTO categoria_pessoa (nome_categoria, cor_hex) VALUES
    ('Família', '#EF4444'),
    ('Amigos', '#3B82F6'),
    ('Profissional', '#10B981');

INSERT INTO tipo_meio_contato (nome_tipo) VALUES
    ('WhatsApp'),
    ('Nostr'),
    ('E-mail'),
    ('Telefone');

