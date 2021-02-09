# Raytracing in Rust

This program is the product of the first two [Ray Tracing in One Weekend books][books].
I wrote it in Rust instead of in C++, as the books do, and I added several special features of my own.

[books]: https://raytracing.github.io/

Special features include:

- Multi-core rendering
- Progress bar and estimated time until completion
- Importing `.obj` meshes
- A JSON5-based format for storing scenes
- .PART files which cache partially-rendered scenes for crash recovery

![cornell box][cover_1.jpg]

![knight mesh][cover_2.jpg]

## Usage

```
Raytracing in a weekend!

USAGE:
    raytracer [FLAGS] [OPTIONS] <input-file> <output-file>

FLAGS:
        --ascii-symbols-only    Do not use non-ASCII symbols
    -h, --help                  Prints help information
        --no-part-file          Don't save partial progress in a part file in case of a crash
    -q, --quiet                 No informational messages printed to stderr
        --recover-corrupt       Try to read as much of a corrupted part file as possible
    -V, --version               Prints version information

OPTIONS:
    -m, --max-bounces <max-bounces>              Maximum number of bounces for any ray [default: 50]
    -p, --progress-bar-len <progress-bar-len>    Manually set length of progress bar
    -s, --ray-samples <ray-samples>              Rays per pixel [default: 100]
    -r, --recover-from <recover-from>            Recover from part file
    -w, --width <width>                          Output image width [default: 600]

ARGS:
    <input-file>     Config file
    <output-file>    Output file
```

As input, use a json configuration file such as the ones in the `examples/` directory.
As the JSON5 standard is used, comments, unquoted field names, and trailing commas are all permitted.

The configuration spec is not documented, but between the examples and the deserialization code in `src/config.rs`, it is possible to more or less figure it out.
I might have documented it properly if I expected anyone to actually try to use it, haha.