-- Índice compartilhado: documentos públicos e tarefas segregadas por usuário.
CREATE TABLE busca_documento (
 id INTEGER PRIMARY KEY, tipo TEXT NOT NULL, recurso_id INTEGER NOT NULL,
 pessoa_id INTEGER, usuario_id INTEGER, titulo TEXT NOT NULL, conteudo TEXT NOT NULL,
 UNIQUE(tipo, recurso_id)
);
CREATE INDEX idx_busca_documento_pessoa ON busca_documento(pessoa_id, tipo);
CREATE INDEX idx_busca_documento_usuario ON busca_documento(usuario_id, tipo);
CREATE VIRTUAL TABLE busca_fts USING fts5(titulo, conteudo, content='busca_documento', content_rowid='id', tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER busca_documento_ai AFTER INSERT ON busca_documento BEGIN
 INSERT INTO busca_fts(rowid,titulo,conteudo) VALUES(new.id,new.titulo,new.conteudo);
END;
CREATE TRIGGER busca_documento_ad AFTER DELETE ON busca_documento BEGIN
 INSERT INTO busca_fts(busca_fts,rowid,titulo,conteudo) VALUES('delete',old.id,old.titulo,old.conteudo);
END;
CREATE TRIGGER busca_documento_au AFTER UPDATE ON busca_documento BEGIN
 INSERT INTO busca_fts(busca_fts,rowid,titulo,conteudo) VALUES('delete',old.id,old.titulo,old.conteudo);
 INSERT INTO busca_fts(rowid,titulo,conteudo) VALUES(new.id,new.titulo,new.conteudo);
END;
CREATE VIEW busca_fonte AS
 SELECT 'pessoa' AS tipo, p.id AS recurso_id, p.id AS pessoa_id, NULL AS usuario_id, p.nome AS titulo,
 COALESCE(p.descricao,'') || ' ' || COALESCE((SELECT group_concat(valor,' ') FROM contato WHERE pessoa_id=p.id),'') || ' ' || COALESCE((SELECT group_concat(e.nome,' ') FROM etiqueta e JOIN pessoa_etiqueta pe ON pe.etiqueta_id=e.id WHERE pe.pessoa_id=p.id),'') AS conteudo FROM pessoa p
 UNION ALL SELECT 'tarefa',id,NULL,usuario_id,titulo,COALESCE(descricao,'') FROM tarefa_calendario
 UNION ALL SELECT 'vinculo',v.id,v.pessoa_origem_id,NULL,po.nome || ' ↔ ' || pd.nome || ' · ' || v.tipo_vinculo,COALESCE(v.descricao,'') FROM pessoa_vinculo v JOIN pessoa po ON po.id=v.pessoa_origem_id JOIN pessoa pd ON pd.id=v.pessoa_destino_id
 UNION ALL SELECT 'anexo_dossie',a.id,a.pessoa_id,NULL,a.nome_arquivo,COALESCE(a.notas,'') || ' ' || CASE WHEN a.mime_type LIKE 'text/%' OR lower(a.nome_arquivo) GLOB '*.md' OR lower(a.nome_arquivo) GLOB '*.txt' THEN CAST(a.conteudo_blob AS TEXT) ELSE '' END FROM anexo_dossie a
 UNION ALL SELECT 'anexo_vinculo',a.id,v.pessoa_origem_id,NULL,a.nome_arquivo,COALESCE(a.notas,'') || ' ' || CASE WHEN a.mime_type LIKE 'text/%' OR lower(a.nome_arquivo) GLOB '*.md' OR lower(a.nome_arquivo) GLOB '*.txt' THEN CAST(a.conteudo_blob AS TEXT) ELSE '' END FROM anexo_vinculo a JOIN pessoa_vinculo v ON v.id=a.vinculo_id
 UNION ALL SELECT 'anexo_tarefa',a.id,NULL,t.usuario_id,a.nome_arquivo,COALESCE(a.notas,'') || ' ' || CASE WHEN a.mime_type LIKE 'text/%' OR lower(a.nome_arquivo) GLOB '*.md' OR lower(a.nome_arquivo) GLOB '*.txt' THEN CAST(a.conteudo_blob AS TEXT) ELSE '' END FROM anexo_tarefa_calendario a JOIN tarefa_calendario t ON t.id=a.tarefa_id
;
CREATE TRIGGER busca_pessoa_insert AFTER INSERT ON pessoa BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_pessoa_update AFTER UPDATE ON pessoa BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_pessoa_delete AFTER DELETE ON pessoa BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id=old.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id=old.id;
END;
CREATE TRIGGER busca_tarefa_calendario_insert AFTER INSERT ON tarefa_calendario BEGIN
 DELETE FROM busca_documento WHERE tipo='tarefa' AND recurso_id=new.id OR (tipo='anexo_tarefa' AND recurso_id IN (SELECT id FROM anexo_tarefa_calendario WHERE tarefa_id=new.id));
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='tarefa' AND recurso_id=new.id OR (tipo='anexo_tarefa' AND recurso_id IN (SELECT id FROM anexo_tarefa_calendario WHERE tarefa_id=new.id));
END;
CREATE TRIGGER busca_tarefa_calendario_update AFTER UPDATE ON tarefa_calendario BEGIN
 DELETE FROM busca_documento WHERE tipo='tarefa' AND recurso_id=new.id OR (tipo='anexo_tarefa' AND recurso_id IN (SELECT id FROM anexo_tarefa_calendario WHERE tarefa_id=new.id));
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='tarefa' AND recurso_id=new.id OR (tipo='anexo_tarefa' AND recurso_id IN (SELECT id FROM anexo_tarefa_calendario WHERE tarefa_id=new.id));
END;
CREATE TRIGGER busca_tarefa_calendario_delete AFTER DELETE ON tarefa_calendario BEGIN
 DELETE FROM busca_documento WHERE tipo='tarefa' AND recurso_id=old.id OR (tipo='anexo_tarefa' AND recurso_id IN (SELECT id FROM anexo_tarefa_calendario WHERE tarefa_id=old.id));
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='tarefa' AND recurso_id=old.id OR (tipo='anexo_tarefa' AND recurso_id IN (SELECT id FROM anexo_tarefa_calendario WHERE tarefa_id=old.id));
END;
CREATE TRIGGER busca_pessoa_vinculo_insert AFTER INSERT ON pessoa_vinculo BEGIN
 DELETE FROM busca_documento WHERE tipo='vinculo' AND recurso_id=new.id OR (tipo='anexo_vinculo' AND recurso_id IN (SELECT id FROM anexo_vinculo WHERE vinculo_id=new.id));
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='vinculo' AND recurso_id=new.id OR (tipo='anexo_vinculo' AND recurso_id IN (SELECT id FROM anexo_vinculo WHERE vinculo_id=new.id));
END;
CREATE TRIGGER busca_pessoa_vinculo_update AFTER UPDATE ON pessoa_vinculo BEGIN
 DELETE FROM busca_documento WHERE tipo='vinculo' AND recurso_id=new.id OR (tipo='anexo_vinculo' AND recurso_id IN (SELECT id FROM anexo_vinculo WHERE vinculo_id=new.id));
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='vinculo' AND recurso_id=new.id OR (tipo='anexo_vinculo' AND recurso_id IN (SELECT id FROM anexo_vinculo WHERE vinculo_id=new.id));
END;
CREATE TRIGGER busca_pessoa_vinculo_delete AFTER DELETE ON pessoa_vinculo BEGIN
 DELETE FROM busca_documento WHERE tipo='vinculo' AND recurso_id=old.id OR (tipo='anexo_vinculo' AND recurso_id IN (SELECT id FROM anexo_vinculo WHERE vinculo_id=old.id));
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='vinculo' AND recurso_id=old.id OR (tipo='anexo_vinculo' AND recurso_id IN (SELECT id FROM anexo_vinculo WHERE vinculo_id=old.id));
END;
CREATE TRIGGER busca_anexo_dossie_insert AFTER INSERT ON anexo_dossie BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_dossie' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_dossie' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_anexo_dossie_update AFTER UPDATE ON anexo_dossie BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_dossie' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_dossie' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_anexo_dossie_delete AFTER DELETE ON anexo_dossie BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_dossie' AND recurso_id=old.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_dossie' AND recurso_id=old.id;
END;
CREATE TRIGGER busca_anexo_vinculo_insert AFTER INSERT ON anexo_vinculo BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_vinculo' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_vinculo' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_anexo_vinculo_update AFTER UPDATE ON anexo_vinculo BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_vinculo' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_vinculo' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_anexo_vinculo_delete AFTER DELETE ON anexo_vinculo BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_vinculo' AND recurso_id=old.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_vinculo' AND recurso_id=old.id;
END;
CREATE TRIGGER busca_anexo_tarefa_calendario_insert AFTER INSERT ON anexo_tarefa_calendario BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_tarefa' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_tarefa' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_anexo_tarefa_calendario_update AFTER UPDATE ON anexo_tarefa_calendario BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_tarefa' AND recurso_id=new.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_tarefa' AND recurso_id=new.id;
END;
CREATE TRIGGER busca_anexo_tarefa_calendario_delete AFTER DELETE ON anexo_tarefa_calendario BEGIN
 DELETE FROM busca_documento WHERE tipo='anexo_tarefa' AND recurso_id=old.id;
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='anexo_tarefa' AND recurso_id=old.id;
END;
CREATE TRIGGER busca_contato_insert AFTER INSERT ON contato BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id IN (new.pessoa_id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id IN (new.pessoa_id);
END;
CREATE TRIGGER busca_contato_update AFTER UPDATE ON contato BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id,new.pessoa_id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id,new.pessoa_id);
END;
CREATE TRIGGER busca_contato_delete AFTER DELETE ON contato BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id);
END;
CREATE TRIGGER busca_pessoa_etiqueta_insert AFTER INSERT ON pessoa_etiqueta BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id IN (new.pessoa_id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id IN (new.pessoa_id);
END;
CREATE TRIGGER busca_pessoa_etiqueta_update AFTER UPDATE ON pessoa_etiqueta BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id,new.pessoa_id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id,new.pessoa_id);
END;
CREATE TRIGGER busca_pessoa_etiqueta_delete AFTER DELETE ON pessoa_etiqueta BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id IN (old.pessoa_id);
END;
CREATE TRIGGER busca_etiqueta_update AFTER UPDATE OF nome ON etiqueta BEGIN
 DELETE FROM busca_documento WHERE tipo='pessoa' AND recurso_id IN (SELECT pessoa_id FROM pessoa_etiqueta WHERE etiqueta_id=new.id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='pessoa' AND recurso_id IN (SELECT pessoa_id FROM pessoa_etiqueta WHERE etiqueta_id=new.id);
END;
CREATE TRIGGER busca_vinculo_nomes AFTER UPDATE OF nome ON pessoa BEGIN
 DELETE FROM busca_documento WHERE tipo='vinculo' AND recurso_id IN (SELECT id FROM pessoa_vinculo WHERE pessoa_origem_id=new.id OR pessoa_destino_id=new.id);
 INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte WHERE tipo='vinculo' AND recurso_id IN (SELECT id FROM pessoa_vinculo WHERE pessoa_origem_id=new.id OR pessoa_destino_id=new.id);
END;
INSERT INTO busca_documento(tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo) SELECT tipo,recurso_id,pessoa_id,usuario_id,titulo,conteudo FROM busca_fonte;
CREATE VIEW busca_visivel AS
 SELECT d.* FROM busca_documento d
 LEFT JOIN pessoa p ON p.id=d.pessoa_id
 LEFT JOIN anexo_vinculo av ON d.tipo='anexo_vinculo' AND av.id=d.recurso_id
 LEFT JOIN pessoa_vinculo v ON v.id=CASE WHEN d.tipo='vinculo' THEN d.recurso_id ELSE av.vinculo_id END
 LEFT JOIN pessoa po ON po.id=v.pessoa_origem_id
 LEFT JOIN pessoa pd ON pd.id=v.pessoa_destino_id
 WHERE (d.tipo IN ('tarefa','anexo_tarefa'))
 OR (d.tipo IN ('pessoa','anexo_dossie') AND p.id IS NOT NULL AND p.excluida_em IS NULL)
 OR (d.tipo IN ('vinculo','anexo_vinculo') AND v.excluido_em IS NULL AND po.id IS NOT NULL AND pd.id IS NOT NULL AND po.excluida_em IS NULL AND pd.excluida_em IS NULL);
CREATE INDEX idx_auditoria_usuario_data ON auditoria(usuario_login,data_evento DESC,id DESC);
CREATE INDEX idx_auditoria_acao_data ON auditoria(acao,data_evento DESC,id DESC);
