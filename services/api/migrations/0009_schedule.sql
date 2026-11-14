-- 개념: 발행 예약과 슬러그 리디렉션.
-- 목적: 서울 시간 입력을 UTC로 저장하고, 취소된 예약이 공개로 이어지지 않게 한다.
-- 사용처: 예약 수정, 늦은 실행, 사이트맵. 0001부터 0008은 수정하지 않는다.

CREATE TABLE publication_schedule (
    id uuid PRIMARY KEY,
    entry_id uuid NOT NULL REFERENCES entry (id),
    revision_id uuid NOT NULL REFERENCES revision (id),
    job_id uuid REFERENCES publication_job (id),
    run_at timestamptz NOT NULL,
    display_at text NOT NULL,
    from_path text NOT NULL,
    version integer NOT NULL DEFAULT 1 CHECK (version >= 1),
    state text NOT NULL CHECK (state IN ('scheduled', 'running', 'canceled', 'done')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX publication_schedule_due_idx ON publication_schedule (state, run_at);

CREATE TABLE entry_redirect (
    id uuid PRIMARY KEY,
    entry_id uuid NOT NULL REFERENCES entry (id),
    from_path text NOT NULL UNIQUE,
    to_path text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
