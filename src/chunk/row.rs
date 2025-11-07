// TODO: runtime enumeration

use super::Voxel;

pub const PAD_MASK: u64 = (1 << 63) | 1;

#[derive(Default, Clone, Copy)]
pub struct RowMasks {
    pub occupied: u64,
    // liquid simulation
    pub liquid: u64,
    // meshing
    pub opaque: u64,
    pub transparent: u64,
}

impl RowMasks {
    #[inline]
    pub fn clear(&mut self, x: u32) {
        self.occupied.set_bit(x, false);
        self.liquid.set_bit(x, false);
        self.opaque.set_bit(x, false);
        self.transparent.set_bit(x, false);
    }

    #[inline]
    pub fn set(&mut self, x: u32, v: Option<Voxel>) {
        match v {
            Some(Voxel::Liquid) => {
                self.occupied.set_bit(x, true);
                self.liquid.set_bit(x, true);
                self.opaque.set_bit(x, false);
                self.transparent.set_bit(x, true);
            }
            Some(_) => {
                self.occupied.set_bit(x, true);
                self.liquid.set_bit(x, false);
                self.opaque.set_bit(x, true);
                self.transparent.set_bit(x, false);
            }
            None => {
                self.clear(x);
            }
        }
    }

    #[inline]
    pub fn fill(&mut self, v: Option<Voxel>) {
        match v {
            Some(Voxel::Liquid) => {
                self.occupied = !0;
                self.liquid = !0;
                self.opaque = 0;
                self.transparent = !0;
            }
            Some(_) => {
                self.occupied = !0;
                self.liquid = 0;
                self.opaque = !0;
                self.transparent = 0;
            }
            None => {
                self.occupied = 0;
                self.liquid = 0;
                self.opaque = 0;
                self.transparent = 0;
            }
        }
    }

    #[inline]
    pub fn fill_padding(&mut self, v: Option<Voxel>) {
        match v {
            Some(Voxel::Liquid) => {
                self.occupied.set_mask(PAD_MASK, true);
                self.liquid.set_mask(PAD_MASK, true);
                self.opaque.set_mask(PAD_MASK, false);
                self.transparent.set_mask(PAD_MASK, true);
            }
            Some(_) => {
                self.occupied.set_mask(PAD_MASK, true);
                self.liquid.set_mask(PAD_MASK, false);
                self.opaque.set_mask(PAD_MASK, true);
                self.transparent.set_mask(PAD_MASK, false);
            }
            None => {
                self.occupied.set_mask(PAD_MASK, false);
                self.liquid.set_mask(PAD_MASK, false);
                self.opaque.set_mask(PAD_MASK, false);
                self.transparent.set_mask(PAD_MASK, false);
            }
        }
    }

    #[inline]
    pub fn is_occupied(&self, x: u32) -> bool {
        self.occupied.get_bit(x)
    }
}

trait BitOps {
    fn set_bit(&mut self, bit: u32, value: bool);

    fn set_mask(&mut self, mask: Self, value: bool);

    fn get_bit(self, bit: u32) -> bool;
}

impl BitOps for u64 {
    #[inline]
    fn set_bit(&mut self, bit: u32, value: bool) {
        let mask = 1 << bit;
        self.set_mask(mask, value)
    }

    #[inline]
    fn set_mask(&mut self, mask: Self, value: bool) {
        if value {
            *self |= mask;
        } else {
            *self &= !mask;
        }
    }

    #[inline]
    fn get_bit(self, bit: u32) -> bool {
        let mask = 1 << bit;
        self & mask != 0
    }
}
