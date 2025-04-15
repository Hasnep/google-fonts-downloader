use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::{command, value_parser, Arg, ArgAction};
use regex::Regex;
use std::str;

struct Args {
    urls: Vec<String>,
    output_dir: PathBuf,
    overwrite: bool,
    quiet: bool,
    verbose: bool,
    fonts_prefix_in_css: String,
}

struct FontInfo {
    family: String,
    style: String,
    weight: String,
    stretch: String,
    display: String,
    url: String,
    format: String,
}

impl FontInfo {
    fn get_extension(&self) -> Option<&'static str> {
        match self.format.as_str() {
            "truetype" => Some("ttf"),
            _ => None,
        }
    }

    fn get_font_filename(&self) -> Option<String> {
        self.get_extension()
            .map(|ext| format!("{}-{}-{}.{}", self.family, self.weight, self.style, ext))
    }

    fn get_css_filename(&self) -> String {
        format!("{}-{}-{}.css", self.family, self.weight, self.style)
    }

    fn generate_css(&self, filename: &str) -> String {
        format!(
            "@font-face {{\n  font-family: '{}';\n  font-style: {};\n  font-weight: {};\n  font-stretch: {};\n  font-display: {};\n  src: url({}) format('{}');\n}}\n",
            self.family, self.style, self.weight, self.stretch, self.display, filename, self.format
        )
    }
}

fn parse_args() -> Args {
    let matches = command!()
        .arg(
            Arg::new("overwrite")
                .short('w')
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help("Whether to overwrite existing files."),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .action(ArgAction::SetTrue)
                .help("Suppress informational output, including verbose output."),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help("Enable verbose output."),
        )
        .arg(
            Arg::new("fonts-prefix")
                .long("fonts-prefix")
                .default_value("./")
                .help("Prefix for font files in CSS output."),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_parser(value_parser!(PathBuf))
                .default_value("./fonts")
                .help("The name of the output directory, will be created if it doesn't exist."),
        )
        .arg(
            Arg::new("url")
                .action(ArgAction::Append) // Accept multiple values
                .required(true),
        )
        .get_matches();

    Args {
        overwrite: matches.get_flag("overwrite"),
        quiet: matches.get_flag("quiet"),
        verbose: matches.get_flag("verbose"),
        fonts_prefix_in_css: matches
            .get_one::<String>("fonts-prefix")
            .unwrap()
            .trim_end_matches('/') // Remove trailing slash
            .to_string(),
        output_dir: matches.get_one::<PathBuf>("output").unwrap().clone(),
        urls: matches
            .get_many::<String>("url")
            .unwrap_or_default()
            .map(std::string::ToString::to_string)
            .collect(),
    }
}

fn ensure_output_dir(output_dir: &PathBuf) -> std::io::Result<()> {
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }
    Ok(())
}

fn extract_font_info(css_content: &str) -> Result<Vec<FontInfo>, Box<dyn std::error::Error>> {
    let font_pattern = Regex::new(
        r"@font-face \{\n  font-family: '(?<font_family>\w+)';\n  font-style: (?<font_style>\w+);\n  font-weight: (?<font_weight>\w+);\n  font-stretch: (?<font_stretch>\w+);\n  font-display: (?<font_display>\w+);\n  src: url\((?<font_url>.+)\) format\('(?<font_format>\w+)'\);\n\}",
    )?;

    let mut fonts = Vec::new();
    for cap in font_pattern.captures_iter(css_content) {
        fonts.push(FontInfo {
            family: cap.name("font_family").unwrap().as_str().to_string(),
            style: cap.name("font_style").unwrap().as_str().to_string(),
            weight: cap.name("font_weight").unwrap().as_str().to_string(),
            stretch: cap.name("font_stretch").unwrap().as_str().to_string(),
            display: cap.name("font_display").unwrap().as_str().to_string(),
            url: cap.name("font_url").unwrap().as_str().to_string(),
            format: cap.name("font_format").unwrap().as_str().to_string(),
        });
    }

    Ok(fonts)
}

fn download_fonts(
    url: &str,
    output_dir: &Path,
    overwrite: bool,
    quiet: bool,
    verbose: bool,
    fonts_prefix_in_css: &str,
    client: &reqwest::blocking::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    if !quiet {
        println!("Downloading CSS: '{url}'.");
    }
    let response = client.get(url).send()?;
    // Parse the response
    let response_bytes = response.bytes()?;
    let css_content = str::from_utf8(&response_bytes)?;
    let fonts = extract_font_info(css_content)?;

    // Download each font
    for font in fonts {
        if !quiet && verbose {
            println!("Found font:");
            println!("  Family: '{}'", font.family);
            println!("  Style: '{}'", font.style);
            println!("  Weight: '{}'", font.weight);
            println!("  Stretch: '{}'", font.stretch);
            println!("  Display: '{}'", font.display);
            println!("  Format: '{}'", font.format);
            println!("  URL: '{}'", font.url);
        }

        if !quiet {
            println!("Downloading font: '{}'.", font.url);
        }
        let font_response = client.get(&font.url).send()?;
        let font_bytes = font_response.bytes()?;

        // Get filename and check format
        let Some(font_filename) = font.get_font_filename() else {
            return Err(format!("Unsupported font format: '{}'", font.format).into());
        };

        // Write font file
        let font_output_path = output_dir.join(&font_filename);
        if font_output_path.exists() && !overwrite {
            if !quiet {
                println!(
                    "Skipped writing to '{}' (file already exists, use --overwrite to overwrite).",
                    font_output_path.display()
                );
            }
        } else {
            // Write the font file
            if let Err(e) = fs::write(&font_output_path, font_bytes) {
                return Err(format!("Error writing font file '{font_filename}': {e}").into());
            } else if !quiet {
                println!("Wrote font file to '{font_filename}'.");
            }
        }

        // Write the CSS file
        let css_filename = font.get_css_filename();
        let css_output_path = output_dir.join(&css_filename);

        if css_output_path.exists() && !overwrite {
            if !quiet {
                println!(
                    "Skipped writing to '{}' (file already exists, use --overwrite to overwrite).",
                    css_output_path.display()
                );
            }
        } else {
            let css_content = font.generate_css(&format!("{fonts_prefix_in_css}/{font_filename}"));

            // Write the CSS file
            if let Err(e) = fs::write(&css_output_path, css_content) {
                return Err(format!("Error writing CSS file {css_filename}: {e}").into());
            } else if !quiet {
                println!("Wrote CSS file to '{css_filename}'.");
            }
        }
    }

    Ok(())
}

fn main() {
    let args = parse_args();

    // Create the output directory if it doesn't exist
    if let Err(e) = ensure_output_dir(&args.output_dir) {
        eprintln!("Failed to create output directory: '{e}'.");
        std::process::exit(1);
    }

    // Create a reusable HTTP client
    let client = reqwest::blocking::Client::new();

    // Download fonts from each URL
    if let Err(e) = args.urls.iter().try_for_each(|url| {
        download_fonts(
            url,
            &args.output_dir,
            args.overwrite,
            args.quiet,
            args.verbose,
            &args.fonts_prefix_in_css,
            &client,
        )
    }) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
