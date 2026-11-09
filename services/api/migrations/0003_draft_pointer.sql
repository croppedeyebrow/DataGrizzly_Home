-- 개념: 글의 초안 포인터.
-- 목적: 저장이 성공한 수정본을 entry가 가리키게 한다. 공개 수정본은 비워 둔다.
-- 사용처: PUT /api/admin/entries/{id}. 0002는 이미 적용된 파일이므로 수정하지 않는다.

ALTER TABLE entry ADD COLUMN summary text NOT NULL DEFAULT '';
ALTER TABLE entry ADD COLUMN draft_revision_id uuid;
ALTER TABLE entry ADD COLUMN published_revision_id uuid;

UPDATE entry AS target
SET draft_revision_id = revision.id
FROM revision
WHERE revision.entry_id = target.id
  AND revision.version = target.version
  AND target.draft_revision_id IS NULL;

ALTER TABLE entry
    ADD CONSTRAINT entry_draft_revision_id_fkey
    FOREIGN KEY (draft_revision_id) REFERENCES revision (id) ON DELETE SET NULL;

ALTER TABLE entry
    ADD CONSTRAINT entry_published_revision_id_fkey
    FOREIGN KEY (published_revision_id) REFERENCES revision (id) ON DELETE SET NULL;
