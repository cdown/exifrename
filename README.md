# exifrename | [![Tests](https://img.shields.io/github/actions/workflow/status/cdown/exifrename/ci.yml?branch=master)](https://github.com/cdown/exifrename/actions?query=branch%3Amaster)

exifrename renames or copies files based on EXIF data.

## Installation

    cargo install exifrename

## Usage

See `--help` for more information on how to populate `-f FORMAT` and other
available options. An example invocation is:

    % exifrename --copy \
    >  -f "$HOME/Photos/{camera_make}/{year}{month}{day}_{hour}{minute}{second}" \
    >  /mnt/sdcard/*.jpg
    /mnt/sdcard/P0001.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184547_0.jpg
    /mnt/sdcard/P0002.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184547_1.jpg
    /mnt/sdcard/P0003.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184548_0.jpg
    /mnt/sdcard/P0004.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184548_1.jpg
    /mnt/sdcard/P0005.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184548_2.jpg
    /mnt/sdcard/P0006.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184548_3.jpg
    /mnt/sdcard/P0007.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184551.jpg
    /mnt/sdcard/P0008.jpg -> /home/cdown/Photos/FUJIFILM/20230126_184802.jpg
    [...]

You can also see what would be changed first using `--dry-run`.

## Performance

exifrename has a strong focus on performance. On a sample modern laptop with a
mid-spec SSD, we take ~0.02 seconds to produce new names for over 5000 files
with the format `{year}{month}{day}_{hour}{minute}{second}`.
