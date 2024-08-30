pub fn tokenize_watch_info(s: &str) -> Box<[Box<str>]> {
    use unicode_normalization::UnicodeNormalization;

    let normalized = s
        .to_lowercase()
        .nfd()
        .filter(|c| c.is_ascii())
        // .map(|c| match c {
        //     '-' | '.' => ' ',
        //     _ => c,
        // })
        .collect::<String>();

    let words: Vec<_> = normalized
        .split_whitespace()
        .filter(|s| !s.is_empty() && *s != "&")
        .map(|mut s| {
            if s.len() > 2 {
                if s.starts_with("fsot") {
                    s = &s[4..];
                } else if s.starts_with("fsot:") {
                    s = &s[5..];
                }

                if s.starts_with('(') || s.starts_with('-') {
                    s = &s[1..];
                }

                while s.len() > 2 && s.ends_with(')')
                    || s.ends_with('-')
                    || s.ends_with(',')
                    || s.ends_with('.')
                {
                    s = &s[..s.len() - 2];
                }
            }

            s.to_owned().into_boxed_str()
        })
        .collect();

    words.into_boxed_slice()
}
