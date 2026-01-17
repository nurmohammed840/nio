#[inline]
/// Safety: caller must insure that `data` is not empty.
pub unsafe fn find_index_of_lowest<T, B: Ord>(data: &[T], min: B, f: impl Fn(&T) -> B) -> usize {
    debug_assert!(!data.is_empty());

    let mut x = f(unsafe { data.get_unchecked(0) });
    let mut x_idx = 0;

    if x <= min {
        return x_idx;
    }

    for y_idx in 1..data.len() {
        let y = f(unsafe { data.get_unchecked(y_idx) });
        if y <= min {
            return y_idx;
        }
        if x < y {
            break;
        }
        x = y;
        x_idx = y_idx;
    }
    x_idx
}
