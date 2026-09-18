#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsiveLayout {
    Compact,
    Standard,
    Wide,
}

impl ResponsiveLayout {
    pub fn from_width(width: f32) -> Self {
        if width >= 1180.0 {
            Self::Wide
        } else if width >= 820.0 {
            Self::Standard
        } else {
            Self::Compact
        }
    }

    pub const fn uses_device_column(self) -> bool {
        !matches!(self, Self::Compact)
    }

    pub const fn uses_output_column(self) -> bool {
        matches!(self, Self::Wide)
    }
}
