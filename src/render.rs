use image::RgbaImage;

use crate::app::AppState;

pub const MAX_SCALE: f32 = 0.8;
pub const MIN_SIZE: u32 = 200;
pub const DEAD_ZONE: u32 = MIN_SIZE / 2;

/// 图片显示尺寸：仅等比缩小以适应显示器，不放大
pub fn calculate_display_size(monitor_size: (u32, u32), img_size: (u32, u32)) -> (u32, u32) {
    let (img_w, img_h) = img_size;
    let (mon_w, mon_h) = (monitor_size.0 as f32, monitor_size.1 as f32);

    let scale = ((mon_w * MAX_SCALE) / img_w as f32)
        .min((mon_h * MAX_SCALE) / img_h as f32)
        .min(1.0);

    (
        (img_w as f32 * scale).round() as u32,
        (img_h as f32 * scale).round() as u32,
    )
}

/// 窗口尺寸：每条边独立地不小于 MIN_SIZE，多余空间由棋盘格填充
pub fn calculate_window_size(monitor_size: (u32, u32), img_size: (u32, u32)) -> (u32, u32) {
    let (dw, dh) = calculate_display_size(monitor_size, img_size);
    (dw.max(MIN_SIZE), dh.max(MIN_SIZE))
}

/// 鼠标在窗口中的相对位置百分比，边缘有死区
pub fn calculate_mouse_percent(state: &AppState) -> (f32, f32) {
    let size = state.window.inner_size();
    let (w, h) = (size.width, size.height);

    let calc_axis = |pos: f32, length: u32| -> f32 {
        let dead = DEAD_ZONE as f32;
        let effective = length as f32 - 2.0 * dead;
        if pos <= dead {
            0.0
        } else if pos >= length as f32 - dead {
            1.0
        } else {
            ((pos - dead) / effective).clamp(0.0, 1.0)
        }
    };

    (
        calc_axis(state.mouse_pos.x as f32, w),
        calc_axis(state.mouse_pos.y as f32, h),
    )
}

#[inline(always)]
fn pixel_to_buf(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

const N: u32 = 16;
const CHECKER_LIGHT: u32 = 0x00FFFFFF;
const CHECKER_DARK: u32 = 0x00AAAAAA;

#[inline(always)]
fn checker_color(x: u32, y: u32) -> u32 {
    if (x / N + y / N) % 2 == 0 {
        CHECKER_LIGHT
    } else {
        CHECKER_DARK
    }
}

#[inline(always)]
fn blend_checker(r: u8, g: u8, b: u8, a: u8, x: u32, y: u32) -> u32 {
    let bg = checker_color(x, y);
    let (bg_r, bg_g, bg_b) = (
        (bg >> 16 & 0xFF) as u16,
        (bg >> 8 & 0xFF) as u16,
        (bg & 0xFF) as u16,
    );
    let (a, ia) = (a as u16, 255 - a as u16);
    pixel_to_buf(
        ((r as u16 * a + bg_r * ia) / 255) as u8,
        ((g as u16 * a + bg_g * ia) / 255) as u8,
        ((b as u16 * a + bg_b * ia) / 255) as u8,
    )
}

pub fn draw_checkerboard(buf: &mut [u32], width: u32, _height: u32) {
    for (i, dst) in buf.iter_mut().enumerate() {
        *dst = checker_color(i as u32 % width, i as u32 / width);
    }
}

fn copy_rgba_to(buf: &mut [u32], buf_w: u32, buf_h: u32, rgba: &RgbaImage, x: i32, y: i32) {
    let (src_w, src_h) = (rgba.width() as i32, rgba.height() as i32);

    let dst_x0 = x.max(0).min(buf_w as i32) as u32;
    let dst_x1 = (x + src_w).max(0).min(buf_w as i32) as u32;
    let dst_y0 = y.max(0).min(buf_h as i32) as u32;
    let dst_y1 = (y + src_h).max(0).min(buf_h as i32) as u32;

    if dst_x0 >= dst_x1 || dst_y0 >= dst_y1 {
        return;
    }

    let (src_x0, src_y0) = ((dst_x0 as i32 - x) as u32, (dst_y0 as i32 - y) as u32);
    let copy_w = dst_x1 - dst_x0;

    for row in 0..(dst_y1 - dst_y0) {
        let dst_start = ((dst_y0 + row) * buf_w + dst_x0) as usize;
        for (col, dst) in buf[dst_start..dst_start + copy_w as usize]
            .iter_mut()
            .enumerate()
        {
            let [r, g, b, a] = rgba.get_pixel(src_x0 + col as u32, src_y0 + row).0;
            *dst = blend_checker(r, g, b, a, dst_x0 + col as u32, dst_y0 + row);
        }
    }
}

pub fn render(buffer: &mut [u32], buffer_size: (u32, u32), image: &RgbaImage) {
    let image_size = image.dimensions();

    if buffer_size.0 > image_size.0 || buffer_size.1 > image_size.1 {
        draw_checkerboard(buffer, buffer_size.0, buffer_size.1);
        copy_rgba_to(
            buffer,
            buffer_size.0,
            buffer_size.1,
            image,
            (buffer_size.0 as i32 - image_size.0 as i32) / 2,
            (buffer_size.1 as i32 - image_size.1 as i32) / 2,
        );
    } else {
        copy_rgba_to(buffer, buffer_size.0, buffer_size.1, image, 0, 0);
    }
}
