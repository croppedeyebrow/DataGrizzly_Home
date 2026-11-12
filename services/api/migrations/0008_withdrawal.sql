-- 개념: 사이트 세대와 글 철회.
-- 목적: 오래된 발행이 최신 세대를 덮지 않게 하고, 철회한 글은 검색 색인보다 먼저 막는다.
-- 사용처: 발행 확인과 철회. 0001부터 0007은 수정하지 않는다.

ALTER TABLE site_release DROP CONSTRAINT site_release_phase_check;
ALTER TABLE site_release ADD CONSTRAINT site_release_phase_check
    CHECK (phase IN ('snapshot', 'candidate', 'verified', 'active', 'failed', 'superseded'));
ALTER TABLE site_release ADD COLUMN release_sequence bigint;
CREATE UNIQUE INDEX site_release_sequence_idx ON site_release (release_sequence) WHERE release_sequence IS NOT NULL;

CREATE TABLE site_generation (
    id integer PRIMARY KEY CHECK (id = 1),
    sequence bigint NOT NULL DEFAULT 0 CHECK (sequence >= 0)
);

INSERT INTO site_generation (id, sequence) VALUES (1, 0);

CREATE TABLE entry_denylist (
    entry_id uuid PRIMARY KEY REFERENCES entry (id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE withdrawal (
    id uuid PRIMARY KEY,
    entry_id uuid NOT NULL UNIQUE REFERENCES entry (id),
    revision_id uuid REFERENCES revision (id),
    state text NOT NULL CHECK (state IN ('blocked', 'purge_failed')),
    cache_state text NOT NULL CHECK (cache_state IN ('local_only', 'purge_failed')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
