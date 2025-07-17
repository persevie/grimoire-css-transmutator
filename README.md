# Grimoire CSS Transmutator

A tool for seamlessly transforming standard CSS into [Grimoire CSS](https://github.com/persevie/grimoire-css) spells.
`grimoire_css_transmutator` is available both as a CLI tool and as a Rust library crate. It helps you convert your CSS files or inline CSS content into a structured JSON format suitable for use with the Grimoire CSS system.

## Features

- **Batch conversion**: Process multiple CSS files or patterns at once.
- **Inline content**: Convert CSS provided as a string.
- **Flexible output**: Save results to a file or print to stdout.
- **Oneliner support**: Optionally include a one-line representation for each class.

## Installation

You need [Rust](https://www.rust-lang.org/tools/install) installed.

```sh
cargo install --path .
```

## Usage

```sh
grimoire_css_transmutator [OPTIONS] [INPUT]
```

### Options

- `-p`, `--paths` Process comma-separated list of CSS file paths or glob patterns
- `-c`, `--content` Process CSS content provided as a string
- `-o`, `--output` Specify output file (default: `./grimoire/transmuted.json`)
- `-l`, `--with-oneliner` Include `oneliner` property in output (default: disabled)
- `-h`, `--help` Display help message

### Examples

Convert multiple CSS files:

```sh
grimoire_css_transmutator -p styles.css,components.css
```

Convert all CSS files in a directory:

```sh
grimoire_css_transmutator -p "src/**/*.css"
```

Convert inline CSS content:

```sh
grimoire_css_transmutator -c '.button { color: red; }' -l
```

Custom output file:

```sh
grimoire_css_transmutator -p '*.css' -o custom_output.json --with-oneliner
```

## Output

The output is a JSON file (or stdout) containing an array of objects, each representing a CSS class and its corresponding Grimoire CSS spells. Example:

```json
{
  "scrolls": [
    {
      "name": "button",
      "spells": ["color=red"],
      "oneliner": ".button { color: red; }"
    }
  ]
}
```

## Library and CLI Usage

You can use `grimoire_css_transmutator` both as a command-line tool and as a Rust library crate.

### As a library

Add `grimoire_css_transmutator_lib` to your `Cargo.toml` dependencies and use it in your Rust code:

```rust
use grimoire_css_transmutator_lib::transmute_from_content;
let (duration, json) = transmute_from_content(".foo { color: blue; }", false).unwrap();
println!("{}", json);
```

## License

MIT

## Links

- [Grimoire CSS](https://github.com/persevie/grimoire-css)
- [Documentation](https://docs.rs/grimoire_css_transmutator)
- [Repository](https://github.com/persevie/grimoire_css_transmutator)
