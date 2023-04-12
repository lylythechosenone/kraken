use core::fmt::{self, Display, Formatter};

use log::info;

pub struct Size(pub usize);
impl Display for Size {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            write!(f, "0B")
        } else if self.0 % 1024usize.pow(6) == 0 {
            write!(f, "{}EiB", self.0 / 1024usize.pow(6))
        } else if self.0 % 1024usize.pow(5) == 0 {
            write!(f, "{}PiB", self.0 / 1024usize.pow(5))
        } else if self.0 % 1024usize.pow(4) == 0 {
            write!(f, "{}TiB", self.0 / 1024usize.pow(4))
        } else if self.0 % 1024usize.pow(3) == 0 {
            write!(f, "{}GiB", self.0 / 1024usize.pow(3))
        } else if self.0 % 1024usize.pow(2) == 0 {
            write!(f, "{}MiB", self.0 / 1024usize.pow(2))
        } else if self.0 % 1024 == 0 {
            write!(f, "{}KiB", self.0 / 1024)
        } else {
            write!(f, "{}B", self.0)
        }
    }
}
