use super::color::RGBAColor;

/// Named palette types.
#[derive(Clone, Debug)]
pub enum PaletteName {
    // Qualitative
    Set1,
    Set2,
    Set3,
    Dark2,
    Paired,
    Pastel1,
    Pastel2,
    Accent,
    // Sequential
    Blues,
    Greens,
    Reds,
    Oranges,
    Purples,
    Greys,
    YlOrRd,
    YlGnBu,
    BuGn,
    BuPu,
    GnBu,
    OrRd,
    PuRd,
    RdPu,
    YlGn,
    YlOrBr,
    // Diverging
    RdBu,
    Spectral,
    PiYG,
    PRGn,
    BrBG,
    PuOr,
    RdGy,
    RdYlBu,
    RdYlGn,
    // Perceptual
    Viridis,
    Magma,
    Plasma,
    Inferno,
    // ggsci journal palettes (as used by ggpubr)
    Npg,
    Aaas,
    Nejm,
    Lancet,
    Jama,
    Jco,
    D3,
}

/// Get colors for a named palette.
pub fn palette(name: &PaletteName) -> &'static [RGBAColor] {
    match name {
        PaletteName::Set1 => &SET1,
        PaletteName::Set2 => &SET2,
        PaletteName::Set3 => &SET3,
        PaletteName::Dark2 => &DARK2,
        PaletteName::Paired => &PAIRED,
        PaletteName::Pastel1 => &PASTEL1,
        PaletteName::Pastel2 => &PASTEL2,
        PaletteName::Accent => &ACCENT,
        PaletteName::Blues => &BLUES,
        PaletteName::Greens => &GREENS,
        PaletteName::Reds => &REDS,
        PaletteName::Oranges => &ORANGES,
        PaletteName::Purples => &PURPLES,
        PaletteName::Greys => &GREYS,
        PaletteName::YlOrRd => &YLORRD,
        PaletteName::YlGnBu => &YLGNBU,
        PaletteName::BuGn => &BUGN,
        PaletteName::BuPu => &BUPU,
        PaletteName::GnBu => &GNBU,
        PaletteName::OrRd => &ORRD,
        PaletteName::PuRd => &PURD,
        PaletteName::RdPu => &RDPU,
        PaletteName::YlGn => &YLGN,
        PaletteName::YlOrBr => &YLORBR,
        PaletteName::RdBu => &RDBU,
        PaletteName::Spectral => &SPECTRAL,
        PaletteName::PiYG => &PIYG,
        PaletteName::PRGn => &PRGN,
        PaletteName::BrBG => &BRBG,
        PaletteName::PuOr => &PUOR,
        PaletteName::RdGy => &RDGY,
        PaletteName::RdYlBu => &RDYLBU,
        PaletteName::RdYlGn => &RDYLGN,
        PaletteName::Viridis => &VIRIDIS,
        PaletteName::Magma => &MAGMA,
        PaletteName::Plasma => &PLASMA,
        PaletteName::Inferno => &INFERNO,
        PaletteName::Npg => &NPG,
        PaletteName::Aaas => &AAAS,
        PaletteName::Nejm => &NEJM,
        PaletteName::Lancet => &LANCET,
        PaletteName::Jama => &JAMA,
        PaletteName::Jco => &JCO,
        PaletteName::D3 => &D3,
    }
}

const fn c(r: u8, g: u8, b: u8) -> RGBAColor {
    RGBAColor { r, g, b, a: 1.0 }
}

// ─── Qualitative ────────────────────────────────────────────

static SET1: [RGBAColor; 9] = [
    c(228, 26, 28),
    c(55, 126, 184),
    c(77, 175, 74),
    c(152, 78, 163),
    c(255, 127, 0),
    c(255, 255, 51),
    c(166, 86, 40),
    c(247, 129, 191),
    c(153, 153, 153),
];

static SET2: [RGBAColor; 8] = [
    c(102, 194, 165),
    c(252, 141, 98),
    c(141, 160, 203),
    c(231, 138, 195),
    c(166, 216, 84),
    c(255, 217, 47),
    c(229, 196, 148),
    c(179, 179, 179),
];

