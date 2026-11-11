use std::num::NonZeroIsize;

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Fullscreen,
    Configure,
    Preview(NonZeroIsize),
}

pub fn parse(args: &[String]) -> Result<Mode, &'static str> {
    if args.is_empty() {
        return Ok(Mode::Configure);
    }
    let first = args[0].to_ascii_lowercase();
    let (switch, inline) = first
        .split_once(':')
        .map_or((first.as_str(), None), |(a, b)| (a, Some(b)));
    match switch {
        "/s" if args.len() == 1 && inline.is_none() => Ok(Mode::Fullscreen),
        "/c" if args.len() <= 2 => Ok(Mode::Configure),
        "/p" if args.len() <= 2 => {
            let value = inline
                .or_else(|| args.get(1).map(String::as_str))
                .ok_or("preview requires a window handle")?;
            if inline.is_some() && args.len() != 1 {
                return Err("unexpected preview argument");
            }
            let handle = value
                .parse::<isize>()
                .ok()
                .and_then(NonZeroIsize::new)
                .ok_or("invalid preview window handle")?;
            Ok(Mode::Preview(handle))
        }
        _ => Err("expected /s, /c, or /p <HWND>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_windows_modes_and_handles() {
        let args = |values: &[&str]| values.iter().map(|v| v.to_string()).collect::<Vec<_>>();
        assert_eq!(parse(&[]), Ok(Mode::Configure));
        assert_eq!(parse(&args(&["/S"])), Ok(Mode::Fullscreen));
        assert_eq!(parse(&args(&["/c:123"])), Ok(Mode::Configure));
        assert_eq!(
            parse(&args(&["/p", "123"])),
            Ok(Mode::Preview(NonZeroIsize::new(123).unwrap()))
        );
        assert_eq!(
            parse(&args(&["/P:123"])),
            Ok(Mode::Preview(NonZeroIsize::new(123).unwrap()))
        );
        for input in [
            &["/p"][..],
            &["/p", "0"],
            &["/p", "abc"],
            &["/s", "extra"],
            &["/p:1", "2"],
            &["/x"],
        ] {
            assert!(parse(&args(input)).is_err());
        }
    }
}
