-- Desativa a política automática de backup por padrão até decisão explícita do administrador.
ALTER TABLE backup_configuracao
    ADD COLUMN politica_confirmada INTEGER NOT NULL DEFAULT 0
    CHECK (politica_confirmada IN (0, 1));

UPDATE backup_configuracao
SET ativo = 0,
    data_atualizacao = CURRENT_TIMESTAMP
WHERE id = 1 AND politica_confirmada = 0;
