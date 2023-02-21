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
    println!("{}", parse_formats("abcde%a%a%%%a"));
}
