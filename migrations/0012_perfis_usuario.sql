ALTER TABLE usuario ADD COLUMN perfil TEXT NOT NULL DEFAULT 'usuario'
    CHECK (perfil IN ('admin', 'usuario'));
-- As contas anteriores à gestão de usuários já eram administrativas.
UPDATE usuario SET perfil = 'admin';
