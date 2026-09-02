use glam::{Vec3, Vec4};
use lib_core::math::vectors::vec3::core::Vector3;
use winit::event_loop::EventLoop;

use crate::app::App;

mod app;
mod config;
mod gui;
mod network;
mod raycast;
mod types;
mod utils;
mod world_generator;

pub fn main() -> anyhow::Result<()> {
    unsafe { std::env::set_var("WINIT_UNIX_BACKEND", "x11") };
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();

    event_loop.run_app(&mut app)?;
    Ok(())
}
