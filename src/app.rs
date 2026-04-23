use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

use crate::input::{Action, handle_input};
use crate::render;

// Your app state — owns windows, renderers, etc.
pub struct App {
    images: Vec<PathBuf>,
    current_image_index: usize,
    current_image: DynamicImage,

    context: Context<OwnedDisplayHandle>,
    state: Option<AppState>,
}

pub struct AppState {
    window: Rc<Window>,
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
}

impl App {
    pub fn new(context: Context<OwnedDisplayHandle>, images: Vec<PathBuf>) -> Result<Self> {
        let first_image = image::open(&images[0])?;

        Ok(Self {
            images,
            current_image_index: 0,
            current_image: first_image,
            context,
            state: None,
        })
    }

    fn change_image(&mut self, index: usize) {
        self.current_image = image::open(&self.images[index]).unwrap();
        let (w, h) = self.current_image.dimensions();

        let _ = self
            .state
            .as_ref()
            .unwrap()
            .window
            .request_inner_size(PhysicalSize::new(w, h));

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

        println!("Creating window with size {}x{}", w, h);

        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = monitor.size();

        let x = (monitor_size.width as i32 - w as i32) / 2;
        let y = (monitor_size.height as i32 - h as i32) / 2;

        let attrs = Window::default_attributes()
            .with_inner_size(PhysicalSize::new(w, h))
            .with_decorations(false)
            .with_title("Buzzer")
            .with_position(PhysicalPosition::new(x, y));

        let window = Rc::new(event_loop.create_window(attrs).unwrap());

        let mut surface = Surface::new(&self.context, Rc::clone(&window)).unwrap();

        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            // Resize surface
            surface.resize(width, height).unwrap();
            println!("Resized surface to {}x{}", width, height);
        }

        self.state = Some(AppState { window, surface });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();

        match event {
            WindowEvent::RedrawRequested => {
                let mut buffer = state.surface.buffer_mut().unwrap();

                render::render(&mut buffer, &self.current_image);

                buffer.present().unwrap();
            }

            _ => {
                if let Some(action) = handle_input(&event, state) {
                    match action {
                        Action::PreviousImage => {
                            if self.current_image_index == 0 {
                                self.current_image_index = self.images.len() - 1;
                            } else {
                                self.current_image_index -= 1;
                            }
                            self.change_image(self.current_image_index);
                        }
                        Action::NextImage => {
                            self.current_image_index =
                                (self.current_image_index + 1) % self.images.len();
                            self.change_image(self.current_image_index);
                        }

                        Action::Quit => event_loop.exit(),
                    }
                }
            }
        }
    }
}
