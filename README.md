# exifrename | [![Tests](https://img.shields.io/github/actions/workflow/status/cdown/exifrename/ci.yml?branch=master)](https://github.com/cdown/exifrename/actions?query=branch%3Amaster)

exifrename renames or copies files based on EXIF data.

## Usage

    exifrename -f FORMAT FILES

You can also see what would be changed first using `--dry-run`. See `--help`
for more options.

How to specify `FORMAT` can also be found in `--help`.

## Performance

exifrename has a strong focus on performance. On a sample modern laptop with a
mid-spec SSD, we take 0.02 seconds to produce new names for over 5000 files
with the format `{year}{month}{day}_{hour}{minute}{second}`.
