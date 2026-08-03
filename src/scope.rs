use std::collections::HashMap;

#[derive(Debug)]
/// Stack of lexical scopes that resolves names from innermost to outermost.
pub struct ScopedMap<T> {
    scopes: Vec<HashMap<String, T>>,
}

impl<T> Default for ScopedMap<T> {
    fn default() -> Self {
        Self { scopes: Vec::new() }
    }
}

impl<T: Clone> ScopedMap<T> {
    /// Creates a scoped map with a single root scope already active.
    pub fn new() -> Self {
        let mut map = Self::default();
        map.enter_scope();
        map
    }

    /// Returns the zero-based depth of the current scope.
    pub fn depth(&self) -> usize {
        self.scopes.len().saturating_sub(1)
    }

    /// Pushes a new child scope.
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pops the current scope.
    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    /// Declares a name in the current scope and rejects same-scope duplicates.
    pub fn declare<E, F>(&mut self, name: &str, value: T, on_duplicate: F) -> Result<(), E>
    where
        F: FnOnce(&str) -> E,
    {
        let scope = self
            .scopes
            .last_mut()
            .expect("scoped map must always have an active scope");

        if scope.contains_key(name) {
            return Err(on_duplicate(name));
        }

        scope.insert(name.to_string(), value);
        Ok(())
    }

    /// Resolves a name by searching from the innermost scope outward.
    pub fn resolve(&self, name: &str) -> Option<&T> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    /// Clones all values declared directly in the current scope.
    pub fn current_scope_values(&self) -> Vec<T> {
        let scope = self
            .scopes
            .last()
            .expect("scoped map must always have an active scope");
        scope.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests;
