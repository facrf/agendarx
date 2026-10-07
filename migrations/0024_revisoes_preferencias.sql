ALTER TABLE pessoa ADD COLUMN versao INTEGER NOT NULL DEFAULT 1;
ALTER TABLE pessoa ADD COLUMN mesclada_destino_id INTEGER REFERENCES pessoa(id) ON DELETE SET NULL;
ALTER TABLE pessoa ADD COLUMN mesclada_em TEXT;
ALTER TABLE pessoa_vinculo ADD COLUMN versao INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tarefa_calendario ADD COLUMN versao INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tarefa_calendario ADD COLUMN lembrete_adiado_ate TEXT;
CREATE TABLE revisao_edicao (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 tipo TEXT NOT NULL CHECK(tipo IN ('pessoa','vinculo','tarefa')),
 recurso_id INTEGER NOT NULL,
 usuario_id INTEGER REFERENCES usuario(id) ON DELETE SET NULL,
 autor_login TEXT NOT NULL,
 versao_anterior INTEGER NOT NULL,
 anterior_json TEXT NOT NULL,
 criado_em TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_revisao_recurso ON revisao_edicao(tipo,recurso_id,id DESC);
CREATE TABLE preferencia_lembrete (
 usuario_id INTEGER PRIMARY KEY REFERENCES usuario(id) ON DELETE CASCADE,
 silencioso_inicio TEXT,
 silencioso_fim TEXT,
 fuso_horario TEXT NOT NULL DEFAULT 'America/Sao_Paulo'
);
CREATE TRIGGER versao_pessoa AFTER UPDATE ON pessoa WHEN NEW.versao=OLD.versao
BEGIN UPDATE pessoa SET versao=versao+1 WHERE id=NEW.id; END;
CREATE TRIGGER versao_vinculo AFTER UPDATE ON pessoa_vinculo WHEN NEW.versao=OLD.versao
BEGIN UPDATE pessoa_vinculo SET versao=versao+1 WHERE id=NEW.id; END;
CREATE TRIGGER versao_tarefa AFTER UPDATE OF titulo,descricao,inicio_em,fim_em,dia_inteiro,status,prioridade,cor_hex,lembrete_minutos ON tarefa_calendario WHEN NEW.versao=OLD.versao
BEGIN UPDATE tarefa_calendario SET versao=versao+1 WHERE id=NEW.id; END;
CREATE TRIGGER versao_contato_insert AFTER INSERT ON contato
BEGIN UPDATE pessoa SET versao=versao+1 WHERE id=NEW.pessoa_id; END;
CREATE TRIGGER versao_contato_update AFTER UPDATE ON contato
BEGIN UPDATE pessoa SET versao=versao+1 WHERE id IN (NEW.pessoa_id,OLD.pessoa_id); END;
CREATE TRIGGER versao_contato_delete AFTER DELETE ON contato
BEGIN UPDATE pessoa SET versao=versao+1 WHERE id=OLD.pessoa_id; END;
ALTER TABLE pessoa ADD COLUMN mesclagem_revisao_limite INTEGER NOT NULL DEFAULT 0;
CREATE TABLE mesclagem_registro (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 origem_id INTEGER NOT NULL,
 destino_id INTEGER NOT NULL,
 autor_login TEXT NOT NULL,
 origem_json TEXT NOT NULL,
 destino_json TEXT NOT NULL,
 criado_em TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TRIGGER versao_tarefa_pessoa_insert AFTER INSERT ON tarefa_calendario_pessoa
BEGIN UPDATE tarefa_calendario SET versao=versao+1 WHERE id=NEW.tarefa_id; END;
CREATE TRIGGER versao_tarefa_pessoa_delete AFTER DELETE ON tarefa_calendario_pessoa
BEGIN UPDATE tarefa_calendario SET versao=versao+1 WHERE id=OLD.tarefa_id; END;
