use std::{collections::HashMap, sync::*};
use spacetimedb::{table, ReducerContext, Table, SpacetimeType};

#[derive(Debug)]
pub struct BlocksHandler {
    values: Vec<Arc<Block>>,
    names: HashMap<String, Arc<Block>>
}

static VALUE: OnceLock<RwLock<BlocksHandler>> = OnceLock::new();

impl BlocksHandler {
    pub fn new(ctx: &ReducerContext) -> Self {
        let values: Vec<Arc<Block>> = ctx.db.block().iter().map(|b| Arc::new(b)).collect();
        let names = HashMap::from_iter(values.iter().map(|v| (v.name.clone(), v.clone())));

        Self { values, names }
    }

    pub fn init(ctx: &ReducerContext) {
        VALUE.set(RwLock::new(Self::new(ctx))).unwrap();
    }

    pub fn get() -> &'static RwLock<Self> {
        VALUE.get().unwrap()
    }

    pub fn find_block(&self, name: &str) -> u16 {
        self.names.get(name).cloned()
            .and_then(|b| Some(b.id)).unwrap_or(0)
    }

    pub fn block(&self, id: u16) -> Option<Arc<Block>> {
        self.values.get(id as usize).cloned()
    }
}

#[derive(SpacetimeType)]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
// Model type with texture file name
pub enum ModelType {
    Empty,
    Cube(String),
    Stair(String),
    Slab(String),
}

impl ModelType {
    pub fn is_meshable(&self) -> bool {
        match self {
            Self::Cube(_) => true,
            // WIP: other meshable blocks
            _ => false
        }
    }
}

// Block type table
#[table(name = block, public)]
#[derive(Debug)]
pub struct Block {
    #[primary_key]
    pub id: u16,
    // block name
    #[unique]
    pub name: String,
    // Texture path and model
    pub model: ModelType,
    // light?
    // collision? todo
}

impl Block {
    pub fn is_meshable(&self) -> bool {
        self.model.is_meshable()
    }
}