use std::sync::Mutex;
use std::time::{Instant, Duration};

use serde::{Deserialize, Serialize};

use crate::bytes::{BytesCoder, AsFromBytes, cast_vec_from_bytes, cast_bytes_from_slice};
use crate::live_voxel_default_deserialize;
use crate::recipes::item::Item;
use crate::voxels::live_voxels::{LiveVoxel, LiveVoxelNewMultiblock};
use crate::{world::global_coords::GlobalCoords, direction::Direction, voxels::{chunks::Chunks}, recipes::{item::PossibleItem, storage::Storage}};

use super::LiveVoxelDesiarialize;

fn new_instant() -> Instant {Instant::now()}

#[derive(Debug, Serialize, Deserialize)]
pub struct Drill {
    dir: [i8; 3],
    storage: [PossibleItem; 1],
    structure_coordinates: Vec<GlobalCoords>,
    #[serde(skip)]
    #[serde(default = "new_instant")] 
    start: Instant,
}
impl Drill {
    pub const DURATION: Duration = Duration::new(4, 0);
}

live_voxel_default_deserialize!(Mutex<Drill>);

impl LiveVoxelNewMultiblock for Mutex<Drill> {
    fn new_multiblock(direction: &Direction, structure_coordinates: Vec<GlobalCoords>) -> Box<dyn LiveVoxel> {
        Box::new(Mutex::new(Drill {
            storage: [PossibleItem::new_none()],
            structure_coordinates,
            start: Instant::now(),
            dir: direction.simplify_to_one_greatest(true, false, true)
        }))
    }
}

impl LiveVoxel for Mutex<Drill> {
    fn structure_coordinates(&self) -> Option<Vec<GlobalCoords>> {
        Some(self.lock().unwrap().structure_coordinates.clone())
    }

    fn update(&self, _: GlobalCoords, chunks: &Chunks) {
        let mut drill = self.lock().unwrap();
        let xyz = drill.structure_coordinates[0];
        let global = GlobalCoords(xyz.0 - drill.dir[0] as i32, xyz.1, xyz.2-drill.dir[2] as i32);
        if let Some(storage) = chunks.voxel_data(global).and_then(|vd| vd.storage()) {
            if let Some(item) = drill.storage[0].0.take() {
                if let Some(r_item) = storage.lock().unwrap().add(&item, false) {
                    drill.storage[0].try_add_item(&r_item);
                }
            }
        }

        if drill.start.elapsed() < Drill::DURATION {return}
        drill.start = Instant::now();
        
        let mut ores = vec![];
        drill.structure_coordinates.iter().for_each(|coord| {
            let ore_coords = GlobalCoords(coord.0, coord.1-1, coord.2);
            let voxel = chunks.voxel_global(ore_coords);
            let Some(voxel) = voxel else {return};
            if let Some(item) = chunks.content.blocks[voxel.id as usize].ore() {
                ores.push(item);
            }
        });
        ores.into_iter().for_each(|ore| {
            drill.storage[0].try_add_item(&ore);
        });
    }

    fn rotation_index(&self) -> Option<u32> {
        let drill = self.lock().unwrap();
        if drill.dir[2] > 0 {return Some(0)};
        if drill.dir[0] < 0 {return Some(3)};
        if drill.dir[2] < 0 {return Some(2)};
        Some(1)
    }

    fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}



impl Storage for Drill {
    fn storage(&self) -> &[PossibleItem] {
        &self.storage
    }

    fn mut_storage(&mut self) -> &mut [PossibleItem] {
        &mut self.storage
    }
}