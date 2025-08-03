# `image_writer_rs`

## Warning

**WARNING WARNING WARNING**

The code assumes:

* You run it on Linux
* The `/dev/disk/by-id/` directory exists and is properly populated
* The `/sys/block/` directory exists and is properly populated

## Features

- Autodetect any writable USB mass storage with non-zero capacity, show choice when more than 1 detected.
- Show warning with exact detected device name and countdown before writing an image - you have 10 seconds to `^C` if
  you
  change your mind.
- Warn if the disk image appears to not be bootable (missing `0x55AA` signature in the first sector)
- Abort if (eventually decompressed) disk image is not a multiple of 512 bytes.
- Support various disk image types (extension is case-insensitive):
    - Uncompressed: `.ISO`, `.FS`, `.IMG`, `.IMA`, `.DD`, `.BIN`, `.RAW`
    - Compressed: `.BZ2`, `.BZIP2`, `.GZ`, `.GZIP`, `.XZ`, `.LZMA`, `.PIXZ`, `.ZST`, `.ZSTD`, `.LZ4`
- Write directly to the device, bypassing cache.
- Verify written data against the original image.

## TODO

- [x] Check if image fits on media.
- [ ] Optionally fix the secondary GPT partition table to end of written media (warning - will invalidate checksum as it
  must modify the primary GPT partition)
