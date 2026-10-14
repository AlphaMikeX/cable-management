use serde::{Deserialize, Serialize};

/// Nominal socket opening sizes in millimetres; metadata holds protocol versions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortKind {
    UsbA,
    UsbB,
    UsbC,
    MiniUsb,
    MicroUsb,
    Hdmi,
    DisplayPort,
    MiniDisplayPort,
    Dvi,
    Vga,
    Rj45,
    Jack35,
    Jack635,
    Xlr,
    Rca,
    Toslink,
    DcBarrel,
    IecC14,
    IecC8,
    Sata,
}

impl PortKind {
    pub const ALL: [Self; 20] = [
        Self::UsbA,
        Self::UsbB,
        Self::UsbC,
        Self::MiniUsb,
        Self::MicroUsb,
        Self::Hdmi,
        Self::DisplayPort,
        Self::MiniDisplayPort,
        Self::Dvi,
        Self::Vga,
        Self::Rj45,
        Self::Jack35,
        Self::Jack635,
        Self::Xlr,
        Self::Rca,
        Self::Toslink,
        Self::DcBarrel,
        Self::IecC14,
        Self::IecC8,
        Self::Sata,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::UsbA => "USB-A",
            Self::UsbB => "USB-B",
            Self::UsbC => "USB-C",
            Self::MiniUsb => "Mini-USB",
            Self::MicroUsb => "Micro-USB",
            Self::Hdmi => "HDMI",
            Self::DisplayPort => "DisplayPort",
            Self::MiniDisplayPort => "Mini-DisplayPort",
            Self::Dvi => "DVI",
            Self::Vga => "VGA",
            Self::Rj45 => "RJ45 / Ethernet",
            Self::Jack35 => "3,5 mm tele",
            Self::Jack635 => "6,35 mm tele",
            Self::Xlr => "XLR",
            Self::Rca => "RCA",
            Self::Toslink => "TOSLINK",
            Self::DcBarrel => "DC rund",
            Self::IecC14 => "IEC C14",
            Self::IecC8 => "IEC C8",
            Self::Sata => "SATA",
        }
    }

    pub fn dimensions(self) -> [f32; 2] {
        match self {
            Self::UsbA => [12., 4.5],
            Self::UsbB => [8.5, 7.5],
            Self::UsbC => [8.4, 2.6],
            Self::MiniUsb => [7.5, 3.9],
            Self::MicroUsb => [6.9, 1.8],
            Self::Hdmi => [14., 4.5],
            Self::DisplayPort => [16.1, 4.8],
            Self::MiniDisplayPort => [7.5, 4.6],
            Self::Dvi => [25., 8.],
            Self::Vga => [17., 8.],
            Self::Rj45 => [12., 10.],
            Self::Jack35 => [3.5, 3.5],
            Self::Jack635 => [6.35, 6.35],
            Self::Xlr => [24., 24.],
            Self::Rca => [8., 8.],
            Self::Toslink => [9., 9.],
            Self::DcBarrel => [5.5, 5.5],
            Self::IecC14 => [28., 20.],
            Self::IecC8 => [16., 8.],
            Self::Sata => [14., 4.],
        }
    }

    pub fn family(self) -> u8 {
        match self {
            Self::UsbA | Self::UsbB | Self::UsbC | Self::MiniUsb | Self::MicroUsb => 1,
            Self::Jack35 | Self::Jack635 | Self::Xlr | Self::Rca => 2,
            Self::DcBarrel | Self::IecC14 | Self::IecC8 => 3,
            Self::Hdmi => 4,
            Self::DisplayPort | Self::MiniDisplayPort => 5,
            Self::Dvi => 6,
            Self::Vga => 7,
            Self::Rj45 => 8,
            Self::Toslink => 9,
            Self::Sata => 10,
        }
    }

    pub fn compatible(self, other: Self) -> bool {
        self.family() == other.family()
    }

    pub fn round(self) -> bool {
        matches!(
            self,
            Self::Jack35 | Self::Jack635 | Self::Xlr | Self::Rca | Self::DcBarrel
        )
    }

    /// Convex outlines in normalized socket-local coordinates.
    pub fn outline(self) -> Vec<[f32; 2]> {
        if self.round() {
            return (0..24)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 24.;
                    [a.cos() * 0.5, a.sin() * 0.5]
                })
                .collect();
        }
        match self {
            Self::Hdmi | Self::MiniUsb | Self::MicroUsb | Self::Vga | Self::Dvi => vec![
                [-0.5, -0.2],
                [-0.32, -0.5],
                [0.32, -0.5],
                [0.5, -0.2],
                [0.5, 0.5],
                [-0.5, 0.5],
            ],
            Self::DisplayPort => vec![
                [-0.5, -0.5],
                [0.5, -0.5],
                [0.5, 0.22],
                [0.23, 0.5],
                [-0.5, 0.5],
            ],
            Self::UsbB | Self::IecC14 => vec![
                [-0.5, -0.5],
                [0.5, -0.5],
                [0.5, 0.2],
                [0.2, 0.5],
                [-0.2, 0.5],
                [-0.5, 0.2],
            ],
            Self::UsbC | Self::IecC8 => (0..20)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 20.;
                    [
                        a.cos() * 0.15 + if a.cos() >= 0. { 0.35 } else { -0.35 },
                        a.sin() * 0.5,
                    ]
                })
                .collect(),
            _ => vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]],
        }
    }
}
