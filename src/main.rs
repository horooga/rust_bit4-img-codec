use aes::Aes128;
use fpe::ff1::{BinaryNumeralString, FF1};
use image::{
    DynamicImage, ImageBuffer, ImageFormat, Rgb,
    imageops::{ColorMap, dither},
};
use once_cell::sync::Lazy;
use rand::{Rng, rng};
use std::{collections::HashMap, fs, io::Write};

static PALETTE: Palette = Palette {
    colors: [
        Rgb([0x00, 0x00, 0x00]),
        Rgb([0xFF, 0xFF, 0xFF]),
        Rgb([0x00, 0x00, 0xFF]),
        Rgb([0x00, 0xFF, 0x00]),
        Rgb([0xFF, 0x00, 0x00]),
        Rgb([0x00, 0xFF, 0xFF]),
        Rgb([0xFF, 0x00, 0xFF]),
        Rgb([0xFF, 0xFF, 0x00]),
        Rgb([0xC0, 0xC0, 0xC0]),
        Rgb([0x80, 0x80, 0x80]),
        Rgb([0x80, 0x00, 0x00]),
        Rgb([0x80, 0x80, 0x00]),
        Rgb([0x00, 0x80, 0x00]),
        Rgb([0x00, 0x80, 0x80]),
        Rgb([0x00, 0x00, 0x80]),
        Rgb([0x80, 0x00, 0x80]),
    ],
};

static ENCODES: Lazy<HashMap<Vec<u8>, String>> = Lazy::new(|| {
    PALETTE
        .colors
        .iter()
        .enumerate()
        .map(|(idx, rgb)| (vec![rgb[0], rgb[1], rgb[2]], format!("{:04b}", idx)))
        .collect()
});

static DECODES: Lazy<HashMap<String, Vec<u8>>> =
    Lazy::new(|| ENCODES.iter().map(|i| (i.1.clone(), i.0.clone())).collect());

struct Palette {
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

fn open_img(path: &str) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, image::ImageError> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = image::ImageReader::open(path)?.decode()?.into_rgb8();
    Ok(img)
}

fn save_img(img: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<(), image::ImageError> {
    match DynamicImage::ImageRgb8(img).save_with_format("decoded.png", ImageFormat::Png) {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}

fn bits_to_bytes(bytes: &[bool]) -> Vec<u8> {
    bytes
        .chunks(8)
        .map(|byte| {
            byte.iter().enumerate().fold(
                0_u8,
                |acc, (i, &bit)| {
                    if bit { acc | (1 << (7 - i)) } else { acc }
                },
            )
        })
        .collect()
}

fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for i in 0..8 {
            bits.push((byte & (1 << (7 - i))) != 0);
        }
    }
    bits
}

fn bytes_to_base64url(bytes: &[u8]) -> String {
    base64_url::encode(bytes)
}

fn base64url_to_bytes(code: &str) -> Option<Vec<u8>> {
    base64_url::decode(code).ok()
}

fn gen_key() -> String {
    let mut rng = rng();
    bytes_to_base64url(
        (0..16)
            .map(|_| rng.random())
            .collect::<Vec<u8>>()
            .as_slice(),
    )
}

fn pack_dimensions(a: u16, b: u16) -> [u8; 3] {
    let combined: u32 = ((a as u32) << 12) | (b as u32);

    [
        ((combined >> 16) & 0xFF) as u8,
        ((combined >> 8) & 0xFF) as u8,
        (combined & 0xFF) as u8,
    ]
}

fn unpack_dimensions(bytes: &[u8]) -> (u32, u32) {
    let combined: u32 = ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32);

    let height = (combined >> 12) as u16 & 0xFFF;
    let width = (combined & 0xFFF) as u16;

    (height as u32, width as u32)
}

fn decode(bits: Vec<bool>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let (width, height) = unpack_dimensions(&bits_to_bytes(&bits.clone()[..=23]));
    let bytes: Vec<u8> = bits[24..]
        .chunks_exact(4)
        .flat_map(|i| {
            DECODES[&i
                .iter()
                .map(|b| std::char::from_digit(if *b { 1 } else { 0 }, 2).unwrap())
                .collect::<String>()]
                .clone()
        })
        .collect();
    if let Some(img) = ImageBuffer::from_raw(width, height, bytes.to_owned()) {
        img
    } else {
        panic!("Error: wrong code or base64url_key");
    }
}

