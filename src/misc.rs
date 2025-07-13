use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb, imageops::ColorMap};
use rand::{Rng, rng};
use std::{io::Write, process::exit};

pub static PALETTE: Palette = Palette {
    colors: [
        Rgb([0x00, 0x00, 0x00]),
        Rgb([0x80, 0x80, 0x80]),
        Rgb([0xC0, 0xC0, 0xC0]),
        Rgb([0xFF, 0xFF, 0xFF]),
        Rgb([0x00, 0x00, 0x80]),
        Rgb([0x00, 0x80, 0x00]),
        Rgb([0x00, 0x80, 0x80]),
        Rgb([0x80, 0x00, 0x00]),
        Rgb([0x80, 0x00, 0x80]),
        Rgb([0x80, 0x80, 0x00]),
        Rgb([0x00, 0x00, 0xFF]),
        Rgb([0x00, 0xFF, 0x00]),
        Rgb([0x00, 0xFF, 0xFF]),
        Rgb([0xFF, 0x00, 0x00]),
        Rgb([0xFF, 0x00, 0xFF]),
        Rgb([0xFF, 0xFF, 0x00]),
    ],
};

pub static COLORS: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0xFF, 0xFF, 0xFF],
    [0x00, 0x00, 0xFF],
    [0x00, 0xFF, 0x00],
    [0xFF, 0x00, 0x00],
    [0x00, 0xFF, 0xFF],
    [0xFF, 0x00, 0xFF],
    [0xFF, 0xFF, 0x00],
    [0xC0, 0xC0, 0xC0],
    [0x80, 0x80, 0x80],
    [0x80, 0x00, 0x00],
    [0x80, 0x80, 0x00],
    [0x00, 0x80, 0x00],
    [0x00, 0x80, 0x80],
    [0x00, 0x00, 0x80],
    [0x80, 0x00, 0x80],
];

pub struct Palette {
    colors: [Rgb<u8>; 16],
}

impl ColorMap for Palette {
    type Color = Rgb<u8>;

    fn index_of(&self, color: &Self::Color) -> usize {
        self.colors
            .iter()
            .enumerate()
            .min_by_key(|&(_, rgb)| {
                let r = rgb[0] as i32 - color[0] as i32;
                let g = rgb[1] as i32 - color[1] as i32;
                let b = rgb[2] as i32 - color[2] as i32;
                r * r + g * g + b * b
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn map_color(&self, color: &mut Self::Color) {
        let idx = self.index_of(color);
        let rgb = self.colors.to_owned()[idx];
        *color = Rgb([rgb[0], rgb[1], rgb[2]]);
    }
}

pub fn get_encode(pixel_target: &[u8]) -> u8 {
    for (idx, pixel) in COLORS.iter().enumerate() {
        if *pixel == pixel_target {
            return idx as u8;
        }
    }
    0
}

fn bytes_to_base64url(bytes: &[u8]) -> String {
    base64_url::encode(bytes)
}

pub fn base64url_to_bytes(code: &str) -> Option<Vec<u8>> {
    base64_url::decode(code).ok()
}

pub fn pack_dimensions(a: u16, b: u16) -> [u8; 3] {
    let combined: u32 = ((a as u32) << 12) | (b as u32);

    [
        ((combined >> 16) & 0xFF) as u8,
        ((combined >> 8) & 0xFF) as u8,
        (combined & 0xFF) as u8,
    ]
}

pub fn unpack_dimensions(bytes: &[u8]) -> (u32, u32) {
    let combined: u32 = ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32);

    let height = (combined >> 12) as u16 & 0xFFF;
    let width = (combined & 0xFFF) as u16;

    (height as u32, width as u32)
}

pub fn byte_to_codes(byte: u8) -> [u8; 2] {
    [byte >> 4, byte & 0x0F]
}

pub fn codes_to_byte(first: u8, second: u8) -> u8 {
    first << 4 | second
}

pub fn write_file(bytes: &[u8], output_file_path: &str) {
    match std::fs::File::create(output_file_path) {
        Ok(mut file) => {
            if let Some(err) = file.write_all(bytes).err() {
                eprintln!("Error: {}", err);
                exit(1);
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    }
}

pub fn open_img(path: &str) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, image::ImageError> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = image::ImageReader::open(path)?.decode()?.into_rgb8();
    Ok(img)
}

pub fn save_img(
    img: ImageBuffer<Rgb<u8>, Vec<u8>>,
    output_file_path: &str,
) -> Result<(), image::ImageError> {
    match DynamicImage::ImageRgb8(img).save_with_format(output_file_path, ImageFormat::Png) {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}

pub fn gen_key() -> String {
    let mut rng = rng();
    bytes_to_base64url(
        (0..16)
            .map(|_| rng.random())
            .collect::<Vec<u8>>()
            .as_slice(),
    )
}

fn closest_color(color_to: Rgb<u8>) -> Rgb<u8> {
    *PALETTE
        .colors
        .iter()
        .min_by_key(|rgb| {
            let r = color_to[0] as i32 - rgb[0] as i32;
            let g = color_to[1] as i32 - rgb[1] as i32;
            let b = color_to[2] as i32 - rgb[2] as i32;
            r.pow(2) + g.pow(2) + b.pow(2)
        })
        .unwrap()
}

pub fn quantization(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
    let (width, height) = img.dimensions();
    for i in 0..height {
        for j in 0..width {
            img.put_pixel(j, i, closest_color(*img.get_pixel(j, i)));
        }
    }
}

pub fn help() {
    println!("Usage: exe [options] [input_file_path] [output_file_path] [base64url_key(opt)]

    options:
        - e - encode mode: input - existing [input_file_path], output - saved [output_file_path] or stderr
        - p - enable dither processing for image encoding (can be disabled when re-encoding of already dithered images)
        - d - decode mode: input - existing [input_file_path], output - saved [output_file_path] or stderr
        - ee - encode-encrypt mode: input - existing [input_file_path] and [base64url_key], output - saved [output_file_path] or stderr
        - dd - decode-decrypt mode: input - existing [input_file_path] and [base64url_key], output - saved [output_file_path] or stderr
        - g - 16bytes base64url stdout key gen (doesn not need any input)
")
}
