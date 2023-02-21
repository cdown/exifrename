use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    fmt: String,

    #[arg(value_parser)]
    files: Vec<PathBuf>,
}

fn parse_formats(fmt: &str) -> String {
    let mut out = vec![];
    let mut chars = fmt.chars().peekable();

    while let Some(cur) = chars.next() {
        if cur != '%' {
            out.push(cur.to_string());
            continue;
        }

        if let Some(next) = chars.peek() {
            match next {
                '%' => out.push("%".to_string()),
                _ => {
                    eprintln!("ignored unknown format %{}", next);
                    out.push("%".to_string())
                }
            };
        }
    }

    out.join("")
}

fn main() {
    let args = Args::parse();
    for file in args.files {
        println!("{}", file.display());
        println!("{}", parse_formats(&args.fmt));
    }
}
