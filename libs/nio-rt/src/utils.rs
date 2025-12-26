#[inline]
/// Safety: caller must insure that `data` is not empty.
pub unsafe fn min_by_key<T, B: Ord>(data: &[T], f: fn(&T) -> B) -> &T {
    debug_assert!(!data.is_empty());

    let mut val_x = unsafe { data.get_unchecked(0) };
    let mut x = f(val_x);

    for val_y in unsafe { data.get_unchecked(1..) } {
        let y = f(val_y);
        if x < y {
            break;
        }
        x = y;
        val_x = val_y;
    }
    val_x
}
