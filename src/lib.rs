use bevy_tasks::*;
use spacetimedb::{
    reducer, table, ReducerContext, Table,
    ScheduleAt, TimeDuration, Timestamp
};

// Morph modules
mod math;
mod chunks;
mod mesher;
mod player;

// Ticks per second
pub const TIPS: i64 = 20;
pub const TICK: i64 = 1_000_000 / TIPS;

// Tasks per second
pub const TAPS: i64 = 5;
pub const TASK: i64 = 1_000_000 / TAPS;

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    AsyncComputeTaskPool::get_or_init(|| TaskPool::new());

    // Tasks proceed schedule loop
    ctx.db.tasks().insert(TasksSchedule {
        scheduled_id: 0,
        scheduled_at: TimeDuration::from_micros(TASK).into(),
    });

    // Main ticks loop
    ctx.db.ticks().insert(Ticks {
        id: 0,
        scheduled_at: TimeDuration::from_micros(TICK).into(),
        previous: ctx.timestamp,
        tickrate: 0.0,
        tick: 0
    });

    // todo: setup main server components
}

#[table(name = tasks, scheduled(proceed_tasks))]
pub struct TasksSchedule {
    #[auto_inc]
    #[primary_key]
    scheduled_id: u64,
    scheduled_at: ScheduleAt
}

#[reducer]
fn proceed_tasks(ctx: &ReducerContext, _: TasksSchedule) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Task pool may not be invoked only via scheduling.".into());
    }

    AsyncComputeTaskPool::get()
        .with_local_executor(|executor| { executor.try_tick() });

    Ok(())
}

#[table(name = ticks, scheduled(run_tick), public)]
pub struct Ticks {
    #[primary_key]
    pub id: u64,
    pub scheduled_at: ScheduleAt,

    previous: Timestamp,
    pub tickrate: f64,
    pub tick: u128
}

#[reducer]
fn run_tick(ctx: &ReducerContext, mut arg: Ticks) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Tick may not be invoked only via scheduling.".into());
    }

    // Begin tick
    arg.tick += 1;
    let delta = ctx.timestamp.duration_since(arg.previous).unwrap();
    arg.tickrate = 1.0 / delta.as_secs_f64();
    arg.previous = ctx.timestamp;

    // Run generator tasks
    //chunks::proceed_generator(ctx);

    // Run mesher tasks
    //mesher::proceed_mesher(ctx);

    ctx.db.ticks().id().update(arg);
    Ok(())
}

