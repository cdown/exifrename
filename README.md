# exifrename | [![Tests](https://img.shields.io/github/actions/workflow/status/cdown/exifrename/ci.yml?branch=master)](https://github.com/cdown/exifrename/actions?query=branch%3Amaster)

exifrename renames or copies files based on EXIF data.

## Installation

    cargo install exifrename

## Usage

See `--help` for more information on how to populate `FORMAT` and other
available options, but the basic invocation is:

    exifrename -f FORMAT FILES

You can also see what would be changed first using `--dry-run`.

## Performance

exifrename has a strong focus on performance. On a sample modern laptop with a
mid-spec SSD, we take 0.02 seconds to produce new names for over 5000 files
with the format `{year}{month}{day}_{hour}{minute}{second}`.
