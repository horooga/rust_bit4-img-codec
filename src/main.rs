use aes::Aes128;
use fpe::ff1::{BinaryNumeralString, FF1};
use image::{
    imageops::{dither, ColorMap},
    DynamicImage, ImageBuffer, ImageFormat, Rgb,
};
use rand::{rng, Rng};
use std::{fs, io::Write, process::exit, sync::Arc, thread};

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

static COLORS: [[u8; 3]; 16] = [
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
    [0x80, 0x00, 0x80],];

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

fn get_encode(pixel_target: &[u8]) -> u8 {
    for (idx, pixel) in COLORS.iter().enumerate() {
        if *pixel == pixel_target {
            return idx as u8;
        }
    }
    0
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

fn write_file(bytes: &[u8]) {
    let mut file = fs::File::create("encoded.bin").unwrap();
    let _ = file.write_all(bytes);
}

fn bytes_to_base64url(bytes: &[u8]) -> String {
    base64_url::encode(bytes)
}

fn base64url_to_bytes(code: &str) -> Option<Vec<u8>> {
    base64_url::decode(code).ok()
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

fn byte_to_codes(byte: u8) -> [u8; 2] {
    [byte << 4, byte & 0x0F]
}

fn codes_to_byte(first: u8, second: u8) -> u8 {
    first << 4 | second
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

fn decode(bytes: Vec<u8>) -> Vec<u8> {
    bytes.into_iter().flat_map(|byte| {
        let [idx1, idx2] = byte_to_codes(byte);
        let mut byte = [0; 8];
        byte[..=4].copy_from_slice(&COLORS[idx1 as usize]);
        byte[4..].copy_from_slice(&COLORS[idx2 as usize]);
        byte
    })
    .collect()
}

fn encode(bytes: &[u8]) -> Vec<u8> {
    let mut output_bytes = Vec::with_capacity(bytes.len() / 6);
    let mut code_prev = None;
    for rgb in bytes.chunks_exact(3) {
        let code_curr = get_encode(rgb);
        if let Some(code_prev) = code_prev {
            output_bytes.push(codes_to_byte(code_prev, code_curr));
        } else {
            code_prev = Some(code_curr);
        }
    }
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

fn process_decode(mut chunk: Vec<u8>, key_opt: Option<String>) -> Vec<u8> {
    if let Some(key) = key_opt {
        if let Some(decrypted) = decrypt(chunk.as_slice(), key.as_str()) {
            chunk = decrypted
        } else {
            eprintln!("Error: invalid code or key");
            exit(1);
        }
    }
    decode(chunk)
}

fn process_encode(mut chunk: Vec<u8>, key_opt: Option<String>) -> Vec<u8> {
    chunk = encode(chunk.as_slice());
    if let Some(key) = key_opt {
        if let Some(encrypted) = encrypt(chunk.as_slice(), key.as_str()) {
            chunk = encrypted
        } else {
            eprintln!("Error: invalid code or key");
            exit(1);
        }
    }
    chunk
}

//using result as enum for two "Ok()" dtypes
fn do_input(encode: bool, input: &str) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, Vec<u8>> {
    if encode {
        if let Ok(img) = open_img(input) {
            Ok(img)
        } else {
            eprintln!("Error: File path does not exists");
            exit(1);
        }
    } else if let Ok(bytes) = fs::read(input) {
        Err(bytes)
    } else {
        eprintln!("Error: File path does not exists");
        exit(1);
    }
}

fn do_decode(bytes: Vec<u8>, key: Option<String>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let data = Arc::new(&bytes[3..]);
    let cpus_amount = num_cpus::get();
    let chunk_size = data.len() / cpus_amount;
    let mut handles = Vec::with_capacity(cpus_amount);
    for i in 0..cpus_amount {
        let data = Arc::clone(&data);
        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size).min(data.len());
        let chunk: Vec<u8> = data[start..end].to_vec();
        let key_bind = key.clone();

        let handle = thread::Builder::new()
            .name(format!("processing-{i}/{cpus_amount}"))
            .spawn(move || process_decode(chunk, key_bind))
            .unwrap();
        handles.push(handle);
    }
    let (width, height) = unpack_dimensions(&bytes[..=2]);
    let mut results = Vec::new();
    for handle in handles {
        let processed_chunk = handle.join().unwrap();
        results.extend(processed_chunk);
    }
    ImageBuffer::from_raw(width, height, results).unwrap()
}

fn do_encode(mut img: ImageBuffer<Rgb<u8>, Vec<u8>>, key: Option<String>) -> Vec<u8> {
    dither(&mut img, &PALETTE);
    let (height, width) = img.dimensions();
    if height > 4095 {
        eprintln!("Error: height is higher than 4095 pixels");
        exit(1);
    }
    if width > 4095 {
        eprintln!("Error: width is higher than 4095 pixels");
        exit(1);
    }
    let data = Arc::new(img.into_raw());
    let cpus_amount = num_cpus::get();
    let chunk_size = data.len() / cpus_amount;
    let mut handles = Vec::with_capacity(cpus_amount);
    for i in 0..cpus_amount {
        let data = Arc::clone(&data);
        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size).min(data.len());
        let chunk: Vec<u8> = data[start..end].to_vec();
        let key_bind = key.clone();

        let handle = thread::Builder::new()
            .name(format!("processing-{i}/{cpus_amount}"))
            .spawn(move || process_encode(chunk, key_bind))
            .unwrap();
        handles.push(handle);
    }
    let mut results = Vec::new();
    for handle in handles {
        let processed_chunk = handle.join().unwrap();
        results.extend(processed_chunk);
    }
    let mut output_bytes = Vec::with_capacity(results.len() + 3);

    output_bytes.extend_from_slice(&pack_dimensions(height as u16, width as u16));
    output_bytes.extend_from_slice(results.as_slice());
    output_bytes
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
    println!("Usage: exe [options(as single word)] [file_path] [base64url_key(opt)]

    options:
        - e - encode mode (always first option): input - existing [file_path], output - created ./encoded.bin or stderr
        - d - decode mode (always first option): input - existing [file_path], output - stdout decode text or stderr
        - ee - encode-encrypt mode (always first option): input - existing [file_path] and [base64url_key], output - created ./encoded.bin or stderr
        - dd - decode-decrypt mode (always first option): input - existing [file_path] and [base64url_key], output - created ./decoded.png or stderr
        - g - 16bytes base64url stdout key gen
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
    let input_bytes = do_input(options.contains("e"), args[2].as_str());
    let key = if options.contains("ee") || options.contains("dd") {
        Some(args[3].clone())
    } else {
        None
    };

    //using result as enum for two "Ok()" dtypes
    let processed_data = if options.starts_with("e") {
        Ok(do_encode(input_bytes.unwrap(), key))
    } else {
        Err(do_decode(input_bytes.unwrap_err(), key))
    };
    println!("{}", do_output(!options.contains("sw"), processed_data));
}
