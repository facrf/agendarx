CREATE TABLE osint_trabalho (
    id TEXT PRIMARY KEY,
    usuario_id INTEGER NOT NULL REFERENCES usuario(id) ON DELETE CASCADE,
    pessoa_id INTEGER NOT NULL REFERENCES pessoa(id) ON DELETE CASCADE,
    estado TEXT NOT NULL CHECK (estado IN ('fila','executando','concluido','cancelado','interrompido','erro')),
    parametros TEXT NOT NULL,
    total INTEGER NOT NULL,
    processados INTEGER NOT NULL DEFAULT 0,
    resultado TEXT NOT NULL,
    erro TEXT,
    tentativa TEXT,
    criado_em TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    atualizado_em TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX osint_trabalho_ativo ON osint_trabalho(usuario_id,pessoa_id) WHERE estado IN ('fila','executando');
CREATE INDEX osint_trabalho_fila ON osint_trabalho(estado,criado_em);
