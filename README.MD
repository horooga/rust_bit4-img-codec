Tool for images encoding and decoding with 4 bits per pixel efficiency

**!!!Warning!!!**

during encoding input image is rewritten with a 4bit palette

**Tool features:**

- 16-color palette (![000000](https://placehold.co/15x15/000000/000000.png), ![808080](https://placehold.co/15x15/808080/808080.png), ![c0c0c0](https://placehold.co/15x15/c0c0c0/c0c0c0.png), ![ffffff](https://placehold.co/15x15/ffffff/ffffff.png), ![000080](https://placehold.co/15x15/000080/000080.png), ![008000](https://placehold.co/15x15/008000/008000.png), ![008080](https://placehold.co/15x15/008080/008080.png), ![800000](https://placehold.co/15x15/800000/800000.png), ![800080](https://placehold.co/15x15/800080/800080.png), ![808000](https://placehold.co/15x15/808000/808000.png), ![0000ff](https://placehold.co/15x15/0000ff/0000ff.png), ![00ff00](https://placehold.co/15x15/00ff00/00ff00.png), ![00ffff](https://placehold.co/15x15/00ffff/00ffff.png), ![ff0000](https://placehold.co/15x15/ff0000/ff0000.png), ![ff00ff](https://placehold.co/15x15/ff00ff/ff00ff.png), ![ffff00](https://placehold.co/15x15/ffff00/ffff00.png)
- 4 bits per symbol encoding due to reduced palette
- AES128 encryption-decryption (length-preserving) available
- Floyd Steinberg dithering available

**Build:**

    cargo build --release

**Usage:**

    exe [options] [input_file_path] [output_file_path] [base64url_key(opt)]

    options:
        - e - encode mode: input - existing [input_file_path], output - saved [output_file_path] or stderr
        - p - enable dither processing for image encoding (can be disabled when re-encoding of already dithered images)
        - d - decode mode: input - existing [input_file_path], output - saved [output_file_path] or stderr
        - ee - encode-encrypt mode: input - existing [input_file_path] and [base64url_key], output - saved [output_file_path] or stderr
        - dd - decode-decrypt mode: input - existing [input_file_path] and [base64url_key], output - saved [output_file_path] or stderr
        - g - 16bytes base64url stdout key gen (doesn not need any input)

"**Examples:**

- input image:
![image.png](./image.png)

- target/release/rust_bit4-img-codec eep image.png encoded.bin GqhLxZ5fhzDbc9pCpoKN4g:
    *saved ./encoded.bin file

- target/release/rust_bit4-img-codec dd encoded.bin decoded.png GqhLxZ5fhzDbc9pCpoKN4g:
    *saved ./decoded.png image

- output image:
![decoded.png](./decoded.png)


**Some details:**

- max image size is 4095x4095, so the first three bytes of encoded is the image dimensions
- dithering can damage images with many areas of the same color