static SET3: [RGBAColor; 12] = [
    c(141, 211, 199),
    c(255, 255, 179),
    c(190, 186, 218),
    c(251, 128, 114),
    c(128, 177, 211),
    c(253, 180, 98),
    c(179, 222, 105),
    c(252, 205, 229),
    c(217, 217, 217),
    c(188, 128, 189),
    c(204, 235, 197),
    c(255, 237, 111),
];

static DARK2: [RGBAColor; 8] = [
    c(27, 158, 119),
    c(217, 95, 2),
    c(117, 112, 179),
    c(231, 41, 138),
    c(102, 166, 30),
    c(230, 171, 2),
    c(166, 118, 29),
    c(102, 102, 102),
];

static PAIRED: [RGBAColor; 12] = [
    c(166, 206, 227),
    c(31, 120, 180),
    c(178, 223, 138),
    c(51, 160, 44),
    c(251, 154, 153),
    c(227, 26, 28),
    c(253, 191, 111),
    c(255, 127, 0),
    c(202, 178, 214),
    c(106, 61, 154),
    c(255, 255, 153),
    c(177, 89, 40),
];

static PASTEL1: [RGBAColor; 9] = [
    c(251, 180, 174),
    c(179, 205, 227),
    c(204, 235, 197),
    c(222, 203, 228),
    c(254, 217, 166),
    c(255, 255, 204),
    c(229, 216, 189),
    c(253, 218, 236),
    c(242, 242, 242),
];

static PASTEL2: [RGBAColor; 8] = [
    c(179, 226, 205),
    c(253, 205, 172),
    c(203, 213, 232),
    c(244, 202, 228),
    c(230, 245, 201),
    c(255, 242, 174),
    c(241, 226, 204),
    c(204, 204, 204),
];

static ACCENT: [RGBAColor; 8] = [
    c(127, 201, 127),
    c(190, 174, 212),
    c(253, 192, 134),
    c(255, 255, 153),
    c(56, 108, 176),
    c(240, 2, 127),
    c(191, 91, 23),
    c(102, 102, 102),
];

// ─── Sequential ─────────────────────────────────────────────

static BLUES: [RGBAColor; 9] = [
    c(247, 251, 255),
    c(222, 235, 247),
    c(198, 219, 239),
    c(158, 202, 225),
    c(107, 174, 214),
    c(66, 146, 198),
    c(33, 113, 181),
    c(8, 81, 156),
    c(8, 48, 107),
];

static GREENS: [RGBAColor; 9] = [
    c(247, 252, 245),
    c(229, 245, 224),
    c(199, 233, 192),
    c(161, 217, 155),
    c(116, 196, 118),
    c(65, 171, 93),
    c(35, 139, 69),
    c(0, 109, 44),
    c(0, 68, 27),
];

static REDS: [RGBAColor; 9] = [
    c(255, 245, 240),
    c(254, 224, 210),
    c(252, 187, 161),
    c(252, 146, 114),
    c(251, 106, 74),
    c(239, 59, 44),
    c(203, 24, 29),
    c(165, 15, 21),
    c(103, 0, 13),
];

static ORANGES: [RGBAColor; 9] = [
    c(255, 245, 235),
    c(254, 230, 206),
    c(253, 208, 162),
    c(253, 174, 107),
    c(253, 141, 60),
    c(241, 105, 19),
    c(217, 72, 1),
    c(166, 54, 3),
    c(127, 39, 4),
];

static PURPLES: [RGBAColor; 9] = [
    c(252, 251, 253),
    c(239, 237, 245),
    c(218, 218, 235),
    c(188, 189, 220),
    c(158, 154, 200),
    c(128, 125, 186),
    c(106, 81, 163),
    c(84, 39, 143),
    c(63, 0, 125),
];

static GREYS: [RGBAColor; 9] = [
    c(255, 255, 255),
    c(240, 240, 240),
    c(217, 217, 217),
    c(189, 189, 189),
    c(150, 150, 150),
    c(115, 115, 115),
    c(82, 82, 82),
    c(37, 37, 37),
    c(0, 0, 0),
];

