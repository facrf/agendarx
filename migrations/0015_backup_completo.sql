-- Política configurável, integridade e compatibilidade dos backups.
ALTER TABLE backup_registro ADD COLUMN tipo TEXT NOT NULL DEFAULT 'manual'
    CHECK (tipo IN ('manual', 'automatico', 'seguranca'));
ALTER TABLE backup_registro ADD COLUMN sha256 TEXT NOT NULL DEFAULT '';
ALTER TABLE backup_registro ADD COLUMN integridade_ok INTEGER NOT NULL DEFAULT 0
    CHECK (integridade_ok IN (0, 1));
ALTER TABLE backup_registro ADD COLUMN versao_app TEXT;
ALTER TABLE backup_registro ADD COLUMN schema_versao INTEGER;

UPDATE backup_registro
SET tipo = CASE WHEN automatico = 1 THEN 'automatico' ELSE 'manual' END;

CREATE UNIQUE INDEX idx_backup_registro_nome ON backup_registro(nome_arquivo);

CREATE TABLE backup_configuracao (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    ativo INTEGER NOT NULL DEFAULT 1 CHECK (ativo IN (0, 1)),
    horario TEXT NOT NULL DEFAULT '03:00' CHECK (
        length(horario) = 5 AND substr(horario, 3, 1) = ':'
    ),
    manter_diarios INTEGER NOT NULL DEFAULT 7 CHECK (manter_diarios BETWEEN 0 AND 365),
    manter_semanais INTEGER NOT NULL DEFAULT 4 CHECK (manter_semanais BETWEEN 0 AND 104),
    manter_mensais INTEGER NOT NULL DEFAULT 6 CHECK (manter_mensais BETWEEN 0 AND 60),
    ultima_execucao_em TEXT,
    ultima_tentativa_em TEXT,
    ultimo_erro TEXT,
    data_atualizacao DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO backup_configuracao (
    id, ativo, horario, manter_diarios, manter_semanais, manter_mensais
) VALUES (1, 1, '03:00', 7, 4, 6);
