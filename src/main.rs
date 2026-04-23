// hide the console on Windows when building in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod input;
mod render;

use anyhow::Result;
use softbuffer::Context;
use std::{env, path::PathBuf, process};
use winit::event_loop::EventLoop;

use crate::app::App;

fn main() -> Result<()> {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();

    if args.is_empty() {
        eprintln!("Usage: buzzer <image1> <image2> ...");
        process::exit(1);
    }

    let event_loop = EventLoop::new()?;
    let context = Context::new(event_loop.owned_display_handle()).unwrap();
    let mut app = App::new(context, args)?;

    event_loop.run_app(&mut app)?;

    Ok(())
}
