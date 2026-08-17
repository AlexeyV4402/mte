use std::marker::PhantomData;

use crate::renderer::block_grid_renderer::types::BlockVertex;
use crate::renderer::block_grid_renderer::types::vertex::GuiVertex;

pub type BlockIndexedPrimitive = IndexedPrimitive<BlockVertex, u32>;
pub type GuiIndexedPrimitive = IndexedPrimitive<GuiVertex, u32>;

pub struct IndexedPrimitive<T, I> {
    pub vertices: Vec<T>,
    pub indices: Vec<I>,
}

impl<T, I> IndexedPrimitive<T, I> {
    pub fn new(vertices: Vec<T>, indices: Vec<I>) -> Self {
        Self { vertices, indices }
    }
}

// #[derive(Default)]
// pub struct IndexedPrimitiveBuilder<V, I>
// where
//     V: Clone,
//     I: Clone + Copy + std::ops::Add<I, Output = I> + std::ops::AddAssign<usize>,
// {
//     vertices: Vec<V>,
//     indices: Vec<I>,
//     vertex_counter: I
// }

// impl<V, I> IndexedPrimitiveBuilder<V, I>
// where
//     V: Clone,
//     I: Clone + Copy + std::ops::Add<I, Output = I> + std::ops::AddAssign<usize>,
// {
//     #[inline]
//     pub fn add_vertices<const N: usize>(&mut self, vertices: &[V; N]) {
//         self.vertices.extend_from_slice(vertices);
//         self.indices.extend_from_slice(&[self.vertex_counter + 0 as I, self.vertex_counter + 1, self.vertex_counter + 2, self.vertex_counter + 0, self.vertex_counter + 2, self.vertex_counter + 3]);
//         self.vertex_counter += N;
//     }

//     pub fn build(self) -> IndexedPrimitive<V, I> {
//         IndexedPrimitive {
//             vertices: self.vertices,
//             indices: self.indices,
//         }
//     }
// }
