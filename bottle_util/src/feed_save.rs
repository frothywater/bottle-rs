/// Shared utilities for feed save operations.
/// These help reduce boilerplate in the complex save() implementations.

use std::collections::HashSet;

use bottle_core::feed::SaveResult;

/// Helper to check if fetched items are empty and handle the response.
pub fn check_empty_fetch(is_empty: bool, is_backward: bool) -> Option<SaveResult> {
    if is_empty {
        Some(SaveResult {
            post_ids: vec![],
            should_stop: true,
            reached_end: is_backward,
        })
    } else {
        None
    }
}

/// Helper to filter out already existing items from fetched items.
pub fn filter_existing<T, F>(
    fetched_items: &[T],
    existing_ids: Vec<impl Into<u64>>,
    get_id: F,
) -> (Vec<&T>, HashSet<u64>)
where
    F: Fn(&T) -> u64,
{
    let existing_set: HashSet<u64> = existing_ids.into_iter().map(Into::into).collect();
    let new_items: Vec<&T> = fetched_items.iter().filter(|item| !existing_set.contains(&get_id(item))).collect();
    (new_items, existing_set)
}

/// Helper to check if all items already exist.
pub fn all_exist(new_items_count: usize, has_existing: bool) -> Option<SaveResult> {
    if new_items_count == 0 && has_existing {
        Some(SaveResult {
            post_ids: vec![],
            should_stop: true,
            reached_end: false,
        })
    } else {
        None
    }
}

/// Standard save result with post IDs and stop condition.
pub fn create_save_result(post_ids: Vec<String>, should_stop: bool, reached_end: bool) -> SaveResult {
    SaveResult {
        post_ids,
        should_stop,
        reached_end,
    }
}
