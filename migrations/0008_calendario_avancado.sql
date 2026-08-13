PRAGMA foreign_keys = ON;

ALTER TABLE tarefa_calendario ADD COLUMN serie_id TEXT;
ALTER TABLE tarefa_calendario ADD COLUMN recorrencia TEXT NOT NULL DEFAULT 'NENHUMA'
    CHECK (recorrencia IN ('NENHUMA', 'DIARIA', 'SEMANAL', 'MENSAL'));
ALTER TABLE tarefa_calendario ADD COLUMN recorrencia_fim_em TEXT;
ALTER TABLE tarefa_calendario ADD COLUMN lembrete_minutos INTEGER
    CHECK (lembrete_minutos IS NULL OR lembrete_minutos BETWEEN 0 AND 525600);
ALTER TABLE tarefa_calendario ADD COLUMN lembrete_dispensado_em TEXT;

CREATE INDEX idx_tarefa_calendario_serie
    ON tarefa_calendario(usuario_id, serie_id, inicio_em);
CREATE INDEX idx_tarefa_calendario_lembrete
    ON tarefa_calendario(usuario_id, lembrete_dispensado_em, inicio_em)
    WHERE lembrete_minutos IS NOT NULL AND status <> 'CONCLUIDA';

CREATE TABLE historico_tarefa_calendario (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tarefa_id INTEGER NOT NULL,
    tipo TEXT NOT NULL CHECK (
        tipo IN (
            'CRIADA', 'ATUALIZADA', 'MOVIDA', 'STATUS_ALTERADO',
            'ANEXO_ADICIONADO', 'ANEXO_EXCLUIDO'
        )
    ),
    descricao TEXT NOT NULL,
    data_evento DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tarefa_id) REFERENCES tarefa_calendario(id)
        ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE INDEX idx_historico_tarefa_calendario
    ON historico_tarefa_calendario(tarefa_id, data_evento DESC, id DESC);
