// use core::fmt;
// use std::marker::PhantomData;

pub type LocalCoordsType = u32;
pub type InternalCoordsType = u32;
pub type GlobalCoordsType = i64;
pub type ChunkCoordsType = i32;
pub type RegionCoordsType = i32;
pub type InRegionCoordsType = i32;

pub type LocalCoords = Coords<Local, LocalCoordsType>;
pub type InternalCoords = Coords<Internal, InternalCoordsType>;
pub type GlobalCoords = Coords<Global, GlobalCoordsType>;
pub type ChunkCoords = Coords<Chunk, ChunkCoordsType>;
pub type RegionCoords = Coords<Region, RegionCoordsType>;
pub type InRegionCoords = Coords<InRegion, InRegionCoordsType>;

#[derive(Clone, Copy, Debug)]
pub struct Local;

#[derive(Clone, Copy, Debug)]
pub struct Global;

#[derive(Clone, Eq, Hash, PartialEq, Copy, Debug)]
pub struct Chunk;

#[derive(Clone, Copy, Debug)]
pub struct Internal;

#[derive(Clone, Eq, Hash, PartialEq, Copy, Debug)]
pub struct Region;

#[derive(Clone, Eq, Hash, PartialEq, Copy, Debug)]
pub struct InRegion;

use std::marker::PhantomData;

use lib_core::math::vectors::vec3::core::Vector3;

#[repr(C)]
#[derive(Clone, Copy, Eq, Hash, PartialEq, Default, Debug)]
pub struct Coords<S, N>(pub Vector3<N>, pub std::marker::PhantomData<S>);

impl<S, N> Coords<S, N> {
    pub fn new(x: N, y: N, z: N) -> Coords<S, N> {
        Coords(Vector3::new(x, y, z), PhantomData)
    }
}

impl<S, N> From<Coords<S, N>> for (N, N, N) {
    fn from(coords: Coords<S, N>) -> Self {
        coords.0.into()
    }
}

impl<S, N> From<(N, N, N)> for Coords<S, N> {
    fn from(coords: (N, N, N)) -> Self {
        Self(Vector3::new(coords.0, coords.1, coords.2), PhantomData)
    }
}

impl<S, N> From<Vector3<N>> for Coords<S, N> {
    fn from(vec: Vector3<N>) -> Self {
        Self(vec, PhantomData)
    }
}
