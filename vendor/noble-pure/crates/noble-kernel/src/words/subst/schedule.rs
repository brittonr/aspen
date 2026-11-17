/// Queue the parts of one program pattern, result stack first.
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; nonzero-sized Pattern slices each have at most isize::MAX entries, so their sum fits usize; the loop and branch guards prove each reverse index stays within its input or output slice."
)]
pub(super) fn queue(
    stack_in: &[crate::shapes::Pattern],
    stack_out: &[crate::shapes::Pattern],
    mut walk: super::Walk,
) -> super::Walk {
    let out_len = stack_out.len();
    let in_len = stack_in.len();
    let mut part_index = 0;
    while part_index < in_len + out_len {
        let part = if part_index < out_len {
            &stack_out[out_len - 1 - part_index]
        } else {
            &stack_in[in_len + out_len - 1 - part_index]
        };
        walk.work.push(super::Task::Part(part.clone()));
        part_index += 1;
    }
    walk
}
