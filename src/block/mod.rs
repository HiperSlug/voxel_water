#![allow(dead_code)]

// TODO: block states

// use arc_swap::ArcSwap;
use bevy::prelude::*;
use enum_map::{EnumMap, enum_map};
use nonmax::NonMaxU16;
use std::ops::{Index, IndexMut};
// use std::sync::Arc;

use crate::render::Face;

// pub static BLOCKS: ArcSwap<Blocks> = ArcSwap::new(Arc::new(Blocks::temp()));

pub struct Block {
    pub liquid: bool,
    pub textures: EnumMap<Face, u16>,
}

#[derive(Deref, DerefMut, Default)]
pub struct Blocks(pub Vec<Block>);

impl Blocks {
    fn temp() -> Self {
        Self(vec![
            Block {
                liquid: false,
                textures: enum_map! {
                    _ => 0
                },
            },
            Block {
                liquid: true,
                textures: enum_map! {
                    _ => 1
                },
            },
        ])
    }
}

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
