use super::color::RGBAColor;

/// Named palette types.
#[derive(Clone, Debug)]
pub enum PaletteName {
    // Qualitative
    Set1,
    Set2,
    Dark2,
    Paired,
    // Sequential
    Blues,
    Greens,
    Reds,
    // Diverging
    RdBu,
    Spectral,
    // Perceptual
    Viridis,
}

/// Get colors for a named palette.
pub fn palette(name: &PaletteName) -> &'static [RGBAColor] {
    match name {
        PaletteName::Set1 => &SET1,
        PaletteName::Set2 => &SET2,
        PaletteName::Dark2 => &DARK2,
        PaletteName::Paired => &PAIRED,
        PaletteName::Blues => &BLUES,
        PaletteName::Greens => &GREENS,
        PaletteName::Reds => &REDS,
        PaletteName::RdBu => &RDBU,
        PaletteName::Spectral => &SPECTRAL,
        PaletteName::Viridis => &VIRIDIS,
    }
}

const fn c(r: u8, g: u8, b: u8) -> RGBAColor {
    RGBAColor { r, g, b, a: 1.0 }
}

// Brewer qualitative: Set1 (9 colors)
static SET1: [RGBAColor; 9] = [
    c(228, 26, 28), c(55, 126, 184), c(77, 175, 74), c(152, 78, 163),
    c(255, 127, 0), c(255, 255, 51), c(166, 86, 40), c(247, 129, 191),
    c(153, 153, 153),
];

// Brewer qualitative: Set2 (8 colors)
static SET2: [RGBAColor; 8] = [
    c(102, 194, 165), c(252, 141, 98), c(141, 160, 203), c(231, 138, 195),
    c(166, 216, 84), c(255, 217, 47), c(229, 196, 148), c(179, 179, 179),
];

// Brewer qualitative: Dark2 (8 colors)
static DARK2: [RGBAColor; 8] = [
    c(27, 158, 119), c(217, 95, 2), c(117, 112, 179), c(231, 41, 138),
    c(102, 166, 30), c(230, 171, 2), c(166, 118, 29), c(102, 102, 102),
];

// Brewer qualitative: Paired (12 colors)
static PAIRED: [RGBAColor; 12] = [
    c(166, 206, 227), c(31, 120, 180), c(178, 223, 138), c(51, 160, 44),
    c(251, 154, 153), c(227, 26, 28), c(253, 191, 111), c(255, 127, 0),
    c(202, 178, 214), c(106, 61, 154), c(255, 255, 153), c(177, 89, 40),
];

// Brewer sequential: Blues (9 stops)
static BLUES: [RGBAColor; 9] = [
    c(247, 251, 255), c(222, 235, 247), c(198, 219, 239), c(158, 202, 225),
    c(107, 174, 214), c(66, 146, 198), c(33, 113, 181), c(8, 81, 156),
    c(8, 48, 107),
];

// Brewer sequential: Greens (9 stops)
static GREENS: [RGBAColor; 9] = [
    c(247, 252, 245), c(229, 245, 224), c(199, 233, 192), c(161, 217, 155),
    c(116, 196, 118), c(65, 171, 93), c(35, 139, 69), c(0, 109, 44),
    c(0, 68, 27),
];

// Brewer sequential: Reds (9 stops)
static REDS: [RGBAColor; 9] = [
    c(255, 245, 240), c(254, 224, 210), c(252, 187, 161), c(252, 146, 114),
    c(251, 106, 74), c(239, 59, 44), c(203, 24, 29), c(165, 15, 21),
    c(103, 0, 13),
];

// Brewer diverging: RdBu (11 stops)
static RDBU: [RGBAColor; 11] = [
    c(103, 0, 31), c(178, 24, 43), c(214, 96, 77), c(244, 165, 130),
    c(253, 219, 199), c(247, 247, 247), c(209, 229, 240), c(146, 197, 222),
    c(67, 147, 195), c(33, 102, 172), c(5, 48, 97),
];

// Brewer diverging: Spectral (11 stops)
static SPECTRAL: [RGBAColor; 11] = [
    c(158, 1, 66), c(213, 62, 79), c(244, 109, 67), c(253, 174, 97),
    c(254, 224, 139), c(255, 255, 191), c(230, 245, 152), c(171, 221, 164),
    c(102, 194, 165), c(50, 136, 189), c(94, 79, 162),
];

// Viridis (16 representative stops)
static VIRIDIS: [RGBAColor; 16] = [
    c(68, 1, 84), c(72, 26, 108), c(71, 47, 126), c(65, 68, 135),
    c(57, 86, 140), c(47, 104, 142), c(38, 121, 142), c(31, 138, 141),
    c(30, 155, 138), c(42, 172, 130), c(70, 188, 115), c(109, 202, 93),
    c(155, 213, 67), c(200, 222, 39), c(240, 229, 30), c(253, 231, 37),
];
