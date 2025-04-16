use clap::{command, value_parser, Arg, ArgAction};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::str;

#[derive(Debug, Clone, PartialEq)]
enum FontFormat {
    TrueType,
    Woff,
    Woff2,
    Unknown,
}

impl FontFormat {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "truetype" => FontFormat::TrueType,
            "woff" => FontFormat::Woff,
            "woff2" => FontFormat::Woff2,
            _ => FontFormat::Unknown,
        }
    }

    fn to_extension(&self) -> String {
        match self {
            FontFormat::TrueType => "ttf".to_string(),
            FontFormat::Woff => "woff".to_string(),
            FontFormat::Woff2 => "woff2".to_string(),
            FontFormat::Unknown => String::new(),
        }
    }
}

struct Args {
    urls: Vec<String>,
    output_dir: PathBuf,
    overwrite: bool,
    quiet: bool,
    verbose: bool,
    fonts_prefix_in_css: String,
}

struct FontInfo {
    css: String,
    writing_system_name: String,
}

fn split_css_into_fonts(css: &str) -> Vec<FontInfo> {
    let mut font_infos = Vec::new();
    let mut pos = 0;

    while pos < css.len() {
        // Find start of comment
        if let Some(comment_start) = css[pos..].find("/*") {
            let comment_start = pos + comment_start;

            // Find end of comment
            if let Some(comment_end) = css[comment_start..].find("*/") {
                let comment_end = comment_start + comment_end + 2; // +2 for "*/"
                                                                   // Extract writing system name without the comment markers
                let writing_system_name =
                    css[comment_start + 2..comment_end - 2].trim().to_string();

                // Find next comment start or end of string
                let next_comment_start = css[comment_end..]
                    .find("/*")
                    .map_or(css.len(), |i| comment_end + i);

                // Extract CSS content between comments
                let css_content = css[comment_end..next_comment_start].trim().to_string();

                if !css_content.is_empty() {
                    font_infos.push(FontInfo {
                        css: css_content,
                        writing_system_name,
                    });
                }

                pos = next_comment_start;

                // If we've reached the end of the string, break
                if pos >= css.len() {
                    break;
                }
            } else {
                break;
            }
        } else {
            // No more comments found
            if pos == 0 {
                font_infos.push(FontInfo {
                    css: css.to_string(),
                    writing_system_name: String::new(),
                });
            } else {
                // Get the remaining CSS after the last comment
                let remaining_css = css[pos..].trim().to_string();
                if !remaining_css.is_empty() {
                    // Use the last writing system name if we have one, or empty string if not
                    let writing_system_name = font_infos
                        .last()
                        .map_or_else(String::new, |info| info.writing_system_name.clone());

                    font_infos.push(FontInfo {
                        css: remaining_css,
                        writing_system_name,
                    });
                }
            }
            break;
        }
    }

    font_infos
}

impl FontInfo {
    fn get_font_family(&self) -> String {
        self.css
            .split("font-family: '")
            .nth(1)
            .unwrap()
            .split("';")
            .next()
            .unwrap()
            .to_string()
    }

