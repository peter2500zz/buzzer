use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

use crate::input::{Action, handle_input};
use crate::render::{self, calculate_window_size};

// Your app state — owns windows, renderers, etc.
pub struct App {
    images: Vec<PathBuf>,
    current_image_index: usize,
    current_image: DynamicImage,
    oversized_image_cache: Option<DynamicImage>,

    context: Context<OwnedDisplayHandle>,
    state: Option<AppState>,
}

pub struct AppState {
    // just the window
    pub window: Rc<Window>,
    // canvases for drawing to the window
    pub surface: Surface<OwnedDisplayHandle, Rc<Window>>,

    // mouse pos
    mouse_pos: PhysicalPosition<f64>,

    // if Some, means zooming
    zoom_level: Option<f32>,
}

impl AppState {
    fn centered_resize_window(&mut self, width: u32, height: u32) {
        let current_pos = self.window.outer_position().unwrap();
        let current_size = self.window.outer_size();

        let center_x = current_pos.x + current_size.width as i32 / 2;
        let center_y = current_pos.y + current_size.height as i32 / 2;

        let new_x = center_x - width as i32 / 2;
        let new_y = center_y - height as i32 / 2;

        self.window
            .set_outer_position(PhysicalPosition::new(new_x, new_y));
        self.resize_window(width, height);
    }

    fn resize_window(&mut self, width: u32, height: u32) {
        let _ = self
            .window
            .request_inner_size(PhysicalSize::new(width, height));

        if let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(width, height).unwrap();
        }
    }
}

impl App {
    pub fn new(context: Context<OwnedDisplayHandle>, images: Vec<PathBuf>) -> Result<Self> {
        let first_image = image::open(&images[0])?;

        Ok(Self {
            images,
            current_image_index: 0,
            current_image: first_image,
            oversized_image_cache: None,
            context,
            state: None,
        })
    }

    fn change_image(&mut self, index: usize) {
        self.current_image = image::open(&self.images[index]).unwrap();
        let (w, h) = calculate_window_size(self.state.as_ref().unwrap(), &self.current_image);

        // 如果图片尺寸比最大可用尺寸还大，缓存一份缩放的小尺寸图片
        if (w, h) < self.current_image.dimensions() {
            self.oversized_image_cache =
                Some(self.current_image.resize_exact(w, h, FilterType::Lanczos3))
        } else {
            self.oversized_image_cache = None;
        }

        self.state.as_mut().unwrap().centered_resize_window(w, h);
    }
}

impl ApplicationHandler for App {
    // Platform signals ready — create windows here
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (w, h) = self.current_image.dimensions();

        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = monitor.size();

        // make sure the window is at the center of the screen
        let x = (monitor_size.width as i32 - w as i32) / 2;
        let y = (monitor_size.height as i32 - h as i32) / 2;

        let attrs = Window::default_attributes()
            .with_inner_size(PhysicalSize::new(w, h))
            .with_decorations(false)
            .with_title("Buzzer")
            .with_position(PhysicalPosition::new(x, y));

        let window = Rc::new(event_loop.create_window(attrs).unwrap());
        // sometimes the initial size is not image size, so request it again
        let _ = window.request_inner_size(PhysicalSize::new(w, h));

        let mut surface = Surface::new(&self.context, Rc::clone(&window)).unwrap();

        let size = window.inner_size();

        println!("Creating window with size {}x{}", size.width, size.height);

        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            // Resize surface
            surface.resize(width, height).unwrap();
            println!("Surface initialized to {}x{}", width, height);
        }

        self.state = Some(AppState {
            window,
            surface,

            mouse_pos: PhysicalPosition::new(0.0, 0.0),
            zoom_level: None,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                state.mouse_pos = position;
            }

            WindowEvent::RedrawRequested => {
                println!("\nRedrawing...");

                let mut buffer = state.surface.buffer_mut().unwrap();

                let image_size = self.current_image.dimensions();
                let buffer_size = (buffer.width().into(), buffer.height().into());

                // 如果有缓存的大图，进行额外判断
                let rgba = if let Some(oversized) = &self.oversized_image_cache {
                    if let Some(_zoom) = state.zoom_level {
                        // 如果正在缩放，使用当前缩放级别计算新的尺寸并缩放图片
                        self.current_image
                            .crop_imm(0, 0, buffer_size.0, buffer_size.1)
                            .to_rgba8()
                    } else {
                        oversized.to_rgba8()
                    }
                } else {
                    println!("No resizing needed for image of size {:?}", image_size);
                    self.current_image.to_rgba8()
                };

                render::render(&mut buffer, buffer_size, &rgba);

                buffer.present().unwrap();
            }

            _ => {
                if let Some(action) = handle_input(&event, state) {
                    match action {
                        Action::ZoomIn => {
                            if self.oversized_image_cache.is_some() {
                                println!("Zooming in...");
                                state.zoom_level = Some(1.);
                                state.window.request_redraw();
                            }
                        }
                        Action::ZoomOut => {
                            if self.oversized_image_cache.is_some() {
                                println!("Zooming out...");
                                state.zoom_level = None;
                                state.window.request_redraw();
                            }
                        }

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