static YLORRD: [RGBAColor; 9] = [
    c(255, 255, 204),
    c(255, 237, 160),
    c(254, 217, 118),
    c(254, 178, 76),
    c(253, 141, 60),
    c(252, 78, 42),
    c(227, 26, 28),
    c(189, 0, 38),
    c(128, 0, 38),
];

static YLGNBU: [RGBAColor; 9] = [
    c(255, 255, 217),
    c(237, 248, 177),
    c(199, 233, 180),
    c(127, 205, 187),
    c(65, 182, 196),
    c(29, 145, 192),
    c(34, 94, 168),
    c(37, 52, 148),
    c(8, 29, 88),
];

static BUGN: [RGBAColor; 9] = [
    c(247, 252, 253),
    c(229, 245, 249),
    c(204, 236, 230),
    c(153, 216, 201),
    c(102, 194, 164),
    c(65, 174, 118),
    c(35, 139, 69),
    c(0, 109, 44),
    c(0, 68, 27),
];

static BUPU: [RGBAColor; 9] = [
    c(247, 252, 253),
    c(224, 236, 244),
    c(191, 211, 230),
    c(158, 188, 218),
    c(140, 150, 198),
    c(140, 107, 177),
    c(136, 65, 157),
    c(129, 15, 124),
    c(77, 0, 75),
];

static GNBU: [RGBAColor; 9] = [
    c(247, 252, 240),
    c(224, 243, 219),
    c(204, 235, 197),
    c(168, 221, 181),
    c(123, 204, 196),
    c(78, 179, 211),
    c(43, 140, 190),
    c(8, 104, 172),
    c(8, 64, 129),
];

static ORRD: [RGBAColor; 9] = [
    c(255, 247, 236),
    c(254, 232, 200),
    c(253, 212, 158),
    c(253, 187, 132),
    c(252, 141, 89),
    c(239, 101, 72),
    c(215, 48, 31),
    c(179, 0, 0),
    c(127, 0, 0),
];

static PURD: [RGBAColor; 9] = [
    c(247, 244, 249),
    c(231, 225, 239),
    c(212, 185, 218),
    c(201, 148, 199),
    c(223, 101, 176),
    c(231, 41, 138),
    c(206, 18, 86),
    c(152, 0, 67),
    c(103, 0, 31),
];

static RDPU: [RGBAColor; 9] = [
    c(255, 247, 243),
    c(253, 224, 221),
    c(252, 197, 192),
    c(250, 159, 181),
    c(247, 104, 161),
    c(221, 52, 151),
    c(174, 1, 126),
    c(122, 1, 119),
    c(73, 0, 106),
];

static YLGN: [RGBAColor; 9] = [
    c(255, 255, 229),
    c(247, 252, 185),
    c(217, 240, 163),
    c(173, 221, 142),
    c(120, 198, 121),
    c(65, 171, 93),
    c(35, 132, 67),
    c(0, 104, 55),
    c(0, 69, 41),
];

static YLORBR: [RGBAColor; 9] = [
    c(255, 255, 229),
    c(255, 247, 188),
    c(254, 227, 145),
    c(254, 196, 79),
    c(254, 153, 41),
    c(236, 112, 20),
    c(204, 76, 2),
    c(153, 52, 4),
    c(102, 37, 6),
];

// ─── Diverging ──────────────────────────────────────────────

static RDBU: [RGBAColor; 11] = [
    c(103, 0, 31),
    c(178, 24, 43),
    c(214, 96, 77),
    c(244, 165, 130),
    c(253, 219, 199),
    c(247, 247, 247),
    c(209, 229, 240),
    c(146, 197, 222),
    c(67, 147, 195),
    c(33, 102, 172),
    c(5, 48, 97),
];

