use fastnoise_lite::*;
use bevy_tasks::*;
use std::sync::*;
use super::*;


/// World gen context
/// todo: load data from config
pub struct Context {
    noise: FastNoiseLite,
}

static CONTEXT: OnceLock<RwLock<Context>> = OnceLock::new();

impl Context {
    pub fn new(seed: i32) -> Self {
        Self { noise: FastNoiseLite::with_seed(seed) }
    }

    pub fn init(seed: i32) {
        let _ = CONTEXT.set(RwLock::new(Self::new(seed)));
    }

    pub fn get() -> &'static RwLock<Self> {
        CONTEXT.get().unwrap()
    }
}

#[derive(Debug)]
pub struct Generator {
    pub queue: Vec<IVec3>,
    pub tasks: HashMap<IVec3, Task<Chunk>>
}

static VALUE: OnceLock<RwLock<Generator>> = OnceLock::new();

impl Generator {
    pub const MAX_TASKS: usize = 32;

    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            tasks: HashMap::new(),
        }
    }

    pub fn init() {
        VALUE.set(RwLock::new(Self::new())).unwrap();
    }

    pub fn get() -> &'static RwLock<Self> {
        VALUE.get().unwrap()
    }

    pub async fn generate(position: IVec3) -> Chunk {
        let _context = Context::get().read().unwrap();
        let blocks = BlocksHandler::get().read().unwrap();
        let mut chunk = Chunk::new(position);

        // WIP: dynamic world generation
        let range = 8;
        if position.x > range 
            || position.x < -range 
            || position.y > range 
            || position.y < -range 
            || position.z > range 
            || position.z < -range { return chunk; }

        let vals = ["air", "dirt", "grass", "stone"];
        
        // todo: generate chunk
        if position.y == 0 {
            for i in 0..SIZE_P3 {
                let name = fastrand::choice(vals).unwrap();
                chunk.set_block(i, blocks.find_block(name));
            }
        }
        
        chunk
    }

    // Generator load stats
    pub fn load() -> (u32, u32) {
        let access = Self::get().read().unwrap();
        (access.tasks.len() as u32, access.queue.len() as u32)
    }
}