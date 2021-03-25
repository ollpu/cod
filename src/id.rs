use std::sync::atomic::{AtomicU64, Ordering};

pub type ID = u64;

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_id() -> ID {
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}


