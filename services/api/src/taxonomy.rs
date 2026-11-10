//! 개념: 주제, 태그, 시리즈의 순서 규칙.
//! 목적: 시리즈 순서를 한 번에 바꾸고, 같은 글을 두 번 넣지 않는다.
//! 사용처: 분류 연결과 `PUT /api/admin/series/{id}/order`.

use std::collections::HashSet;
use uuid::Uuid;

/// 개념: 분류 종류.
/// 목적: 주제·태그·시리즈 이외의 값을 저장하지 않는다.
/// 사용처: 분류 생성.
pub fn valid_kind(kind: &str) -> bool {
    matches!(kind, "topic" | "tag" | "series")
}

/// 개념: 요청한 글 목록에 중복이 없는지 본다.
/// 목적: 한 요청이 같은 글을 두 위치에 넣지 못하게 한다.
/// 사용처: 분류 연결과 순서 변경.
pub fn unique_ids(ids: &[Uuid]) -> bool {
    let mut seen = HashSet::new();
    ids.iter().all(|id| seen.insert(*id))
}

/// 개념: 시리즈에 이미 있는 글만 새 순서로 배치한다.
/// 목적: 빠지거나 중복된 순서 요청은 저장하기 전에 거절한다.
/// 사용처: `reorder_series`.
pub fn order_plan(existing: &[Uuid], requested: &[Uuid]) -> Result<Vec<(Uuid, i32)>, &'static str> {
    if existing.len() != requested.len() {
        return Err("mismatch");
    }
    if !unique_ids(requested) {
        return Err("duplicate");
    }
    let known: HashSet<&Uuid> = existing.iter().collect();
    let mut plan = Vec::with_capacity(requested.len());
    for (index, id) in requested.iter().enumerate() {
        if !known.contains(id) {
            return Err("unknown");
        }
        let position = i32::try_from(index + 1).map_err(|_| "overflow")?;
        plan.push((*id, position));
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::order_plan;
    use uuid::Uuid;

    #[test]
    fn order_plan_rejects_duplicates_and_swaps_members() {
        let first = Uuid::nil();
        let second = Uuid::from_u128(2);
        assert_eq!(order_plan(&[first, second], &[first, first]).unwrap_err(), "duplicate");
        let plan = order_plan(&[first, second], &[second, first]).expect("swap");
        assert_eq!(plan, vec![(second, 1), (first, 2)]);
        assert!(order_plan(&[first], &[]).is_err());
    }
}
