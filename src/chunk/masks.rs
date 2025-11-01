use bevy::prelude::*;

use super::double_buffered::DoubleBuffered;
use super::index::{Index2d, Index3d};
use super::{DEFAULT_MASK, Mask, Voxel};

pub const PAD_MASK: u64 = (1 << 63) | 1;

#[derive(Clone)]
pub struct LiquidTickMasks {
    pub some_mask: Mask,
    pub liquid_mask: Mask,
}

impl Default for LiquidTickMasks {
    fn default() -> Self {
        Self {
            some_mask: DEFAULT_MASK,
            liquid_mask: DEFAULT_MASK,
        }
    }
}

pub struct Masks {
    pub dblt_masks: DoubleBuffered<LiquidTickMasks>,
    pub transparent_mask: Mask,
}

impl Default for Masks {
    fn default() -> Self {
        Self {
            dblt_masks: default(),
            transparent_mask: DEFAULT_MASK,
        }
    }
}

impl Masks {
    #[inline]
    pub fn some_mask(&self) -> &Mask {
        &self.dblt_masks.front.some_mask
    }

    #[inline]
    pub fn set(&mut self, p: impl Index3d, v: Option<Voxel>) {
        let (x, i_2d) = p.x_and_i_2d();
        let bit = 1 << x;

        match v {
            Some(Voxel::Liquid) => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] |= bit;
                    lt_masks.liquid_mask[i_2d] |= bit;
                });
                self.transparent_mask[i_2d] |= bit;
            }
            Some(_) => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] |= bit;
                    lt_masks.liquid_mask[i_2d] &= !bit;
                });
                self.transparent_mask[i_2d] &= !bit;
            }
            None => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] &= !bit;
                    lt_masks.liquid_mask[i_2d] &= !bit;
                });
                self.transparent_mask[i_2d] &= !bit;
            }
        }
    }

    #[inline]
    pub fn fill_row(&mut self, p: impl Index2d, v: Option<Voxel>) {
        let i_2d = p.i_2d();

        match v {
            Some(Voxel::Liquid) => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] = !0;
                    lt_masks.liquid_mask[i_2d] = !0;
                });
                self.transparent_mask[i_2d] = !0;
            }
            Some(_) => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] = !0;
                    lt_masks.liquid_mask[i_2d] = 0;
                });
                self.transparent_mask[i_2d] = 0;
            }
            None => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] = 0;
                    lt_masks.liquid_mask[i_2d] = 0;
                });
                self.transparent_mask[i_2d] = 0;
            }
        }
    }

    #[inline]
    pub fn set_row_padding(&mut self, p: impl Index2d, v: Option<Voxel>) {
        let i_2d = p.i_2d();

        match v {
            Some(Voxel::Liquid) => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] |= PAD_MASK;
                    lt_masks.liquid_mask[i_2d] |= PAD_MASK;
                });
                self.transparent_mask[i_2d] |= PAD_MASK;
            }
            Some(_) => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] |= PAD_MASK;
                    lt_masks.liquid_mask[i_2d] &= !PAD_MASK;
                });
                self.transparent_mask[i_2d] &= !PAD_MASK;
            }
            None => {
                self.dblt_masks.for_each(|lt_masks| {
                    lt_masks.some_mask[i_2d] &= !PAD_MASK;
                    lt_masks.liquid_mask[i_2d] &= !PAD_MASK;
                });
                self.transparent_mask[i_2d] &= !PAD_MASK;
            }
        }
    }

    #[inline]
    pub fn is_some(&self, p: impl Index3d) -> bool {
        let (x, i_2d) = p.x_and_i_2d();

        let bit = 1 << x;
        self.dblt_masks.front.some_mask[i_2d] & bit != 0
    }
}
