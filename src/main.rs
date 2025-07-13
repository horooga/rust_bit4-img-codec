use aes::Aes128;
use fpe::ff1::{BinaryNumeralString, FF1};
use image::{ImageBuffer, Rgb, imageops::dither};
use std::{fs, process::exit, sync::Arc, thread};

mod misc;
use misc::*;

fn encode(bytes: &[u8]) -> Vec<u8> {
    let mut output_bytes = Vec::new();
    let mut code_prev = None;
    for rgb in bytes.chunks(3) {
        let code_curr = get_encode(rgb);
        if let Some(code_prev_some) = code_prev {
            output_bytes.push(codes_to_byte(code_prev_some, code_curr));
            code_prev = None
        } else {
            code_prev = Some(code_curr);
        }
    }
    if let Some(code_prev_some) = code_prev {
        output_bytes.push(codes_to_byte(code_prev_some, 0));
    }
    output_bytes
}

fn decode(bytes: Vec<u8>) -> Vec<u8> {
    bytes
        .into_iter()
        .flat_map(|byte| {
            let [idx1, idx2] = byte_to_codes(byte);
            let first = COLORS[idx1 as usize];
            let second = COLORS[idx2 as usize];
            [
                first[0], first[1], first[2], second[0], second[1], second[2],
            ]
        })
        .collect()
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

fn decrypt(cipher: &[u8], key: &str) -> Option<Vec<u8>> {
    let byte_key = base64url_to_bytes(key)?;
    let ff1 = FF1::<Aes128>::new(byte_key.as_slice(), 2).unwrap();
    Some(
        ff1.decrypt(&[], &BinaryNumeralString::from_bytes_le(cipher))
            .unwrap()
            .to_bytes_le(),
    )
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

//using result as enum for two "Ok()" dtypes
fn do_input(encode: bool, input: &str) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, Vec<u8>> {
    if encode {
        return match open_img(input) {
            Ok(img) => Ok(img),
            Err(err) => {
                eprintln!("Error: {}", err);
                exit(1);
            }
        };
    }
    match fs::read(input) {
        Ok(bytes) => Err(bytes),
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    }
}

fn do_encode(
    mut img: ImageBuffer<Rgb<u8>, Vec<u8>>,
    key_opt: Option<String>,
    dithering: bool,
) -> Vec<u8> {
    if dithering {
        dither(&mut img, &PALETTE);
    } else {
        quantization(&mut img);
    }
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
        let key_bind = key_opt.clone();

        let handle = thread::Builder::new()
            .name(format!("processing-{i}/{cpus_amount}"))
            .spawn(move || process_encode(chunk, key_bind))
            .unwrap();
        handles.push(handle);
    }
    let mut result = Vec::new();
    for handle in handles {
        let processed_chunk = handle.join().unwrap();
        result.extend(processed_chunk);
    }
    let mut output_bytes = Vec::with_capacity(result.len() + 3);

    output_bytes.extend_from_slice(&pack_dimensions(height as u16, width as u16));
    output_bytes.extend_from_slice(result.as_slice());
    output_bytes
}

fn do_decode(bytes: Vec<u8>, key_opt: Option<String>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let data = Arc::new(&bytes[3..]);
    let cpus_amount = num_cpus::get();
    let chunk_size = data.len() / cpus_amount;
    let mut handles = Vec::with_capacity(cpus_amount);
    for i in 0..cpus_amount {
        let data = Arc::clone(&data);
        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size).min(data.len());
        let chunk: Vec<u8> = data[start..end].to_vec();
        let key_bind = key_opt.clone();

        let handle = thread::Builder::new()
            .name(format!("processing-{i}/{cpus_amount}"))
            .spawn(move || process_decode(chunk, key_bind))
            .unwrap();
        handles.push(handle);
    }
    let (width, height) = unpack_dimensions(&bytes[..=2]);
    let mut result = Vec::new();
    for handle in handles {
        let processed_chunk = handle.join().unwrap();
        result.extend(processed_chunk);
    }
    ImageBuffer::from_raw(width, height, result).unwrap()
}

//using result as enum for two "Ok()" dtypes
fn do_output(
    data: Result<Vec<u8>, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    output_file_path: &str,
) -> String {
    match data {
        Ok(bytes) => {
            write_file(bytes.as_slice(), output_file_path);
            exit(1);
        }
        Err(img) => {
            _ = save_img(img.clone(), output_file_path);
            exit(1);
        }
    }
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
        Some(args[4].clone())
    } else {
        None
    };

    //using result as enum for two "Ok()" dtypes
    let processed_data = if options.contains("e") {
        Ok(do_encode(input_bytes.unwrap(), key, options.contains("p")))
    } else {
        Err(do_decode(input_bytes.unwrap_err(), key))
    };
    println!("{}", do_output(processed_data, args[3].as_str()));
}
