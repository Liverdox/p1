use std::{cell::UnsafeCell, collections::VecDeque, i32, ptr::null, sync::Arc};

use crate::{content::Content, voxels::{new_chunk::{Chunk, LocalCoord}, new_chunks::{ChunkCoord, Chunks, GlobalCoord, WORLD_BLOCK_HEIGHT}}};

/// It's very unsafe, but very fast.
/// May issue STATUS_HEAP_CORRUPTION during relocation.
#[derive(Debug)]
pub struct LightQueue(UnsafeCell<VecDeque<LightEntry>>);

impl LightQueue {
    #[inline(always)]
    /// Safety
    /// If in any way the amount of light exceeds the capacity, we are fucked.
    pub fn new(capacity: usize) -> LightQueue {
        Self(UnsafeCell::new(VecDeque::<LightEntry>::with_capacity(capacity)))
    }

    #[inline(always)]
    pub fn push(&self, light: LightEntry) {
        unsafe {&mut *self.0.get()}.push_back(light);
    }

    #[inline(always)]
    pub fn pop(&self) -> Option<LightEntry> {
        unsafe {&mut *self.0.get()}.pop_front()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LightEntry {
    pub coords: GlobalCoord,
    pub light: u8,
}

impl LightEntry {
    #[inline]
    pub fn new(coords: GlobalCoord, light: u8) -> LightEntry {
        LightEntry { coords, light }
    }
}

pub struct ChunkBuffer(ChunkCoord, Option<Arc<Chunk>>);
impl ChunkBuffer {
    #[inline]
    pub fn new() -> Self {Self::default()}

    #[inline]
    pub unsafe fn get_or_init(&mut self, chunks: &Chunks, coords: GlobalCoord) -> Option<Arc<Chunk>> {
        let cc: ChunkCoord = coords.into();
        if self.0 == cc {
            self.1.clone()
        } else {
            let chunk = chunks.chunk(cc)?.clone();
            self.0 = cc;
            self.1 = Some(chunk);
            self.1.clone()
        }
    }
}

impl Default for ChunkBuffer {
    #[inline]
    fn default() -> Self {Self(ChunkCoord::new(i32::MAX, i32::MAX), None)}
}

const NEIGHBOURS: [GlobalCoord; 6] = [
    GlobalCoord::new(1,  0, 0),
    GlobalCoord::new(-2, 0, 0),
    GlobalCoord::new(1, 1, 0),
    GlobalCoord::new(0, -2, 0),
    GlobalCoord::new(0, 1, 1),
    GlobalCoord::new(0, 0, -2),
];

#[derive(Debug)]
pub struct LightSolver {
    add_queue: LightQueue,
    remove_queue: LightQueue,
    channel: usize,
}

unsafe impl Send for LightSolver {}
unsafe impl Sync for LightSolver {}

impl LightSolver {
    /// Safety
    /// If in any way the amount of light exceeds the capacity, we are fucked.
    pub fn new(channel: usize, add_queue_cap: usize, remove_queue_cap: usize) -> LightSolver {
        LightSolver {
            add_queue: LightQueue::new(add_queue_cap),
            remove_queue: LightQueue::new(remove_queue_cap),
            channel
        }
    }


    pub fn add_with_emission(&self, chunks: &Chunks, x: i32, y: i32, z: i32, emission: u8) {
        if emission <= 1 {return};
        
        let global = GlobalCoord::new(x, y, z);
        let Some(chunk) = chunks.chunk(global.into()) else {return};

        let entry = LightEntry::new(global, emission);
        let local = LocalCoord::from(global);
        chunk.light_map()[local].set(emission, self.channel);
        chunk.modify(true);
        
        self.add_queue.push(entry);
    }

    pub fn add(&self, chunks: &Chunks, x: i32, y: i32, z: i32) {
        let emission = chunks.light((x,y,z).into(), self.channel);
        self.add_with_emission(chunks, x, y, z, emission);
    }


    pub fn remove(&self, chunks: &Chunks, x: i32, y: i32, z: i32) {
        let global = GlobalCoord::new(x, y, z);
        let Some(chunk) = chunks.chunk(global.into()) else {return};

        let index = LocalCoord::from(global).index();

        let light = unsafe {chunk.lightmap.0.get_unchecked(index).get_unchecked(self.channel)};
        unsafe {chunk.lightmap.0.get_unchecked(index).set_unchecked(0, self.channel)};

        self.remove_queue.push(LightEntry::new(global, light));
    }


    pub fn solve(&self, chunks: &Chunks, content: &Content) {
        self.solve_remove_queue(chunks);
        self.solve_add_queue(chunks, content);
    }

    fn solve_remove_queue(&self, chunks: &Chunks) {
        let mut chunk_buffer = ChunkBuffer::new();
        while let Some(mut entry) = self.remove_queue.pop() {
            let entry_prev_light = entry.light;
            for offsets in NEIGHBOURS.iter() {
                entry.coords += offsets;

                let Some(chunk) = (unsafe {chunk_buffer.get_or_init(chunks, entry.coords)}) else {continue};
                let index = LocalCoord::from(entry.coords).index();

                entry.light = unsafe {chunk.lightmap.0[index]
                    .get_unchecked(self.channel)};

                if entry.light != 0 && entry_prev_light != 0 && entry.light == entry_prev_light - 1 {
                    self.remove_queue.push(entry);
                    unsafe {chunk.lightmap.0[index]
                        .set_unchecked(0, self.channel)};
                    chunk.modify(true);
                } else if entry.light >= entry_prev_light {
                    self.add_queue.push(entry);
                }
            }
        }
    }

    fn solve_add_queue(&self, chunks: &Chunks, content: &Content) {
        let mut chunk_buffer = ChunkBuffer::new();
        while let Some(mut entry) = self.add_queue.pop() {
            if entry.light <= 1 { continue; }
            let prev_light = entry.light;
            entry.light -= 1;
            for offsets in NEIGHBOURS.iter() {
                entry.coords += offsets;
                if entry.coords.y < 0 || entry.coords.y >= WORLD_BLOCK_HEIGHT as i32 {continue}

                let Some(chunk) = (unsafe {chunk_buffer.get_or_init(chunks, entry.coords)}) else {continue};
                let index = LocalCoord::from(entry.coords).index();

                let light = unsafe {chunk.lightmap.0[index]
                    .get_unchecked(self.channel)};
                let id = unsafe {chunk.voxels.0[index].id()};

                if content.blocks[id as usize].is_light_passing() && (light+2) <= prev_light {
                    self.add_queue.push(entry);
                    unsafe {chunk.lightmap.0.get_unchecked(index)
                        .set_unchecked(entry.light, self.channel)};
                    chunk.modify(true);
                }
            }
        }
    }
}