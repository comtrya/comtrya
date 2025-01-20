use std::iter::from_fn;

pub trait ZipLongest<T>: Iterator<Item = T> {
    fn zip_longest<U, I>(self, other: I) -> impl Iterator<Item = (Option<T>, Option<U>)>
    where
        Self: Sized,
        I: IntoIterator<Item = U>;
}

impl<T, I: Iterator<Item = T>> ZipLongest<T> for I {
    fn zip_longest<U, J>(self, other: J) -> impl Iterator<Item = (Option<T>, Option<U>)>
    where
        J: IntoIterator<Item = U>,
    {
        let mut a = self.fuse();
        let mut b = other.into_iter().fuse();

        from_fn(move || match (a.next(), b.next()) {
            (None, None) => None,
            (a, b) => Some((a, b)),
        })
    }
}
