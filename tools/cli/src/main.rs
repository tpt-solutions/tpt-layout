//! `tpt-layout-cli`: lays out a JSON box tree (see the `tpt-layout-spec` crate for the format).
//!
//! ```text
//! tpt-layout-cli svg    <input.json | -> [-o <output.svg>]   # draw as SVG (stdout by default)
//! tpt-layout-cli layout <input.json | ->                     # print every box's geometry as JSON
//! ```
//!
//! Use `-` to read the document from standard input.

use std::io::{self, Read, Write};
use std::process::ExitCode;

const USAGE: &str = "usage:
  tpt-layout-cli svg    <input.json | -> [-o <output.svg>]
  tpt-layout-cli layout <input.json | ->";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("tpt-layout-cli: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let (command, rest) = args.split_first().ok_or(USAGE)?;
    match command.as_str() {
        "svg" => {
            let (input, output) = parse_svg_args(rest)?;
            let json = read_input(input)?;
            let svg = tpt_layout_spec::render_svg(&json)?;
            write_output(output, svg.as_bytes())
        }
        "layout" => {
            let [input] = rest else {
                return Err(USAGE.into());
            };
            let json = read_input(input)?;
            let out = tpt_layout_spec::layout_json(&json)?;
            write_output(None, out.as_bytes())
        }
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command \"{other}\"\n{USAGE}")),
    }
}

/// Splits `<input> [-o <output>]`. The output path is optional.
fn parse_svg_args(rest: &[String]) -> Result<(&str, Option<&str>), String> {
    match rest {
        [input] => Ok((input, None)),
        [input, flag, output] if flag == "-o" => Ok((input, Some(output))),
        _ => Err(USAGE.into()),
    }
}

fn read_input(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        Ok(text)
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
    }
}

fn write_output(path: Option<&str>, bytes: &[u8]) -> Result<(), String> {
    match path {
        Some(path) => std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}")),
        None => io::stdout()
            .write_all(bytes)
            .and_then(|_| io::stdout().write_all(b"\n"))
            .map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_arguments_take_an_optional_output() {
        let args: Vec<String> = ["in.json"].iter().map(|s| s.to_string()).collect();
        assert_eq!(parse_svg_args(&args).unwrap(), ("in.json", None));
        let args: Vec<String> = ["in.json", "-o", "out.svg"].iter().map(|s| s.to_string()).collect();
        assert_eq!(parse_svg_args(&args).unwrap(), ("in.json", Some("out.svg")));
    }

    #[test]
    fn unknown_commands_are_errors() {
        let args = vec!["florp".to_string()];
        assert!(run(&args).is_err());
    }
}
