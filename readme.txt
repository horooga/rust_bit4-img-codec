tool for coding-encoding images with 4 bits per pixel efficiency

!!!Warning!!!
during encoding input image is rewritten with a 4bit palette using Floyd-Steinberg dithering

Tool features:
    - 16-color palette
    - 4 bits per symbol encoding due to reduced alphabet
    - AES128 encryption-decryption (length-preserving) available
    - (in development) opportunity to use strings of base64url symbols as an alternative to encoded binary files

Usage: exe [options(as single word)] [file_path | base64url_code] [base64url_key](opt)

    options:
        - e - encode mode (always first option): input - existing [file_path], output - created ./encoded.bin or stdout error text
        - d - decode mode (always first option): input - existing [file_path], output - stdout decode text or stdout error text
        - ee - encode-encrypt mode (always first option): input - existing [file_path] and [base64url_key], output - created ./encoded.bin or stdout error text
        - dd - decode-decrypt mode (always first option): input - existing [file_path] and [base64url_key], output - stdout decode text or stdout error text
        - (in development) sw - string write: replaces output .bin file of a decoding operation with a base64url_code
        - (in development) sr - string read: replaces input .bin or .txt [file_path] for decoding or encoding operation with a [input_string] or a [base64url_code] correspondingly
        - g - 16bytes base64url key gen

Some details:
    - max image size is 4095x4095, so the first three bytes of encoded is the image dimensions
    - compiled exe will be named "codec4" as specified in Cargo.toml

