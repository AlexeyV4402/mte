use bitcode::{Decode, Encode};
// use num_traits::FromPrimitive;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};

pub type PlayerIdType = u32;
pub type PlayerCountType = u16;

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, FromPrimitive, Encode, Decode)]
pub enum Faction {
    Stalker = 0,
    Bandit = 1,
    Military = 2,
    Duty = 3,
    Freedom = 4,
}

pub fn empty_inventory() -> Vec<u8> {
    let inv: Vec<Item> = Vec::new();
    bincode::serialize(&inv).unwrap()
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Item {}
