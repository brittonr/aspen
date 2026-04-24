use std::collections::BTreeSet;
use std::fmt;

/// Implement `Display` for `BTreeSet<T>` if `T` is `Display`.
///
/// It formats elements as a comma-separated list enclosed in brackets.
pub(crate) struct DisplayBtreeSet<'a, T: fmt::Display>(pub &'a BTreeSet<T>);

impl<T: fmt::Display> fmt::Display for DisplayBtreeSet<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let len = self.0.len();
        for (idx, item) in self.0.iter().enumerate() {
            write!(f, "{}", item)?;
            let next_index = idx.saturating_add(1);
            if next_index != len {
                write!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

#[allow(unused)]
pub(crate) trait DisplayBtreeSetExt<'a, T: fmt::Display> {
    fn display(&'a self) -> DisplayBtreeSet<'a, T>;
}

impl<T> DisplayBtreeSetExt<'_, T> for BTreeSet<T>
where T: fmt::Display
{
    fn display(&self) -> DisplayBtreeSet<'_, T> {
        DisplayBtreeSet(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_btreeset() {
        let set = (1..=3).collect::<BTreeSet<_>>();
        let display = DisplayBtreeSet(&set);

        assert_eq!(display.to_string(), "[1,2,3]");
    }

    #[test]
    fn test_display_empty_set() {
        let set = BTreeSet::<i32>::new();
        let display = DisplayBtreeSet(&set);

        assert_eq!(display.to_string(), "[]");
    }

    #[test]
    fn test_display_btreeset_with_1_item() {
        let set = (1..=1).collect::<BTreeSet<_>>();
        let display = DisplayBtreeSet(&set);

        assert_eq!(display.to_string(), "[1]");
    }
}
