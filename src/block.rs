// TODO: block states

use bevy::prelude::*;
use enum_map::{EnumMap, enum_map};
use nonmax::NonMaxU16;
use std::{
    ops::{Index, IndexMut},
    sync::LazyLock,
};

use crate::render::*;

pub static BLOCKS: LazyLock<Blocks> = LazyLock::new(temp);

pub struct Block {
    pub liquid: bool,
    pub transparent: bool,
    pub textures: EnumMap<Face, u16>,
}

#[derive(Deref, DerefMut, Default)]
pub struct Blocks(pub Vec<Block>);

impl Index<BlockIndex> for Blocks {
    type Output = Block;

    #[inline]
    fn index(&self, index: BlockIndex) -> &Self::Output {
        &self.0[index.get()]
    }
}

impl IndexMut<BlockIndex> for Blocks {
    #[inline]
    fn index_mut(&mut self, index: BlockIndex) -> &mut Self::Output {
        &mut self.0[index.get()]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockIndex(pub NonMaxU16);

impl BlockIndex {
    #[inline]
    pub fn get(self) -> usize {
        self.0.get() as usize
    }
}

pub const GRASS: BlockIndex = BlockIndex(unsafe { NonMaxU16::new_unchecked(0) });
pub const WATER: BlockIndex = BlockIndex(unsafe { NonMaxU16::new_unchecked(1) });
pub const DIRT: BlockIndex = BlockIndex(unsafe { NonMaxU16::new_unchecked(2) });

pub fn temp() -> Blocks {
    Blocks(vec![
        Block {
            liquid: false,
            transparent: false,
            textures: enum_map! {
                PosX | NegX | PosZ | NegZ => 1,
                PosY => 0,
                NegY => 2,
            },
        },
        Block {
            liquid: true,
            transparent: true,
            textures: enum_map! {
                _ => 3
            },
        },
        Block {
            liquid: false,
            transparent: false,
            textures: enum_map! {
                _ => 2,
            }
        }
    ])
}
