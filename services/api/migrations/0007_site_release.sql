-- 개념: 발행 스냅샷과 릴리스 세대.
-- 목적: 빌드 입력을 수정본 시점에 고정하고, 공개 확인 전에는 성공으로 닫지 않는다.
-- 사용처: 후보 빌드와 포인터 확인. 0001부터 0006은 수정하지 않는다.

CREATE TABLE site_release (
    id uuid PRIMARY KEY,
    job_id uuid NOT NULL UNIQUE REFERENCES publication_job (id),
    entry_id uuid NOT NULL REFERENCES entry (id),
    revision_id uuid NOT NULL REFERENCES revision (id),
    schema_version integer NOT NULL CHECK (schema_version = 1),
    snapshot jsonb NOT NULL,
    snapshot_hash text NOT NULL,
    build_token_hash text NOT NULL,
    manifest_hash text,
    candidate_prefix text NOT NULL,
    phase text NOT NULL CHECK (phase IN ('snapshot', 'candidate', 'verified', 'active', 'failed')),
    failure_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    confirmed_at timestamptz
);

CREATE INDEX site_release_phase_idx ON site_release (phase, created_at);
