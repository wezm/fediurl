CREATE TABLE "instances"
(
    id            INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    domain        TEXT    NOT NULL CHECK ( length(domain) <= 253 ),
    client_id     TEXT    NOT NULL,
    client_secret TEXT    NOT NULL,
    banned_until  INTEGER NULL,
    created_at    INTEGER NOT NULL DEFAULT ( unixepoch() ),
    updated_at    INTEGER NOT NULL DEFAULT ( unixepoch() )
) STRICT;

CREATE UNIQUE INDEX instances_domain_idx ON instances (domain);
