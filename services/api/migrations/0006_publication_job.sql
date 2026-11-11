-- 개념: 발행 접수 작업.
-- 목적: 같은 요청은 한 작업으로 모으고, 오래된 작업자의 완료가 새 세대를 덮지 않게 한다.
-- 사용처: POST /api/admin/entries/{id}/publications. 0001부터 0005는 수정하지 않는다.
-- 공개 포인터와 공개 파일은 이 표에서 바꾸지 않는다. 공개 확인은 이후 배포 태스크다.

CREATE TABLE publication_job (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    job_number bigint GENERATED ALWAYS AS IDENTITY,
    entry_id uuid NOT NULL REFERENCES entry (id),
    revision_id uuid NOT NULL REFERENCES revision (id),
    state text NOT NULL CHECK (state IN ('queued', 'building', 'deploying', 'succeeded', 'failed', 'canceled')),
    fence bigint NOT NULL DEFAULT 0 CHECK (fence >= 0),
    attempt integer NOT NULL DEFAULT 0 CHECK (attempt >= 0),
    operator_retries integer NOT NULL DEFAULT 0 CHECK (operator_retries >= 0),
    failure_code text,
    release_id uuid,
    lease_until timestamptz,
    retry_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX publication_job_state_idx ON publication_job (state, created_at);

CREATE TABLE publication_request (
    idempotency_key text PRIMARY KEY,
    payload_hash text NOT NULL,
    job_id uuid NOT NULL REFERENCES publication_job (id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE publication_upload (
    job_id uuid NOT NULL REFERENCES publication_job (id),
    generation bigint NOT NULL CHECK (generation > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (job_id, generation)
);
