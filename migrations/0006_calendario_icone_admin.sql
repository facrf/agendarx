PRAGMA foreign_keys = ON;

ALTER TABLE usuario ADD COLUMN icone_admin_blob BLOB;
ALTER TABLE usuario ADD COLUMN icone_admin_mime_type TEXT;
ALTER TABLE usuario ADD COLUMN icone_admin_atualizado_em DATETIME;

CREATE TABLE tarefa_calendario (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario_id INTEGER NOT NULL,
    titulo TEXT NOT NULL CHECK (
        length(trim(titulo)) BETWEEN 1 AND 160
    ),
    descricao TEXT CHECK (
        descricao IS NULL OR length(descricao) <= 5000
    ),
    inicio_em TEXT NOT NULL,
    fim_em TEXT,
    dia_inteiro INTEGER NOT NULL DEFAULT 0 CHECK (dia_inteiro IN (0, 1)),
    status TEXT NOT NULL DEFAULT 'PENDENTE' CHECK (
        status IN ('PENDENTE', 'EM_ANDAMENTO', 'CONCLUIDA')
    ),
    prioridade TEXT NOT NULL DEFAULT 'NORMAL' CHECK (
        prioridade IN ('BAIXA', 'NORMAL', 'ALTA')
    ),
    cor_hex TEXT NOT NULL DEFAULT '#13716D' CHECK (
        length(cor_hex) = 7 AND substr(cor_hex, 1, 1) = '#'
    ),
    data_criacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    data_atualizacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (usuario_id) REFERENCES usuario(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE tarefa_calendario_pessoa (
    tarefa_id INTEGER NOT NULL,
    pessoa_id INTEGER NOT NULL,
    PRIMARY KEY (tarefa_id, pessoa_id),
    FOREIGN KEY (tarefa_id) REFERENCES tarefa_calendario(id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (pessoa_id) REFERENCES pessoa(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE INDEX idx_tarefa_calendario_usuario_inicio
    ON tarefa_calendario(usuario_id, inicio_em);
CREATE INDEX idx_tarefa_calendario_usuario_status
    ON tarefa_calendario(usuario_id, status);
CREATE INDEX idx_tarefa_calendario_pessoa
    ON tarefa_calendario_pessoa(pessoa_id, tarefa_id);