static SPECTRAL: [RGBAColor; 11] = [
    c(158, 1, 66),
    c(213, 62, 79),
    c(244, 109, 67),
    c(253, 174, 97),
    c(254, 224, 139),
    c(255, 255, 191),
    c(230, 245, 152),
    c(171, 221, 164),
    c(102, 194, 165),
    c(50, 136, 189),
    c(94, 79, 162),
];

static PIYG: [RGBAColor; 11] = [
    c(142, 1, 82),
    c(197, 27, 125),
    c(222, 119, 174),
    c(241, 182, 218),
    c(253, 224, 239),
    c(247, 247, 247),
    c(230, 245, 208),
    c(184, 225, 134),
    c(127, 188, 65),
    c(77, 146, 33),
    c(39, 100, 25),
];

static PRGN: [RGBAColor; 11] = [
    c(64, 0, 75),
    c(118, 42, 131),
    c(153, 112, 171),
    c(194, 165, 207),
    c(231, 212, 232),
    c(247, 247, 247),
    c(217, 240, 211),
    c(166, 219, 160),
    c(90, 174, 97),
    c(27, 120, 55),
    c(0, 68, 27),
];

static BRBG: [RGBAColor; 11] = [
    c(84, 48, 5),
    c(140, 81, 10),
    c(191, 129, 45),
    c(223, 194, 125),
    c(246, 232, 195),
    c(245, 245, 245),
    c(199, 234, 229),
    c(128, 205, 193),
    c(53, 151, 143),
    c(1, 102, 94),
    c(0, 60, 48),
];

static PUOR: [RGBAColor; 11] = [
    c(127, 59, 8),
    c(179, 88, 6),
    c(224, 130, 20),
    c(253, 184, 99),
    c(254, 224, 182),
    c(247, 247, 247),
    c(216, 218, 235),
    c(178, 171, 210),
    c(128, 115, 172),
    c(84, 39, 136),
    c(45, 0, 75),
];

static RDGY: [RGBAColor; 11] = [
    c(103, 0, 31),
    c(178, 24, 43),
    c(214, 96, 77),
    c(244, 165, 130),
    c(253, 219, 199),
    c(255, 255, 255),
    c(224, 224, 224),
    c(186, 186, 186),
    c(135, 135, 135),
    c(77, 77, 77),
    c(26, 26, 26),
];

static RDYLBU: [RGBAColor; 11] = [
    c(165, 0, 38),
    c(215, 48, 39),
    c(244, 109, 67),
    c(253, 174, 97),
    c(254, 224, 144),
    c(255, 255, 191),
    c(224, 243, 248),
    c(171, 217, 233),
    c(116, 173, 209),
    c(69, 117, 180),
    c(49, 54, 149),
];

static RDYLGN: [RGBAColor; 11] = [
    c(165, 0, 38),
    c(215, 48, 39),
    c(244, 109, 67),
    c(253, 174, 97),
    c(254, 224, 139),
    c(255, 255, 191),
    c(217, 239, 139),
    c(166, 217, 106),
    c(102, 189, 99),
    c(26, 152, 80),
    c(0, 104, 55),
];

// ─── Perceptual ─────────────────────────────────────────────

static VIRIDIS: [RGBAColor; 16] = [
    c(68, 1, 84),
    c(72, 26, 108),
    c(71, 47, 126),
    c(65, 68, 135),
    c(57, 86, 140),
    c(47, 104, 142),
    c(38, 121, 142),
    c(31, 138, 141),
    c(30, 155, 138),
    c(42, 172, 130),
    c(70, 188, 115),
    c(109, 202, 93),
    c(155, 213, 67),
    c(200, 222, 39),
    c(240, 229, 30),
    c(253, 231, 37),
];

static MAGMA: [RGBAColor; 16] = [
    c(0, 0, 4),
    c(16, 12, 50),
    c(41, 17, 90),
    c(72, 12, 110),
    c(101, 19, 110),
    c(131, 29, 103),
    c(160, 42, 93),
    c(187, 55, 84),
    c(213, 72, 72),
    c(232, 99, 62),
    c(247, 131, 57),
    c(254, 167, 69),
    c(254, 203, 99),
    c(252, 235, 141),
    c(252, 254, 188),
    c(252, 253, 191),
];

