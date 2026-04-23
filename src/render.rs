use std::rc::Rc;

use image::{DynamicImage, GenericImageView, imageops::FilterType};
use softbuffer::Buffer;
use winit::{event_loop::OwnedDisplayHandle, window::Window};

pub fn render(buffer: &mut Buffer<OwnedDisplayHandle, Rc<Window>>, image: &DynamicImage) {
    let buffer_size = (buffer.width().into(), buffer.height().into());
    let image_size = image.dimensions();

    let rgba = if buffer_size != image_size {
        let resized_image =
            image
                .clone()
                .resize_exact(buffer_size.0, buffer_size.1, FilterType::Lanczos3);
        resized_image.to_rgba8()
    } else {
        image.to_rgba8()
    };

    for (dst, src) in buffer.iter_mut().zip(rgba.pixels()) {
        let [r, g, b, _a] = src.0;
        *dst = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    }
}
