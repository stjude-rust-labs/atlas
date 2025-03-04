use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use flate2::read::MultiGzDecoder;

pub fn open<P>(src: P) -> io::Result<Box<dyn Read>>
where
    P: AsRef<Path>,
{
    let file = File::open(src.as_ref())?;

    if is_gzip(src) {
        Ok(Box::new(MultiGzDecoder::new(file)))
    } else {
        Ok(Box::new(file))
    }
}

fn is_gzip<P>(src: P) -> bool
where
    P: AsRef<Path>,
{
    src.as_ref()
        .extension()
        .map(|ext| ext == "gz")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_gzip() {
        assert!(is_gzip("in.txt.gz"));
        assert!(!is_gzip("in.txt"));
    }
}
