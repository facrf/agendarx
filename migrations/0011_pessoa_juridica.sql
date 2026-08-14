ALTER TABLE pessoa ADD COLUMN pessoa_juridica INTEGER NOT NULL DEFAULT 0
    CHECK (pessoa_juridica IN (0, 1));
