#[derive(Debug, Default)]
pub struct DoubleBuffered<T> {
    front: T,
    back: T,
    swapped: bool,
}

impl<T> DoubleBuffered<T> {
    pub fn front(&self) -> &T {
        if self.swapped {
            &self.back
        } else {
            &self.front
        }
    }

    pub fn front_mut(&mut self) -> &mut T {
        if self.swapped {
            &mut self.back
        } else {
            &mut self.front
        }
    }

    /// `[front, back]`
    pub fn swap_mut(&mut self) -> [&mut T; 2] {
        let swapped = self.swapped;
        self.swapped = !swapped;

        if swapped {
            [&mut self.back, &mut self.front]
        } else {
            [&mut self.front, &mut self.back]
        }
    }
}
