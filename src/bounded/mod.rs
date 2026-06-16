pub(crate) trait VecSink<T> {
    fn item_count(&self) -> usize;
    fn reserve_items(&mut self, additional: usize);
    fn push_item(&mut self, value: T);
    fn extend_cloned_items(&mut self, incoming: &[T])
    where T: Clone;
}

impl<T> VecSink<T> for Vec<T> {
    fn item_count(&self) -> usize {
        self.len()
    }

    fn reserve_items(&mut self, additional: usize) {
        self.reserve(additional);
    }

    fn push_item(&mut self, value: T) {
        self.push(value);
    }

    fn extend_cloned_items(&mut self, incoming: &[T])
    where T: Clone {
        self.extend(incoming.iter().cloned());
    }
}