static PLASMA: [RGBAColor; 16] = [
    c(13, 8, 135),
    c(53, 5, 157),
    c(82, 1, 163),
    c(109, 1, 159),
    c(133, 7, 147),
    c(156, 23, 127),
    c(175, 42, 106),
    c(192, 61, 85),
    c(206, 82, 66),
    c(218, 105, 46),
    c(228, 130, 24),
    c(236, 157, 6),
    c(240, 185, 11),
    c(239, 213, 38),
    c(232, 240, 73),
    c(240, 249, 33),
];

static INFERNO: [RGBAColor; 16] = [
    c(0, 0, 4),
    c(14, 11, 49),
    c(39, 15, 90),
    c(67, 10, 107),
    c(95, 13, 106),
    c(122, 21, 97),
    c(149, 33, 81),
    c(174, 49, 60),
    c(196, 69, 38),
    c(215, 95, 15),
    c(231, 124, 3),
    c(243, 155, 7),
    c(250, 189, 28),
    c(252, 222, 67),
    c(247, 252, 118),
    c(252, 255, 164),
];

// ─── ggsci journal palettes (as bundled by ggpubr / ggsci) ──

/// Nature Publishing Group (`ggsci::pal_npg("nrc")`).
static NPG: [RGBAColor; 10] = [
    c(230, 75, 53),
    c(77, 187, 213),
    c(0, 160, 135),
    c(60, 84, 136),
    c(243, 155, 127),
    c(132, 145, 180),
    c(145, 209, 194),
    c(220, 0, 0),
    c(126, 97, 72),
    c(176, 156, 133),
];

/// Science / AAAS (`ggsci::pal_aaas`).
static AAAS: [RGBAColor; 10] = [
    c(59, 73, 146),
    c(238, 0, 0),
    c(0, 139, 69),
    c(99, 24, 121),
    c(0, 130, 128),
    c(187, 0, 33),
    c(95, 85, 155),
    c(162, 0, 86),
    c(128, 129, 128),
    c(27, 25, 25),
];

/// New England Journal of Medicine (`ggsci::pal_nejm`).
static NEJM: [RGBAColor; 8] = [
    c(188, 60, 41),
    c(0, 114, 181),
    c(225, 135, 39),
    c(32, 133, 78),
    c(120, 118, 177),
    c(111, 153, 173),
    c(255, 220, 145),
    c(238, 76, 151),
];

/// The Lancet (`ggsci::pal_lancet("lanonc")`).
static LANCET: [RGBAColor; 9] = [
    c(0, 70, 139),
    c(237, 0, 0),
    c(66, 181, 64),
    c(0, 153, 180),
    c(146, 94, 159),
    c(253, 175, 145),
    c(173, 0, 42),
    c(173, 182, 182),
    c(27, 25, 25),
];

/// JAMA — Journal of the American Medical Association (`ggsci::pal_jama`).
static JAMA: [RGBAColor; 7] = [
    c(55, 78, 85),
    c(223, 143, 68),
    c(0, 161, 213),
    c(178, 71, 69),
    c(121, 175, 151),
    c(106, 101, 153),
    c(128, 121, 107),
];

/// Journal of Clinical Oncology (`ggsci::pal_jco`).
static JCO: [RGBAColor; 10] = [
    c(0, 115, 194),
    c(239, 192, 0),
    c(134, 134, 134),
    c(205, 83, 76),
    c(122, 166, 220),
    c(0, 60, 103),
    c(143, 119, 0),
    c(59, 59, 59),
    c(167, 48, 48),
    c(74, 105, 144),
];

/// D3.js category10 (`ggsci::pal_d3("category10")`).
static D3: [RGBAColor; 10] = [
    c(31, 119, 180),
    c(255, 127, 14),
    c(44, 160, 44),
    c(214, 39, 40),
    c(148, 103, 189),
    c(140, 86, 75),
    c(227, 119, 194),
    c(127, 127, 127),
    c(188, 189, 34),
    c(23, 190, 207),
];
