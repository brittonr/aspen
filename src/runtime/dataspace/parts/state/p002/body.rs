impl RuntimeState {
    fn has_visible_assertion(&self, value: &Value) -> bool {
        self.assertions.iter().any(|assertion| assertion.value == *value)
    }

    fn holds_assertion(&self, actor: &str, value: &Value) -> bool {
        self.assertions.iter().any(|assertion| assertion.actor == actor && assertion.value == *value)
    }

    fn visible_owner_count(&self, value: &Value) -> usize {
        self.assertions.iter().filter(|assertion| assertion.value == *value).count()
    }
}
