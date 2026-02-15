use anyhow::Error as AnyhowError;
use std::fmt;

#[derive(Debug)]
pub enum HdrError {
    Image(String),
    Alignment(String),
    Merge(String),
    Tonemap(String),
    Exr(String),
    Exif(String),
    Io(String),
    InvalidInput(String),
}

impl fmt::Display for HdrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HdrError::Image(msg) => write!(f, "Image error: {}", msg),
            HdrError::Alignment(msg) => write!(f, "Alignment error: {}", msg),
            HdrError::Merge(msg) => write!(f, "HDR merge error: {}", msg),
            HdrError::Tonemap(msg) => write!(f, "Tonemap error: {}", msg),
            HdrError::Exr(msg) => write!(f, "EXR error: {}", msg),
            HdrError::Exif(msg) => write!(f, "EXIF error: {}", msg),
            HdrError::Io(msg) => write!(f, "I/O error: {}", msg),
            HdrError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for HdrError {}

impl From<image::ImageError> for HdrError {
    fn from(err: image::ImageError) -> Self {
        HdrError::Image(err.to_string())
    }
}

impl From<std::io::Error> for HdrError {
    fn from(err: std::io::Error) -> Self {
        HdrError::Io(err.to_string())
    }
}

impl From<exif::Error> for HdrError {
    fn from(err: exif::Error) -> Self {
        HdrError::Exif(err.to_string())
    }
}

impl From<AnyhowError> for HdrError {
    fn from(err: AnyhowError) -> Self {
        HdrError::Image(err.to_string())
    }
}
