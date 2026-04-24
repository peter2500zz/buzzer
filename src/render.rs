use image::{DynamicImage, GenericImageView, RgbaImage};

use crate::app::AppState;

pub const MAX_SCALE: f32 = 0.8;
pub const MIN_SIZE: u32 = 200;
pub const DEAD_ZONE: u32 = MIN_SIZE / 2;

pub fn calculate_window_size(state: &AppState, image: &DynamicImage) -> (u32, u32) {
    // 最大不能比显示器 * MAX_SCALE 大
    // 最小不能比 MIN_SIZE 小

    let (img_w, img_h) = image.dimensions();
    let (disp_w, disp_h): (f32, f32) = state.window.current_monitor().unwrap().size().into();

    let w = img_w.clamp(MIN_SIZE, (disp_w * MAX_SCALE) as u32);
    let h = img_h.clamp(MIN_SIZE, (disp_h * MAX_SCALE) as u32);

    (w, h)
}

/// 计算鼠标在窗口中的相对位置百分比，考虑了边缘的死区
pub fn calculate_mouse_percent(state: &AppState) -> (f32, f32) {
    let size = state.window.inner_size();
    let (w, h) = (size.width, size.height);

    let mouse_x = state.mouse_pos.x as f32;
    let mouse_y = state.mouse_pos.y as f32;

    let calc_axis = |pos: f32, length: u32| -> f32 {
        let dead = DEAD_ZONE as f32;
        let effective = (length as f32) - 2.0 * dead;

        if pos <= dead {
            0.0
        } else if pos >= (length as f32) - dead {
            1.0
        } else {
            ((pos - dead) / effective).clamp(0.0, 1.0)
        }
    };

    (calc_axis(mouse_x, w), calc_axis(mouse_y, h))
}

#[inline(always)]
fn pixel_to_buf(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

// 决定了 checkerboard 的格子大小和颜色
const N: u32 = 16;

const CHECKER_LIGHT: u32 = 0x00FFFFFF;
const CHECKER_DARK: u32 = 0x00AAAAAA;

/// 根据坐标计算棋盘格颜色
#[inline(always)]
fn checker_color(x: u32, y: u32) -> u32 {
    if (x / N + y / N) % 2 == 0 {
        CHECKER_LIGHT
    } else {
        CHECKER_DARK
    }
}

/// 将 RGBA 颜色与 checkerboard 背景混合，得到最终显示的颜色
///
/// 由于 winit 的跨平台支持性，Alpha 通道将不被使用。这里与棋盘格混合色彩以模拟透明效果。
#[inline(always)]
fn blend_checker(r: u8, g: u8, b: u8, a: u8, x: u32, y: u32) -> u32 {
    let bg = checker_color(x, y);
    let bg_r = ((bg >> 16) & 0xFF) as u16;
    let bg_g = ((bg >> 8) & 0xFF) as u16;
    let bg_b = (bg & 0xFF) as u16;

    let a = a as u16;
    let ia = 255 - a;

    let out_r = (r as u16 * a + bg_r * ia) / 255;
    let out_g = (g as u16 * a + bg_g * ia) / 255;
    let out_b = (b as u16 * a + bg_b * ia) / 255;

    pixel_to_buf(out_r as u8, out_g as u8, out_b as u8)
}

/// 填充整个缓冲区为棋盘格背景
pub fn draw_checkerboard(buf: &mut [u32], width: u32, _height: u32) {
    for (i, dst) in buf.iter_mut().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        *dst = checker_color(x, y);
    }
}

/// 将 RGBA 图片复制到缓冲区的特定位置，自动处理边界和 alpha 混合
fn copy_rgba_to(
    buf: &mut [u32],
    buf_width: u32,
    buf_height: u32,
    rgba: &image::RgbaImage,
    x: i32,
    y: i32,
) {
    let src_width = rgba.width();
    let src_height = rgba.height();

    let dst_x0 = x.max(0).min(buf_width as i32) as u32;
    let dst_x1 = (x + src_width as i32).max(0).min(buf_width as i32) as u32;
    let dst_y0 = y.max(0).min(buf_height as i32) as u32;
    let dst_y1 = (y + src_height as i32).max(0).min(buf_height as i32) as u32;

    if dst_x0 >= dst_x1 || dst_y0 >= dst_y1 {
        return;
    }

    let src_x0 = (dst_x0 as i32 - x) as u32;
    let src_y0 = (dst_y0 as i32 - y) as u32;
    let copy_width = dst_x1 - dst_x0;

    for row in 0..(dst_y1 - dst_y0) {
        let dst_row_start = ((dst_y0 + row) * buf_width + dst_x0) as usize;
        let dst_slice = &mut buf[dst_row_start..dst_row_start + copy_width as usize];

        for (col, dst) in dst_slice.iter_mut().enumerate() {
            let px = rgba.get_pixel(src_x0 + col as u32, src_y0 + row);
            let [r, g, b, a] = px.0;
            let abs_x = dst_x0 + col as u32;
            let abs_y = dst_y0 + row;
            *dst = blend_checker(r, g, b, a, abs_x, abs_y);
        }
    }
}

pub fn render(buffer: &mut [u32], buffer_size: (u32, u32), image: &RgbaImage) {
    let image_size = image.dimensions();

    // 如果缓冲区比图片大，图片居中原始尺寸绘制
    if buffer_size > image_size {
        draw_checkerboard(buffer, buffer_size.0, buffer_size.1);

        copy_rgba_to(
            buffer,
            buffer_size.0,
            buffer_size.1,
            &image,
            (buffer_size.0 as i32 - image_size.0 as i32) / 2,
            (buffer_size.1 as i32 - image_size.1 as i32) / 2,
        );

        return;
    }

    copy_rgba_to(buffer, buffer_size.0, buffer_size.1, &image, 0, 0);
}
