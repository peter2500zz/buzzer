use std::rc::Rc;

use image::{DynamicImage, GenericImageView, imageops::FilterType};
use softbuffer::Buffer;
use winit::{event_loop::OwnedDisplayHandle, window::Window};

use crate::app::AppState;

pub fn calculate_window_size(state: &AppState, image: &DynamicImage) -> (u32, u32) {
    // 最大不能比显示器 * MAX_SCALE 大
    // 最小不能比 MIN_SIZE 小

    const MAX_SCALE: f32 = 0.8;
    const MIN_SIZE: u32 = 200;

    let (img_w, img_h) = image.dimensions();
    let (disp_w, disp_h): (f32, f32) = state.window.current_monitor().unwrap().size().into();

    let w = img_w.clamp(MIN_SIZE, (disp_w * MAX_SCALE) as u32);
    let h = img_h.clamp(MIN_SIZE, (disp_h * MAX_SCALE) as u32);

    (w, h)
}

pub fn render(buffer: &mut Buffer<OwnedDisplayHandle, Rc<Window>>, image: &DynamicImage) {
    let buffer_size = (buffer.width().into(), buffer.height().into());
    let image_size = image.dimensions();

    // 如果缓冲区比图片大，图片居中原始尺寸绘制
    // 如果缓冲区比图片小，图片缩放到缓冲区大小绘制

    let rgba = if buffer_size != image_size {
        println!("Resizing image from {:?} to {:?}", image_size, buffer_size);

        let resized_image =
            image
                .clone()
                .resize_exact(buffer_size.0, buffer_size.1, FilterType::Lanczos3);
        resized_image.to_rgba8()
    } else {
        println!("No resizing needed for image of size {:?}", image_size);
        image.to_rgba8()
    };

    for (dst, src) in buffer.iter_mut().zip(rgba.pixels()) {
        let [r, g, b, _a] = src.0;
        *dst = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    }
}

