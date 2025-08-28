use spacetimedb::rand::Rng;
use fastnoise_lite::*;
use std::sync::*;
use super::*;

/// World gen context
/// todo: load data from config
pub struct Generator {
    noise: FastNoiseLite
}

static VALUE: OnceLock<RwLock<Generator>> = OnceLock::new();

impl Generator {
    pub fn new(seed: i32) -> Self {
        let noise = FastNoiseLite::with_seed(seed);

        Self { noise }
    }

    pub fn init(seed: i32) {
        let _ = VALUE.set(RwLock::new(Self::new(seed)));
    }

    pub fn value() -> &'static RwLock<Self> {
        VALUE.get().unwrap()
    }

}

// Get block by name
pub fn find_block(ctx: &ReducerContext, name: impl Into<String>) -> u16 {
    ctx.db.block().name().find(&name.into())
        .and_then(|b| Some(b.id)).unwrap_or(0)
}

pub fn generate_chunk(ctx: &ReducerContext, pos: IVec3) -> Chunk {
    // WIP: dynamic world size
    let mut chunk = Chunk::new(pos);

    let range = 4;
    if pos.x > range || pos.x < -range || pos.y > range || pos.y < -range || pos.z > range || pos.z < -range {
        return ctx.db.chunk().insert(chunk);
    }

    let vals = ["air", "dirt", "grass", "stone"];
    let l = vals.len();

    if pos.y == 0 {
        for i in 0..SIZE.pow(3) {
            let rand = ctx.rng().gen_range(0..l);

            chunk.set_block(i, find_block(ctx, vals[rand]));
        }
    }
    
    ctx.db.chunk().insert(chunk)
}
