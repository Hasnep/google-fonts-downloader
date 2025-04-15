# google-fonts-downloader

A command-line tool to download font files and their associated CSS from Google Fonts.

## Installation

### Nix

If you use Nix, you can run the tool with `nix run`:

```shell
nix run github:hasnep/google-fonts-downloader
```

or add it as an input to your Nix flake:

```nix
{
  inputs = {
    nixpkgs.url = "...";
    google-fonts-downloader = {
      url = "github:Hasnep/google-fonts-downloader";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = ...
}
```

### Build from source

You can also clone the repository and install it using `cargo`.

```shell
git clone https://github.com/hasnep/google-fonts-downloader.git
cd google-fonts-downloader
cargo install --path .
```

## Usage

To download a font from a URL such as `https://fonts.googleapis.com/css2?family=Roboto&display=swap`, run the `google-fonts-downloader` tool:

```shell
google-fonts-downloader [OPTIONS] <URL>...
```

- `--overwrite` (`-w`) - Overwrite existing files instead of skipping them.
- `--quiet` (`-q`) - Suppress all informational output.
- `--verbose` (`-v`) - Show detailed information about each font being processed.
- `--output <DIR>` (`-o`) - Specify the output directory for downloaded files, defaults to `./fonts`.
- `--fonts-prefix <PREFIX>` - Set the path to the fonts relative to the CSS files, defaults to `./`.
