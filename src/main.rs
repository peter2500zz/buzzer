// hide the console on Windows when building in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

// Your app state — owns windows, renderers, etc.
struct App {
    images: Vec<PathBuf>,
    current_image_index: usize,
    current_image: DynamicImage,

    context: Context<OwnedDisplayHandle>,
    state: Option<AppState>,
}

struct AppState {
    window: Rc<Window>,
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
}

impl App {
    fn change_image(&mut self, index: usize) {
        self.current_image = image::open(&self.images[index]).unwrap();
        let (w, h) = self.current_image.dimensions();

        let _ = self
            .state
            .as_ref()
            .unwrap()
            .window
            .request_inner_size(LogicalSize::new(w, h));

        if let (Some(width), Some(height)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            self.state
                .as_mut()
                .unwrap()
                .surface
                .resize(width, height)
                .unwrap();
        }

        self.state.as_ref().unwrap().window.request_redraw();
    }
}

impl ApplicationHandler for App {
    // Platform signals ready — create windows here
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (w, h) = self.current_image.dimensions();

        let attrs = Window::default_attributes()
            .with_inner_size(LogicalSize::new(w, h))
            .with_decorations(false)
            .with_title("Buzzer");

        let window = Rc::new(event_loop.create_window(attrs).unwrap());

        let mut surface = Surface::new(&self.context, Rc::clone(&window)).unwrap();

        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            // Resize surface
            surface.resize(width, height).unwrap();
        }

        self.state = Some(AppState { window, surface });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                // render here
                // self.state.as_ref().unwrap().window.request_redraw();

                let surface = &mut self.state.as_mut().unwrap().surface;

                let mut buffer = surface.buffer_mut().unwrap();

                let rgba = self.current_image.to_rgba8();

                for (dst, src) in buffer.iter_mut().zip(rgba.pixels()) {
                    let [r, g, b, _a] = src.0;
                    *dst = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }

                buffer.present().unwrap();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(named),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match named {
                NamedKey::Escape => event_loop.exit(),
                NamedKey::ArrowUp => println!("↑"),
                NamedKey::ArrowDown => println!("↓"),
                NamedKey::ArrowLeft => {
                    if self.current_image_index == 0 {
                        self.current_image_index = self.images.len() - 1;
                    } else {
                        self.current_image_index -= 1;
                    }
                    self.change_image(self.current_image_index);
                }
                NamedKey::ArrowRight => {
                    self.current_image_index = (self.current_image_index + 1) % self.images.len();
                    self.change_image(self.current_image_index);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let mut args: Vec<PathBuf> = env::args().map(PathBuf::from).collect();

    if args.len() < 2 {
        eprintln!("Usage: buzzer <image1> <image2> ...");
        std::process::exit(1);
    }

    args.remove(0); // Remove the program name

    let first_image = image::open(&args[0])?;

    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        images: args,
        current_image_index: 0,
        current_image: first_image,
        context: Context::new(event_loop.owned_display_handle()).unwrap(),
        state: None,
    };
    event_loop.run_app(&mut app).unwrap();

    Ok(())
}
