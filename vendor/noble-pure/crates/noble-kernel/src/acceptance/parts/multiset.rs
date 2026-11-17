//! Bounded bag equality for distinguishing stack shape from stack order.

pub(super) fn same(left: &[crate::types::Ty], right: &[crate::types::Ty]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut used = alloc::vec![false; right.len()];
    let mut is_matched = true;
    let mut item_index = 0;
    while is_matched && item_index < left.len() {
        match find_unused(right, &used, &left[item_index]) {
            Some(index) => {
                used[index] = true;
                item_index += 1;
            }
            None => is_matched = false,
        }
    }
    is_matched
}

/// The first unused index in `right` holding `item`, when one exists.
fn find_unused(
    right: &[crate::types::Ty],
    used: &[bool],
    item: &crate::types::Ty,
) -> Option<usize> {
    let mut found = None;
    let mut index = 0;
    while index < right.len() {
        if !used[index] && &right[index] == item {
            found = Some(index);
            break;
        }
        index += 1;
    }
    found
}
