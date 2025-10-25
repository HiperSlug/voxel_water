use super::*;

pub const PAD_MASK: u64 = (1 << 63) | 1;

#[derive(Clone)]
pub struct Masks {
    pub some_mask: [u64; AREA],
    pub liquid_mask: [u64; AREA],
}

impl Default for Masks {
    fn default() -> Self {
        Self {
            some_mask: [0; AREA],
            liquid_mask: [0; AREA],
        }
    }
}

impl Masks {
    pub fn set(&mut self, p: impl Index3d, v: Option<Voxel>) {
        let (x, i_2d) = p.x_and_i_2d();
        let mask = 1 << x;

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i_2d] |= mask;
                self.liquid_mask[i_2d] |= mask;
            }
            Some(_) => {
                self.some_mask[i_2d] |= mask;
                self.liquid_mask[i_2d] &= !mask;
            }
            None => {
                self.some_mask[i_2d] &= !mask;
                self.liquid_mask[i_2d] &= !mask;
            }
        }
    }

    pub fn fill_row(&mut self, p: impl Index2d, v: Option<Voxel>) {
        let i = p.i_2d();

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] = u64::MAX;
                self.liquid_mask[i] = u64::MAX;
            }
            Some(_) => {
                self.some_mask[i] = u64::MAX;
                self.liquid_mask[i] = 0;
            }
            None => {
                self.some_mask[i] = 0;
                self.liquid_mask[i] = 0;
            }
        }
    }

    pub fn set_row_padding(&mut self, p: impl Index2d, v: Option<Voxel>) {
        let i = p.i_2d();

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] |= PAD_MASK;
                self.liquid_mask[i] |= PAD_MASK;
            }
            Some(_) => {
                self.some_mask[i] |= PAD_MASK;
                self.liquid_mask[i] &= !PAD_MASK;
            }
            None => {
                self.some_mask[i] &= !PAD_MASK;
                self.liquid_mask[i] &= !PAD_MASK;
            }
        }
    }

    pub fn is_some(&self, p: impl Index3d) -> bool {
        let (x, i_2d) = p.x_and_i_2d();

        self.some_mask[i_2d] & (1 << x) != 0
    }
}
