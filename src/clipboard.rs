use clipboard_win::types::c_uint;
use eframe::epaint::ColorImage;
use image::{ImageBuffer, Rgba};
use image::imageops::{FilterType, resize};
use md5::{Digest, Md5};
use winapi::um::winuser::CF_DIB;

/// 粘贴板数据类
#[derive(Clone)]
pub struct Data {
    data_type: c_uint,
    raw: Vec<u8>,
    md5: [u8; 16],
    data: Option<String>,
    img: Option<ColorImage>,
}

impl Data {
    pub(crate) fn new(data_type: c_uint) -> Data {
        Data {
            data_type,
            raw: Vec::new(),
            md5: [0; 16],
            data: None,
            img: None,
        }
    }

    pub fn set_raw(&mut self, raw: Vec<u8>) {
        let mut result = Md5::new();
        result.update(raw.clone());
        self.raw = raw;
        self.md5 = <[u8; 16]>::from(result.finalize());
        // 如果是图片类型，自动生成可展示图片
        if self.data_type.eq(&CF_DIB) {
            self.img = Some(self.generate_image());
        }
    }

    pub fn set_data(&mut self, data: String) {
        self.data = Some(data);
    }

    pub fn get_content(&self) -> Vec<u8> {
        return self.raw.clone();
    }

    pub fn get_md5(&self) -> [u8; 16] {
        return self.md5;
    }

    pub fn get_type(&self) -> c_uint {
        return self.data_type;
    }

    pub fn get_data(&self) -> String {
        return self.data.clone().unwrap_or(String::from("")).trim().to_string();
    }

    pub fn get_image(&self) -> ColorImage {
        return self.img.clone().unwrap_or(ColorImage::default());
    }

    fn generate_image(&self) -> ColorImage {
        let dib_data = self.get_content();
        let width = (dib_data[4] as u32) | ((dib_data[5] as u32) << 8) | ((dib_data[6] as u32) << 16) | ((dib_data[7] as u32) << 24);
        let height = (dib_data[8] as u32) | ((dib_data[9] as u32) << 8) | ((dib_data[10] as u32) << 16) | ((dib_data[11] as u32) << 24);
        let stride = ((width * 32 + 31) / 32) * 4;
        let mut buffer = vec![0; (height * stride) as usize];
        let l = buffer.len();
        buffer.copy_from_slice(&dib_data[40..(40 + l)]);

        let mut image_buffer = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let offset = ((height - 1 - y) * stride + x * 4) as usize;
                let pixel = Rgba([
                    buffer[offset + 2],
                    buffer[offset + 1],
                    buffer[offset],
                    buffer[offset + 3],
                ]);
                image_buffer.put_pixel(x, y, pixel);
            }
        }

        // 限制图像大小为最大宽度和高度为 150 像素
        let max_size = 150;

        return if width > max_size || height > max_size {
            let scale = if width > height {
                max_size as f32 / width as f32
            } else {
                max_size as f32 / height as f32
            };
            let new_width = (width as f32 * scale) as u32;
            let new_height = (height as f32 * scale) as u32;
            let resized_image = resize(&image_buffer, new_width, new_height, FilterType::Lanczos3);
            ColorImage::from_rgba_unmultiplied(
                [resized_image.width() as usize, resized_image.height() as usize],
                &resized_image,
            )
        } else {
            ColorImage::from_rgba_unmultiplied(
                [image_buffer.width() as usize, image_buffer.height() as usize],
                &image_buffer,
            )
        };
    }
}
