use crate::{world::{World, global_xyz::GlobalXYZ}, player::player::Player, direction::Direction};

use super::{interaction::BlockInteraction, block_type::BlockType, light_permeability::LightPermeability};


pub struct MultiBlock {
    pub id: u32,
    pub emission: [u8; 3],
    pub light_permeability: LightPermeability,
    pub block_type: BlockType,
    pub is_additional_data: bool,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
}


impl BlockInteraction for MultiBlock {
    fn id(&self) -> u32 {self.id}
    fn emission(&self) -> &[u8; 3] {&self.emission}
    fn light_permeability(&self) -> LightPermeability {self.light_permeability}
    fn block_type(&self) -> &BlockType {&self.block_type}
    fn is_additional_data(&self) -> bool {self.is_additional_data}

    fn is_multiblock(&self) -> bool {true}
    fn width(&self) -> usize {self.width}
    fn height(&self) -> usize {self.height}
    fn depth(&self) -> usize {self.depth}

    fn on_block_break(&self, world: &mut World, _: &mut Player, xyz: &GlobalXYZ) {
        if let Some(xyz) = world.chunks.remove_multiblock_structure(xyz.0, xyz.1, xyz.2) {
            xyz.iter().for_each(|(x, y, z)| {
                world.light.on_block_break(&mut world.chunks, *x, *y, *z);
            });
        };
    }

    fn on_block_set(&self, world: &mut World, _: &mut Player, xyz: &GlobalXYZ, dir: &Direction) -> bool {
        // FIX THIS SHIT
        let mut width = self.width as i32;
        let mut depth = self.depth as i32;
        if self.id() == 15 {
            let d = dir.simplify_to_one_greatest(true, false, true);
            if d[2] < 0 {width = -(self.width as i32)};
            if d[2] < 0 {depth = -(self.depth as i32)};
            if d[0] > 0 {depth = -(self.depth as i32)};
            if d[0] < 0 {width = -(self.width as i32)};
        }
        
        let coords = world.chunks
            .add_multiblock_structure(xyz, width, self.height as i32, depth, self.id(), dir);
        if let Some(coords) = coords {
            coords.iter().for_each(|(x, y, z)| {
                world.light.on_block_set(&mut world.chunks, *x, *y, *z, self.id());
            });
            return true;
        }
        false
    }
}