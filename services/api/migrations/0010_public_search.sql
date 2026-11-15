-- 기능: 공개 검색에 쓰는 문서와 절.
-- 목적: 발행된 릴리스의 제목, 본문, 절, 앵커, 분류만 저장한다. 초안 원본 테이블은 검색하지 않는다.
-- 연관: services/api/src/discovery.rs, services/api/src/snapshot.rs의 발행 확인, entry_denylist.

CREATE TABLE public_search_document (
    id uuid PRIMARY KEY,
    release_id uuid NOT NULL REFERENCES site_release (id),
    entry_id uuid NOT NULL REFERENCES entry (id),
    revision_id uuid NOT NULL REFERENCES revision (id),
    title text NOT NULL,
    body text NOT NULL,
    kind text NOT NULL,
    project_slug text NOT NULL DEFAULT '',
    taxonomy text NOT NULL DEFAULT '',
    state text NOT NULL CHECK (state IN ('candidate')),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX public_search_document_release_idx ON public_search_document (release_id, entry_id);

CREATE TABLE public_search_section (
    id uuid PRIMARY KEY,
    document_id uuid NOT NULL REFERENCES public_search_document (id),
    release_id uuid NOT NULL REFERENCES site_release (id),
    entry_id uuid NOT NULL REFERENCES entry (id),
    revision_id uuid NOT NULL REFERENCES revision (id),
    heading text NOT NULL,
    anchor text NOT NULL,
    body text NOT NULL,
    position integer NOT NULL CHECK (position >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX public_search_section_release_idx ON public_search_section (release_id, entry_id, position);

CREATE TABLE public_search_load (
    release_id uuid PRIMARY KEY REFERENCES site_release (id),
    document_count integer NOT NULL CHECK (document_count >= 0),
    section_count integer NOT NULL CHECK (section_count >= 0),
    validation_count integer NOT NULL CHECK (validation_count >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);