fn encode(mut img: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Vec<u8> {
    dither(&mut img, &PALETTE);
    let bytes = &img.clone().into_raw();

    let pixel_bits: Vec<bool> = bytes
        .chunks_exact(3)
        .map(|rgb| {
            ENCODES
                .get(&rgb.to_vec())
                .unwrap_or_else(|| panic!("{:?}", rgb))
                .to_owned()
        })
        .collect::<Vec<String>>()
        .join("")
        .chars()
        .map(|c| c == '1')
        .collect::<Vec<bool>>();
    let pixel_bytes = bits_to_bytes(pixel_bits.as_slice());
    let mut output_bytes = Vec::with_capacity(pixel_bytes.len() + 3);
    let (height, width) = &img.dimensions();
    if *height > 4095 {
        println!("Warning: heigth is higher than 4095 pixels");
    }
    if *width > 4095 {
        println!("Warning: width is higher than 4095 pixels")
    }
    output_bytes.extend_from_slice(&pack_dimensions(*height as u16, *width as u16));
    output_bytes.extend_from_slice(pixel_bytes.as_slice());
    output_bytes
}

fn decrypt(cipher: &[u8], key: &str) -> Option<Vec<u8>> {
    let byte_key = base64url_to_bytes(key)?;
    let ff1 = FF1::<Aes128>::new(byte_key.as_slice(), 2).unwrap();
    Some(
        ff1.decrypt(&[], &BinaryNumeralString::from_bytes_le(cipher))
            .unwrap()
            .to_bytes_le(),
    )
}

fn encrypt(bytes: &[u8], key: &str) -> Option<Vec<u8>> {
    let byte_key = base64url_to_bytes(key)?;
    let ff1 = FF1::<Aes128>::new(byte_key.as_slice(), 2).unwrap();
    Some(
        ff1.encrypt(&[], &BinaryNumeralString::from_bytes_le(bytes))
            .unwrap()
            .to_bytes_le(),
    )
}

fn write_file(bytes: &[u8]) {
    let mut file = fs::File::create("encoded.bin").unwrap();
    let _ = file.write_all(bytes);
}

//using result as enum for two "Ok()" dtypes
fn do_input(
    file_read: bool,
    encode: bool,
    input: &str,
) -> Result<Result<ImageBuffer<Rgb<u8>, Vec<u8>>, Vec<u8>>, String> {
    if file_read {
        if encode {
            if let Ok(img) = open_img(input) {
                Ok(Ok(img))
            } else {
                Err("Error: File path does not exists".to_string())
            }
        } else if let Ok(bytes) = fs::read(input) {
            Ok(Err(bytes))
        } else {
            Err("Error: File path does not exists".to_string())
        }
    } else if let Some(bytes) = base64url_to_bytes(input) {
        Ok(Err(bytes))
    } else {
        Err("Error: Invalid code".to_string())
    }
}

fn do_decode(bytes: Vec<u8>, key: Option<&str>) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, String> {
    if let Some(some_key) = key {
        if let Some(decrypted) = decrypt(bytes.as_slice(), some_key) {
            Ok(decode(bytes_to_bits(decrypted.as_slice())))
        } else {
            Err("Error: Invalid key".to_string())
        }
    } else {
        Ok(decode(bytes_to_bits(bytes.as_slice())))
    }
}

fn do_encode(img: ImageBuffer<Rgb<u8>, Vec<u8>>, key: Option<&str>) -> Result<Vec<u8>, String> {
    let mut encoded = encode(img);

    if key.is_some() {
        encoded = if let Some(encrypted) = encrypt(encoded.as_slice(), key.unwrap()) {
            encrypted
        } else {
            return Err("Error: Invalid key".to_string());
        }
    }
    Ok(encoded)
}

//using result as enum for two "Ok()" dtypes
fn do_output(file_output: bool, data: Result<Vec<u8>, ImageBuffer<Rgb<u8>, Vec<u8>>>) -> String {
    match data {
        Ok(bytes) => {
            if file_output {
                write_file(bytes.as_slice());
                "File saved".to_string()
            } else {
                bytes_to_base64url(bytes.as_slice())
            }
        }
        Err(img) => {
            let _ = save_img(img.clone());
            "Image saved".to_string()
        }
    }
}
fn help() {
    println!("Usage: exe [options(as single word)] [file_path | base64url_code | input_string] [base64url_key](opt)

    options:
        - e - encode mode (always first option): input - existing [file_path], output - created ./encoded.bin or stdout error text
        - d - decode mode (always first option): input - existing [file_path], output - stdout decode text or stdout error text
        - ee - encode-encrypt mode (always first option): input - existing [file_path] and [base64url_key], output - created ./encoded.bin or stdout error text
        - dd - decode-decrypt mode (always first option): input - existing [file_path] and [base64url_key], output - created ./decoded.png or stdout error text
        - sw - string write: replaces output .bin file of a decoding operation with a base64url_code
        - sr - string read: replaces input .bin [file_path] for decoding operation with a [base64url_code]
        - g - 16bytes base64url key gen
")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        help();
        return;
    } else if args[1] == "g" {
        println!("{}", gen_key());
        return;
    }
    let options = args[1].clone();
    let input_bytes = match do_input(
        !options.contains("sr"),
        options.contains("e"),
        args[2].as_str(),
    ) {
        Ok(data) => data,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };
    let key = if options.contains("ee") || options.contains("dd") {
        Some(args[3].as_str())
    } else {
        None
    };

    //using result as enum for two "Ok()" dtypes
    let processed_data = if options.starts_with("e") {
        Ok(match do_encode(input_bytes.unwrap(), key) {
            Ok(data) => data,
            Err(err) => {
                println!("{}", err);
                return;
            }
        })
    } else {
        Err(match do_decode(input_bytes.unwrap_err(), key) {
            Ok(text) => text,
            Err(err) => {
                println!("{}", err);
                return;
            }
        })
    };
    println!("{}", do_output(!options.contains("sw"), processed_data));
}
