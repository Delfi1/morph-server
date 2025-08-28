use std::{
    collections::*,
    sync::*,
};
use include_directory::{include_directory, Dir};
use spacetimedb::{table, ReducerContext, Table};
use super::math::*;

mod blocks;
pub use blocks::*;

mod generate;

pub(super) static SCHEME_DIR: Dir<'static> = include_directory!("./schema");

#[derive(Debug, Default)]
// Current loaded chunks
pub struct LoadArea(RwLock<HashMap<IVec3, Arc<Chunk>>>);
static VALUE: OnceLock<LoadArea> = OnceLock::new();

impl LoadArea {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init() {
        VALUE.set(Self::new()).unwrap();
    }

    pub fn value() -> &'static Self {
        VALUE.get().unwrap()
    }

    pub fn insert(pos: IVec3, chunk: Arc<Chunk>) {
        let value = Self::value();
        let mut access = value.0.write().unwrap();

        access.insert(pos, chunk);
    }

    pub fn remove(pos: &IVec3) {
        let value = Self::value();
        let mut access = value.0.write().unwrap();

        access.remove(pos);
    }

    pub fn get(pos: &IVec3) -> Option<Arc<Chunk>> {
        let value = Self::value();
        let access = value.0.read().unwrap();

        access.get(pos).cloned()
    }
}

// Chunk constants
pub const SIZE: usize = 16;
pub const SIZE_I32: i32 = SIZE as i32;
pub const SIZE_P3: usize = SIZE.pow(3);

// Block size in bytes and chunk buffer (data) size
pub const BLOCK_SIZE: usize = 12;
pub const BYTE: usize = 8;

// How much bytes
pub const BUF_SIZE: usize = SIZE_P3 * BLOCK_SIZE / BYTE;

#[table(name = chunk, public)]
#[derive(Debug)]
pub struct Chunk {
    #[auto_inc]
    #[primary_key]
    id: u64,

    #[unique]
    pub position: StIVec3,
    // Compressed chunk data
    pub data: Vec<u8>
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        let position = position.into();
        let data = std::iter::repeat_n(0, BUF_SIZE).collect();
        Self { id: 0, position, data }
    }

    pub fn get_block(&self, index: usize) -> u16 {
        let result = 0u16;

        todo!("get block by id");

        result
    }

    pub fn set_block(&mut self, index: usize, value: u16) {
        let mut i = index * 2 * BYTE / BLOCK_SIZE;
        if index % 2 != 0 { i -= 1 };
        let (a, b) = (self.data[i], self.data[i+1]);

        
        todo!("set block by id");
    }

    /// XZY coord system
    pub fn block_index(pos: IVec3) -> usize {
        let x = pos.x % SIZE_I32;
        let z = pos.z * SIZE_I32;
        let y = pos.y * SIZE_I32.pow(2);

        (x + y + z) as usize
    }
}

#[repr(transparent)]
/// Contains all near chunks:
/// 
/// Current; Left; Right; Down; Up; Back; Forward;
pub struct ChunksRefs([Arc<Chunk>; 7]);

impl ChunksRefs {
    // Array of chunk neighbours positions
    pub const OFFSETS: [IVec3; 7] = [
        IVec3::ZERO,  // current
        IVec3::NEG_Y, // down
        IVec3::Y,     // up
        IVec3::NEG_X, // left
        IVec3::X,     // right
        IVec3::NEG_Z, // forward
        IVec3::Z,     // back
    ];

    // Helper function: create an array from Vec
    fn to_array<T: std::fmt::Debug, const N: usize>(data: Vec<T>) -> [T; N] {
        data.try_into().expect("Wrong size")
    }

    // Helper function: get chunk from BD
    pub fn get_chunk(position: IVec3) -> Option<Arc<Chunk>> {
        LoadArea::get(&position)
    }

    // Create chunk refs
    pub fn new(pos: IVec3) -> Option<Self> {
        let mut data = Vec::<Arc<Chunk>>::with_capacity(7);
        for n in 0..7 {
            data.push(Self::get_chunk(pos + ChunksRefs::OFFSETS[n])?)
        }

        Some(Self(Self::to_array(data)))
    }

    fn offset_index(v: IVec3) -> usize {
        Self::OFFSETS.iter().position(|p| p==&v).unwrap()
    }

    fn chunk_index(x: usize, y: usize, z: usize) -> usize {
        let (cx, cy, cz) = (
            (x / SIZE) as i32,
            (y / SIZE) as i32, 
            (z / SIZE) as i32
        );
        
        Self::offset_index(IVec3::new(cx, cy, cz) - IVec3::ONE)
    }
    
    fn block_index(x: usize, y: usize, z: usize) -> usize {
        let (bx, by, bz) = (
            (x % SIZE) as i32,
            (y % SIZE) as i32,
            (z % SIZE) as i32
        );

        Chunk::block_index(IVec3::new(bx, by, bz))
    }

    pub fn get_block(&self, pos: IVec3) -> u16 {
        let x = (pos.x + SIZE_I32) as usize;
        let y = (pos.y + SIZE_I32) as usize;
        let z = (pos.z + SIZE_I32) as usize;
        let chunk = Self::chunk_index(x, y, z);
        let block = Self::block_index(x, y, z);

        self.0[chunk].get_block(block)
    }
}

