#[inline]
/// Safety: caller must insure that `data` is not empty.
pub unsafe fn min_index_by_key<T, B: Ord>(data: &[T], f: fn(&T) -> B) -> usize {
    debug_assert!(!data.is_empty());

    let mut x = f(unsafe { data.get_unchecked(0) });
    let mut x_idx = 0;

    for y_idx in 1..data.len() {
        let y = f(unsafe { data.get_unchecked(y_idx) });
        if x < y {
            break;
        }
        x = y;
        x_idx = y_idx;
    }
    x_idx
}
