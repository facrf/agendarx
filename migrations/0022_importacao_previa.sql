CREATE TABLE importacao_previa (
    token TEXT PRIMARY KEY,
    usuario_id INTEGER NOT NULL REFERENCES usuario(id) ON DELETE CASCADE,
    conteudo TEXT NOT NULL,
    expira_em INTEGER NOT NULL
);
CREATE INDEX importacao_previa_expiracao ON importacao_previa(expira_em);
