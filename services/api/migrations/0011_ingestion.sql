-- 기능: 문서 접수에 쓰는 업무 테이블.
-- 목적: 작업 상태와 체크포인트 그래프를 분리하고, 기존 CMS 행은 그대로 둔다.
-- 연관: services/api/src/ingestion.rs, contracts/ingestion/openapi.yaml, docs/11_document_workflow/WF04_DATA_API.md.
-- 되돌림: 아래 새 테이블만 역순으로 DROP 한다. entry, media, owner, schema_meta, workflow 체크포인트는 지우지 않는다.
-- DROP TABLE ingestion_requests, ingestion_draft_receipts, ingestion_events, ingestion_commands, ingestion_artifacts, ingestion_jobs, ingestion_sources;

CREATE TABLE ingestion_sources (
    id uuid PRIMARY KEY REFERENCES media (id),
    owner_id uuid NOT NULL REFERENCES owner (id),
    kind text NOT NULL CHECK (kind IN ('upload')),
    private_object_key text NOT NULL,
    sha256 text NOT NULL,
    size integer NOT NULL CHECK (size > 0),
    mime text NOT NULL,
    original_url text,
    fetched_at timestamptz,
    metadata_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ingestion_sources_owner_sha_idx ON ingestion_sources (owner_id, sha256);

CREATE TABLE ingestion_jobs (
    id uuid PRIMARY KEY,
    owner_id uuid NOT NULL REFERENCES owner (id),
    source_id uuid NOT NULL REFERENCES ingestion_sources (id),
    status text NOT NULL CHECK (status IN (
        'QUEUED', 'RUNNING', 'NEEDS_REVIEW', 'RESUME_QUEUED', 'RETRY_WAIT',
        'DRAFT_SAVED', 'FAILED', 'REJECTED', 'CANCELLED'
    )),
    version integer NOT NULL CHECK (version > 0),
    workflow_version integer NOT NULL CHECK (workflow_version > 0),
    thread_id uuid NOT NULL UNIQUE,
    attempt integer NOT NULL DEFAULT 0 CHECK (attempt >= 0),
    next_attempt_at timestamptz,
    lease_token text,
    lease_until timestamptz,
    cancel_requested_at timestamptz,
    draft_id uuid,
    error_code text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ingestion_jobs_owner_status_idx ON ingestion_jobs (owner_id, status, updated_at DESC);

CREATE TABLE ingestion_requests (
    owner_id uuid NOT NULL REFERENCES owner (id),
    request_id uuid NOT NULL,
    payload_hash text NOT NULL,
    job_id uuid NOT NULL REFERENCES ingestion_jobs (id),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, request_id)
);

CREATE TABLE ingestion_artifacts (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id uuid NOT NULL REFERENCES ingestion_jobs (id),
    stage text NOT NULL,
    artifact_version integer NOT NULL CHECK (artifact_version > 0),
    object_key text NOT NULL,
    content_hash text NOT NULL,
    size integer NOT NULL CHECK (size >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (job_id, stage, artifact_version)
);

CREATE TABLE ingestion_commands (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    command_id uuid NOT NULL UNIQUE,
    job_id uuid NOT NULL REFERENCES ingestion_jobs (id),
    expected_review_version integer NOT NULL CHECK (expected_review_version > 0),
    action text NOT NULL CHECK (action IN ('approve', 'reject')),
    sanitized_payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    status text NOT NULL CHECK (status IN ('stored', 'applied', 'rejected')),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ingestion_commands_one_approve_idx
    ON ingestion_commands (job_id, expected_review_version)
    WHERE action = 'approve' AND status <> 'rejected';

CREATE TABLE ingestion_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id uuid NOT NULL REFERENCES ingestion_jobs (id),
    event_type text NOT NULL,
    safe_details jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ingestion_draft_receipts (
    job_id uuid PRIMARY KEY REFERENCES ingestion_jobs (id),
    idempotency_key text NOT NULL UNIQUE,
    payload_hash text NOT NULL,
    entry_id uuid NOT NULL REFERENCES entry (id),
    revision_id uuid NOT NULL REFERENCES revision (id),
    created_at timestamptz NOT NULL DEFAULT now()
);
