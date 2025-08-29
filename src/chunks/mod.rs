use std::{
    collections::*,
    sync::*,
};
use include_directory::{include_directory, Dir};
use spacetimedb::{table, ReducerContext, Table};
use bevy_tasks::*;

mod blocks;
pub use blocks::*;

mod generate;
pub use generate::*;

use super::math::*;

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
pub const HALF_BYTE: usize = BYTE / 2;

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
        let i = index * BLOCK_SIZE / BYTE;
        let (a, b) = match index % 2 == 0 {
            // First and second bytes
            // 0110_0001 1001_0010 1110_1000 => 0110_0001 and 1001    
            true => (self.data[i] as u16, (self.data[i+1] >> HALF_BYTE) as u16),
            // Second and third bytes
            // 0110_0001 1001_0010 1110_1000 => 0010 and 1110_1000
            false => ((self.data[i] << HALF_BYTE) as u16, self.data[i+1] as u16)
        };
        
        a << HALF_BYTE | b
    }

    pub fn set_block(&mut self, index: usize, value: u16) {
        let i = index * BLOCK_SIZE / BYTE;
        match index % 2 == 0 {
            true => {
                // 0000_0101_1100_0011 => 0101_1100 and 0011
                let (a, b) = ((value >> HALF_BYTE) as u8, (value & 0b1111) as u8);

                self.data[i] = a;
                self.data[i+1] = (self.data[i+1] & 0b0000_1111) | (b << HALF_BYTE);
            },
            false => {
                // 0000_0101_1100_0011 => 0101 and 1100_0011
                let (a, b) = ((value >> BYTE) as u8, (value & 0b1111_1111) as u8);

                self.data[i] = (self.data[i] & 0b1111_0000) | a;
                self.data[i+1] = b;
            }
        }
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

pub fn proceed_generator(ctx: &ReducerContext) {
    let task_pool = AsyncComputeTaskPool::get();
    let mut generator = Generator::get().write().unwrap();

    for (pos, task) in generator.tasks.drain().collect::<Vec<_>>() {
        if !task.is_finished() {
            generator.tasks.insert(pos, task);
            continue;
        }

        let chunk = block_on(task);
        log::info!("Generated chunk: {}", pos);
        LoadArea::insert(pos, Arc::new(ctx.db.chunk().insert(chunk)));
    }

    let l = generator.queue.len().min(Generator::MAX_TASKS - generator.tasks.len());
    for pos in generator.queue.drain(0..l).collect::<Vec<IVec3>>() {
        let task = task_pool.spawn(Generator::generate(pos));
        generator.tasks.insert(pos, task);
    }
}

pub fn init_blocks(ctx: &ReducerContext) {
    // clear blocks data
    for block in ctx.db.block().iter() {
        ctx.db.block().id().delete(block.id);
    }

    let blocks_file = SCHEME_DIR.get_file("blocks.json")
        .expect("Blocks data file is not found");
    
    let blocks: Vec<(String, ModelType)> = blocks_file.contents_utf8()
        .and_then(|data| serde_json::from_str(data).ok())
        .expect("Blocks data file parse error");

    for (id, (name, model)) in blocks.into_iter().enumerate() {
        let id = id as u16;
        ctx.db.block().insert(Block { id, name, model });
    }

    BlocksHandler::init(ctx);
}

// Init main values and world area
pub fn setup(ctx: &ReducerContext) {
    Context::init(0);
    Generator::init();
    LoadArea::init();
    init_blocks(ctx);

    let range = 10;
    let l = ((range*2)+1) as usize;
    let mut area = Vec::with_capacity(l.pow(3));

    for x in -range..=range {
        for y in -range..=range {
            for z in -range..=range {
                area.push(ivec3(x, y, z));
            }
        }
    }

    let mut generator = Generator::get().write().unwrap();
    let mut mesher = super::mesher::Mesher::get().write().unwrap();
    
    for pos in area {
        generator.queue.push(pos);
        mesher.queue.push(pos);
    }
}