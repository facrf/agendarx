ALTER TABLE pessoa ADD COLUMN risco_justificativa TEXT NOT NULL DEFAULT '';
ALTER TABLE pessoa ADD COLUMN risco_revisado_em TEXT;

CREATE TABLE pessoa_risco_historico (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pessoa_id INTEGER NOT NULL REFERENCES pessoa(id) ON DELETE CASCADE,
    autor_id INTEGER REFERENCES usuario(id) ON DELETE SET NULL,
    autor_login TEXT NOT NULL,
    registrado_em TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    anterior_json TEXT,
    novo_json TEXT NOT NULL
);
CREATE INDEX idx_risco_historico_pessoa ON pessoa_risco_historico(pessoa_id, id DESC);
