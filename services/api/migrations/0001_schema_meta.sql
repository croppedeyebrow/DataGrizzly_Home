CREATE TABLE schema_meta (
    schema_version integer PRIMARY KEY,
    applied_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO schema_meta (schema_version) VALUES (1);
