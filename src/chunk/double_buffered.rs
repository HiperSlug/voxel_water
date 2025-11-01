#[derive(Default)]
pub struct DoubleBuffered<T> {
    pub front: T,
    pub back: T,
}

impl<T> DoubleBuffered<T> {
    #[inline]
    pub fn for_each(&mut self, f: impl Fn(&mut T)) {
        f(&mut self.front);
        f(&mut self.back);
    }
}

impl<T: Clone> DoubleBuffered<T> {
    #[inline]
    pub fn copy_back_to_front(&mut self) {
        self.front = self.back.clone();
    }
}
