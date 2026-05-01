use aspen_traits::KvRead;

pub fn needs_async_trait<T: KvRead>(_store: &T) {}
