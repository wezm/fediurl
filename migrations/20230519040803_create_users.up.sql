CREATE TABLE "users"
(
    id           INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    instance_id  INTEGER NOT NULL,
    access_token TEXT    NOT NULL,
    banned_until INTEGER NULL,
    created_at    INTEGER NOT NULL DEFAULT ( unixepoch() ),
    updated_at    INTEGER NOT NULL DEFAULT ( unixepoch() ),

    FOREIGN KEY (instance_id) REFERENCES instances (id)
) STRICT;
