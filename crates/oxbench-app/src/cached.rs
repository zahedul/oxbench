/// Remembers which inputs a result came from, so work is redone only when they change.
pub struct Cached<K, V> {
    pub key: K,
    pub value: V,
}

impl<K: PartialEq, V> Cached<K, V> {
    /// The cached value for `key`, computing it first when the slot is empty or stale.
    pub fn refresh(slot: &mut Option<Self>, key: K, compute: impl FnOnce(&K) -> V) -> &V {
        let is_stale = slot.as_ref().is_some_and(|cached| cached.key != key);
        if is_stale {
            *slot = None;
        }
        let cached = slot.get_or_insert_with(|| {
            let value = compute(&key);
            Cached { key, value }
        });
        &cached.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recomputes_only_when_the_key_changes() {
        let mut slot = None;
        let mut runs = 0;
        let mut double = |key: &i32| {
            runs += 1;
            key * 2
        };
        assert_eq!(*Cached::refresh(&mut slot, 2, &mut double), 4);
        assert_eq!(*Cached::refresh(&mut slot, 2, &mut double), 4);
        assert_eq!(*Cached::refresh(&mut slot, 3, &mut double), 6);
        assert_eq!(runs, 2);
    }
}
