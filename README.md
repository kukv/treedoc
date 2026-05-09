# treedoc

A `tree(1)`-style directory listing with one extra trick: each entry can carry a
short comment, kept alongside the project in a `.treedoc.yaml` sidecar file.

Useful for README directory maps, architecture docs, onboarding guides, and
blog posts that need to explain a repo layout.

```
my-project/
├── src/         # application code
│   ├── lib.rs   # core logic
│   └── main.rs  # entry point
├── Cargo.toml   # dependency manifest
└── README.md    # documentation
```

## Install

### From crates.io

```sh
cargo install treedoc
```

### Pre-built binaries

Grab the archive for your platform from the
[GitHub Releases](https://github.com/kukv/treedoc/releases) page.

### From source

```sh
git clone https://github.com/kukv/treedoc.git
cd treedoc
cargo install --path .
```

## Usage

### Show a tree

```sh
treedoc            # current directory
treedoc src        # a specific path
treedoc -L 2       # limit recursion depth
treedoc -a         # include dotfiles
treedoc --no-ignore  # don't honour .gitignore
```

By default, `.gitignore` rules are respected, dotfiles are hidden, and output
is colored when stdout is a TTY (`NO_COLOR` and `--no-color` are honoured).

### Manage comments

```sh
treedoc init                          # create .treedoc.yaml with empty comments
treedoc set src/main.rs "entry point" # set or update one entry
treedoc edit                          # open .treedoc.yaml in $EDITOR
```

`treedoc init` refuses to overwrite an existing sidecar; pass `--force` to
replace it.

### Output formats

```sh
treedoc --format console   # default: colored TTY output
treedoc --format plain     # uncolored
treedoc --format markdown  # wrapped in a ``` code fence
```

Useful for pasting into a README:

```sh
treedoc --format markdown >> README.md
```

## Sidecar format

`.treedoc.yaml` is a flat mapping from relative path to comment string:

```yaml
src: "application code"
src/main.rs: "entry point"
src/lib.rs: "core logic"
Cargo.toml: "dependency manifest"
README.md: "documentation"
```

- Paths use forward slashes regardless of the host OS.
- A trailing slash on directory keys is optional (`src` and `src/` both match).
- The sidecar file itself is hidden from the rendered tree.
- Empty-string comments are kept in the YAML (so `treedoc init` can seed them)
  but are not printed.

## Options

```
Usage: treedoc [OPTIONS] [PATH]
       treedoc <COMMAND>

Commands:
  show  Render the tree (default)
  init  Create a .treedoc.yaml populated with empty comments for every entry
  set   Set or update a comment for a single path
  edit  Open .treedoc.yaml in $EDITOR

Options:
  -L, --depth <DEPTH>      Descend at most this many levels of directories
  -a, --all                Show entries whose names begin with a dot
      --no-ignore          Do not honour .gitignore rules
      --no-color           Disable colored output
      --format <FORMAT>    Output format [default: console]
                           [possible values: console, plain, markdown]
  -h, --help               Print help
  -V, --version            Print version
```

## License

[MIT](LICENSE)