    fn get_font_style(&self) -> String {
        self.css
            .split("font-style: ")
            .nth(1)
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    fn get_font_weight(&self) -> String {
        self.css
            .split("font-weight: ")
            .nth(1)
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    fn get_font_stretch(&self) -> Option<String> {
        // Check if font-stretch property exists in the CSS
        if self.css.contains("font-stretch:") {
            // Extract the font-stretch value
            Some(
                self.css
                    .split("font-stretch: ")
                    .nth(1)
                    .unwrap()
                    .split(';')
                    .next()
                    .unwrap()
                    .to_string(),
            )
        } else {
            // Return None if font-stretch property is not present
            None
        }
    }

    fn get_font_display(&self) -> String {
        self.css
            .split("font-display: ")
            .nth(1)
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    fn get_font_url_and_format(&self) -> (String, FontFormat) {
        // Extract the URL and format from the CSS source property

        let src_part = self
            .css
            .split("src: ")
            .nth(1)
            .unwrap()
            .split(';')
            .next()
            .unwrap();

        // Extract the URL from the url() part
        let url_start = src_part.find("url(").unwrap() + 4;
        let url_end = src_part.find(')').unwrap();
        let url = src_part[url_start..url_end].trim_matches('"').to_string();

        // Extract the format from the format() part
        let format_start = src_part.find("format('").unwrap() + 8;
        let format_end = src_part.find("')").unwrap();
        let format_str = src_part[format_start..format_end].to_string();
        let format = FontFormat::from_str(&format_str);

        (url, format)
    }

    fn get_font_url(&self) -> String {
        self.get_font_url_and_format().0
    }

    fn get_font_format(&self) -> FontFormat {
        self.get_font_url_and_format().1
    }

    fn get_font_filename(&self) -> String {
        format!(
            "{}-{}-{}-{}.{}",
            self.get_font_family().to_lowercase().replace(' ', "-"),
            self.get_font_weight(),
            self.get_font_style(),
            self.writing_system_name,
            self.get_font_format().to_extension()
        )
    }

    fn get_css_filename(&self) -> String {
        format!(
            "{}-{}-{}-{}.css",
            self.get_font_family().to_lowercase().replace(' ', "-"),
            self.get_font_weight(),
            self.get_font_style(),
            self.writing_system_name
        )
    }

    fn get_new_css(&self, font_prefix: &str) -> String {
        let original_url = self.get_font_url();
        let font_filename = self.get_font_filename();
        let new_url = format!("{font_prefix}/{font_filename}");
        self.css.replace(&original_url, &new_url)
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
    // Google Fonts serves different CSS content based on the User-Agent.
    // Without a browser-like User-Agent, it returns a simplified version without writing system comments.
    // Setting a browser User-Agent ensures we get the full CSS with all writing system information.
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()?;
    // Parse the response
    let response_bytes = response.bytes()?;
    let css_content = str::from_utf8(&response_bytes)?;

    if verbose {
        println!("Downloaded CSS content ({} bytes)", css_content.len());
    }

    let fonts = split_css_into_fonts(css_content);

    if verbose {
        println!("Found {} font entries in the CSS", fonts.len());
    }

    // Download each font
    for font in fonts {
        if !quiet {
            println!("Downloading font file: '{}'.", font.get_font_url());
        }

        if verbose {
            println!("  Font family: {}", font.get_font_family());
            println!("  Font style: {}", font.get_font_style());
            println!("  Font weight: {}", font.get_font_weight());
            if let Some(stretch) = font.get_font_stretch() {
                println!("  Font stretch: {stretch}");
            }
            println!("  Font display: {}", font.get_font_display());
            println!("  Writing system: {}", font.writing_system_name);
            println!("  Format: {:?}", font.get_font_format());
            println!("  Extension: {}", font.get_font_format().to_extension());
        }

        let font_file_response = client.get(font.get_font_url()).send()?;
        let font_file_bytes = font_file_response.bytes()?;

        if verbose {
            println!("  Downloaded font file ({} bytes)", font_file_bytes.len());
        }

        // Write font file
        let font_output_path = output_dir.join(font.get_font_filename());
        if font_output_path.exists() && !overwrite {
            if !quiet {
                println!(
                    "Skipped writing to '{}' (file already exists, use --overwrite to overwrite).",
                    font_output_path.display()
                );
            }
        } else {
            // Write the font file
            if let Err(e) = fs::write(&font_output_path, font_file_bytes) {
                return Err(format!(
                    "Error writing font file '{}': {}",
                    &font.get_font_filename(),
                    e
                )
                .into());
            } else if !quiet {
                println!("Wrote font file to '{}'.", &font.get_font_filename());
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
            let css_content = font.get_new_css(fonts_prefix_in_css);

            if verbose {
                println!("  Writing CSS file with updated font path: {css_filename}");
            }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_css_into_fonts() {
        let css = r"/* latin */
@font-face {
  font-family: 'Creepster';
  font-style: normal;
  font-weight: 400;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/creepster/v13/AlZy_zVUqJz4yMrniH4Rcn35fh4Dog.woff2) format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}
/* latin */
@font-face {
  font-family: 'Gravitas One';
  font-style: normal;
  font-weight: 400;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/gravitasone/v19/5h1diZ4hJ3cblKy3LWakKQmqCm5MjXPjbA.woff2) format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}";

        let result = split_css_into_fonts(css);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].writing_system_name, "latin");
        assert_eq!(result[0].css, "@font-face {\n  font-family: 'Creepster';\n  font-style: normal;\n  font-weight: 400;\n  font-display: swap;\n  src: url(https://fonts.gstatic.com/s/creepster/v13/AlZy_zVUqJz4yMrniH4Rcn35fh4Dog.woff2) format('woff2');\n  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;\n}");
        assert_eq!(result[1].writing_system_name, "latin");
        assert_eq!(result[1].css, "@font-face {\n  font-family: 'Gravitas One';\n  font-style: normal;\n  font-weight: 400;\n  font-display: swap;\n  src: url(https://fonts.gstatic.com/s/gravitasone/v19/5h1diZ4hJ3cblKy3LWakKQmqCm5MjXPjbA.woff2) format('woff2');\n  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;\n}");
    }
}
