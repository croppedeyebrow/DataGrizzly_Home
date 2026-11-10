-- 개념: 첨부, 분류, 과정 연결의 저장 구조.
-- 목적: 파일 경로와 원본 이름을 분리하고, 분류를 지워도 글이 남게 한다.
-- 사용처: 미디어 업로드, 주제·시리즈, 프로젝트 과정 기록. 0001부터 0003은 수정하지 않는다.

ALTER TABLE project ADD COLUMN status text NOT NULL DEFAULT '개발 중';

ALTER TABLE project
    ADD CONSTRAINT project_status_check
    CHECK (status IN ('공개', '개발 중', '유지보수'));

ALTER TABLE project_log ADD COLUMN display_order integer NOT NULL DEFAULT 0;

ALTER TABLE project_log ADD COLUMN commit_url text NOT NULL DEFAULT '';

ALTER TABLE project_log
    ADD CONSTRAINT project_log_display_order_check
    CHECK (display_order >= 0 AND display_order <= 10000);

CREATE TABLE media (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    storage_key text NOT NULL UNIQUE,
    original_name text NOT NULL,
    media_type text NOT NULL,
    bytes integer NOT NULL CHECK (bytes > 0 AND bytes <= 8388608),
    checksum text NOT NULL,
    status text NOT NULL CHECK (status IN ('uploading', 'verified', 'referenced')),
    public boolean NOT NULL DEFAULT false,
    alt text NOT NULL DEFAULT '',
    caption text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX media_created_at_idx ON media (created_at DESC);

CREATE TABLE revision_media (
    revision_id uuid NOT NULL REFERENCES revision (id) ON DELETE RESTRICT,
    media_id uuid NOT NULL REFERENCES media (id) ON DELETE RESTRICT,
    PRIMARY KEY (revision_id, media_id)
);

CREATE INDEX revision_media_media_id_idx ON revision_media (media_id);

CREATE TABLE taxonomy (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    kind text NOT NULL CHECK (kind IN ('topic', 'tag', 'series')),
    slug text NOT NULL,
    title text NOT NULL,
    description text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (kind, slug)
);

CREATE TABLE entry_taxonomy (
    entry_id uuid NOT NULL REFERENCES entry (id) ON DELETE CASCADE,
    taxonomy_id uuid NOT NULL REFERENCES taxonomy (id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, taxonomy_id)
);

CREATE INDEX entry_taxonomy_taxonomy_id_idx ON entry_taxonomy (taxonomy_id);

CREATE TABLE series_entry (
    series_id uuid NOT NULL REFERENCES taxonomy (id) ON DELETE CASCADE,
    entry_id uuid NOT NULL REFERENCES entry (id) ON DELETE CASCADE,
    position integer NOT NULL CHECK (position > 0),
    PRIMARY KEY (series_id, entry_id),
    UNIQUE (series_id, position)
);
