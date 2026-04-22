use std::env;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

// Your app state — owns windows, renderers, etc.
struct App {
    // images: Vec<PathBuf>,
    current_image: DynamicImage,

    context: Context<OwnedDisplayHandle>,
    state: Option<AppState>,
}

struct AppState {
    // window: Rc<Window>,
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
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

        self.state = Some(AppState { 
            // window, 
            surface 
        });
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
        // images: args,
        current_image: first_image,
        context: Context::new(event_loop.owned_display_handle()).unwrap(),
        state: None,
    };
    event_loop.run_app(&mut app).unwrap();

    Ok(())
}
