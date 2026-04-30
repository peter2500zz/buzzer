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
use crate::render::{self, calculate_display_size, calculate_mouse_percent, calculate_window_size};

pub struct App {
    images: Vec<PathBuf>,
    current_image_index: usize,
    current_image: DynamicImage,
    oversized_image_cache: Option<DynamicImage>,

    context: Context<OwnedDisplayHandle>,
    state: Option<AppState>,
}

pub struct AppState {
    pub window: Rc<Window>,
    pub surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    pub mouse_pos: PhysicalPosition<f64>,
    pub zoom_level: Option<f32>,
}

impl AppState {
    fn centered_resize_window(&mut self, width: u32, height: u32) {
        let pos = self.window.outer_position().unwrap();
        let size = self.window.outer_size();

        let new_x = pos.x + size.width as i32 / 2 - width as i32 / 2;
        let new_y = pos.y + size.height as i32 / 2 - height as i32 / 2;

        self.window
            .set_outer_position(PhysicalPosition::new(new_x, new_y));
        let _ = self
            .window
            .request_inner_size(PhysicalSize::new(width, height));
        if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(w, h).unwrap();
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
        // 快速读取尺寸，先 resize 窗口，再加载完整图片和缓存
        let img_dims = image::image_dimensions(&self.images[index]).unwrap();
        let monitor_size = {
            let s = self
                .state
                .as_ref()
                .unwrap()
                .window
                .current_monitor()
                .unwrap()
                .size();
            (s.width, s.height)
        };
        let (win_w, win_h) = calculate_window_size(monitor_size, img_dims);
        let (disp_w, disp_h) = calculate_display_size(monitor_size, img_dims);

        {
            let state = self.state.as_mut().unwrap();
            state.zoom_level = None;
            state.centered_resize_window(win_w, win_h);
        }

        self.current_image = image::open(&self.images[index]).unwrap();
        self.oversized_image_cache = if (disp_w, disp_h) < img_dims {
            Some(
                self.current_image
                    .resize_exact(disp_w, disp_h, FilterType::Lanczos3),
            )
        } else {
            None
        };

        self.state.as_mut().unwrap().window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = (monitor.size().width, monitor.size().height);
        let img_dims = self.current_image.dimensions();

        let (win_w, win_h) = calculate_window_size(monitor_size, img_dims);
        let (disp_w, disp_h) = calculate_display_size(monitor_size, img_dims);

        if (disp_w, disp_h) < img_dims {
            self.oversized_image_cache = Some(self.current_image.resize_exact(
                disp_w,
                disp_h,
                FilterType::Lanczos3,
            ));
        }

        let x = (monitor_size.0 as i32 - win_w as i32) / 2;
        let y = (monitor_size.1 as i32 - win_h as i32) / 2;

        let window = Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(PhysicalSize::new(win_w, win_h))
                        .with_decorations(false)
                        .with_title("Buzzer")
                        .with_position(PhysicalPosition::new(x, y)),
                )
                .unwrap(),
        );
        let _ = window.request_inner_size(PhysicalSize::new(win_w, win_h));

        let mut surface = Surface::new(&self.context, Rc::clone(&window)).unwrap();
        let size = window.inner_size();
        if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
            surface.resize(w, h).unwrap();
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
            WindowEvent::Resized(size) => {
                if let (Some(w), Some(h)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    state.surface.resize(w, h).unwrap();
                }
                state.window.request_redraw();
            }

            WindowEvent::CursorMoved { position, .. } => {
                state.mouse_pos = position;
                if state.zoom_level.is_some() {
                    state.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                let mouse_percent = calculate_mouse_percent(state);
                let mut buffer = state.surface.buffer_mut().unwrap();

                let image_size = self.current_image.dimensions();
                let buffer_size = (buffer.width().into(), buffer.height().into());

                let rgba = if let Some(oversized) = &self.oversized_image_cache {
                    if let Some(zoom) = state.zoom_level {
                        let unclamped_w = (buffer_size.0 as f32 / zoom) as u32;
                        let unclamped_h = (buffer_size.1 as f32 / zoom) as u32;
                        let crop_w = unclamped_w.min(image_size.0);
                        let crop_h = unclamped_h.min(image_size.1);

                        let cx = (image_size.0 as f32 * mouse_percent.0) as i32;
                        let cy = (image_size.1 as f32 * mouse_percent.1) as i32;

                        let x0 = (cx - crop_w as i32 / 2).clamp(0, (image_size.0 - crop_w) as i32)
                            as u32;
                        let y0 = (cy - crop_h as i32 / 2).clamp(0, (image_size.1 - crop_h) as i32)
                            as u32;

                        let cropped = self.current_image.crop_imm(x0, y0, crop_w, crop_h);

                        let was_clamped = crop_w < unclamped_w || crop_h < unclamped_h;
                        if !was_clamped && (crop_w != buffer_size.0 || crop_h != buffer_size.1) {
                            cropped
                                .resize_exact(buffer_size.0, buffer_size.1, FilterType::Lanczos3)
                                .to_rgba8()
                        } else {
                            cropped.to_rgba8()
                        }
                    } else {
                        oversized.to_rgba8()
                    }
                } else {
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
                                state.zoom_level = Some(1.0);
                                state.window.request_redraw();
                            }
                        }
                        Action::ZoomOut => {
                            if self.oversized_image_cache.is_some() {
                                state.zoom_level = None;
                                state.window.request_redraw();
                            }
                        }
                        Action::PreviousImage => {
                            self.current_image_index = if self.current_image_index == 0 {
                                self.images.len() - 1
                            } else {
                                self.current_image_index - 1
                            };
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
