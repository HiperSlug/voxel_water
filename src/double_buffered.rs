#[derive(Debug, Default)]
pub struct DoubleBuffered<T> {
    buffers: [T; 2],
    /// false => [[front, back]], \
    /// true => [[back, front]],
    swapped: bool,
}

impl<T> DoubleBuffered<T> {
    pub fn front(&self) -> &T {
        if self.swapped {
            &self.buffers[1]
        } else {
            &self.buffers[0]
        }
    }

    pub fn front_mut(&mut self) -> &mut T {
        if self.swapped {
            &mut self.buffers[1]
        } else {
            &mut self.buffers[0]
        }
    }

    /// [[front, back]]
    pub fn buffers_mut(&mut self) -> [&mut T; 2] {
        let (left, right) = self.buffers.split_at_mut(1);
        if self.swapped {
            [&mut right[0], &mut left[0]]
        } else {
            [&mut left[0], &mut right[0]]
        }
    }

    pub fn swap(&mut self) {
        self.swapped = !self.swapped;
    }
}
