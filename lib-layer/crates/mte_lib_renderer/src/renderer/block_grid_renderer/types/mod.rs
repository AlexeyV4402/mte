pub mod vertex;

pub use vertex::BlockVertex;

#[derive(Clone, Copy)]
pub struct RendererCreateArgs {
    pub block_properties: &'static [u8],
    pub layer_count: u32,
    pub outline_vertices: &'static [u8],
}
