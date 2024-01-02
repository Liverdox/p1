use std::{sync::{Mutex, Arc, mpsc::Sender}, time::{Duration, Instant}, thread::{self, JoinHandle}};

use crate::{world::{World, chunk_coords::ChunkCoords, global_coords::GlobalCoords}, graphic::render::{RenderResult, render}, unsafe_mutex::UnsafeMutex, WORLD_EXIT};

pub fn spawn(
    world: Arc<UnsafeMutex<World>>,
    sender: Sender<RenderResult>,
    render_result: Arc<Mutex<Option<RenderResult>>>
) -> JoinHandle<()> {
    let mut results = Vec::<RenderResult>::new();
    let b = thread::Builder::new().name("renderer".to_owned()).stack_size(32 * 1024 * 1024);
    b.spawn(move || {loop {
        if unsafe { WORLD_EXIT } {break};
        let mut world = unsafe {world.lock_unsafe()}.unwrap();

        let ox = world.chunks.ox;
        let oz = world.chunks.oz;
        let width = world.chunks.width;
        let depth = world.chunks.depth;
        
        if let Some(chunk) = world.chunks.find_unrendered() {
            chunk.modify(false);
            if let Some(result) = render(chunk.xyz.nindex(width, depth, ox, oz), &world) {
                let _ = sender.send(result);
            }
        } else {
            drop(world);
            thread::sleep(Duration::from_millis(16));
        }
    }}).unwrap()
}