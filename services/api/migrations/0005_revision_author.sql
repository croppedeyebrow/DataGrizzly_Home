-- 개념: 수정본을 만든 소유자.
-- 목적: 이력 목록에 변경자를 적고, 과거 수정본 행은 그대로 둔다.
-- 사용처: 미리보기와 복원. 0001부터 0004는 수정하지 않는다.

ALTER TABLE revision
    ADD COLUMN created_by uuid REFERENCES owner (id) ON DELETE SET NULL;
