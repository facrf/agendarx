ALTER TABLE pessoa_vinculo ADD COLUMN excluido_em DATETIME;

CREATE INDEX idx_vinculo_lixeira ON pessoa_vinculo(excluido_em, id);
