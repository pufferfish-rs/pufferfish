struct AbortOnDrop;

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        std::process::abort();
    }
}

pub fn replace_with<T, F: FnOnce(T) -> T>(dest: &mut T, f: F) {
    // SAFETY: We abort if `f` panics, so we're guaranteed to end up with a valid value.
    unsafe {
        let old = std::ptr::read(dest);
        let abort = AbortOnDrop;
        let new = f(old);
        std::mem::forget(abort);
        std::ptr::write(dest, new);
    }
}

pub fn type_name<T>() -> &'static str {
    let s = std::any::type_name::<T>();
    &s[s.rmatch_indices("::")
        .find_map(|(j, _)| (s.find('<').unwrap_or(s.len()) > j).then(|| j + 2))
        .unwrap_or(0)..]
}
