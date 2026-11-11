//! Printer tracking dots: the metadata on every page.
//!
//! Many colour laser printers add a faint grid of yellow dots to every page,
//! a machine identification code with the printer's serial number and the
//! date and time of printing. The EFF decoded the Xerox DocuColor pattern in
//! 2005: 15 columns by 8 rows, one value per column, each column with a
//! parity dot. In 2017 a scanned NSA report published by a news site still
//! carried them, and they pointed straight at the printer and the minute it
//! was used.
//!
//! The layout below follows the same idea (one value per column, an odd
//! parity dot on top of each), though the column order is this city's own.

pub const COLS: usize = 15;
pub const ROWS: usize = 8;

/// A dot grid: `grid[row][col]`, row 0 holds the parity dots.
pub type Grid = [[bool; COLS]; ROWS];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrintInfo {
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    /// two digits
    pub year: u8,
    /// eight decimal digits
    pub serial: u32,
}

/// What each column holds.
pub const FIELDS: [&str; COLS] = ["unused", "minute", "hour", "day", "month", "year", "unused", "serial 1-2", "serial 3-4", "serial 5-6", "serial 7-8", "unused", "unused", "unused", "unused"];

pub fn values(info: &PrintInfo) -> [u8; COLS] {
    let s = info.serial % 100_000_000;
    let mut v = [0u8; COLS];
    v[1] = info.minute;
    v[2] = info.hour;
    v[3] = info.day;
    v[4] = info.month;
    v[5] = info.year;
    v[7] = (s / 1_000_000) as u8;
    v[8] = (s / 10_000 % 100) as u8;
    v[9] = (s / 100 % 100) as u8;
    v[10] = (s % 100) as u8;
    v
}

/// Seven data bits per column (rows 1..7, most significant at the top) and an
/// odd-parity dot in row 0.
pub fn encode(info: &PrintInfo) -> Grid {
    let mut g = [[false; COLS]; ROWS];
    for (col, v) in values(info).iter().enumerate() {
        let mut ones = 0;
        for bit in 0..7 {
            let on = (v >> (6 - bit)) & 1 == 1;
            g[bit + 1][col] = on;
            ones += on as u32;
        }
        g[0][col] = ones % 2 == 0;
    }
    g
}

pub fn column_value(g: &Grid, col: usize) -> u8 {
    (0..7).fold(0u8, |acc, bit| (acc << 1) | u8::from(g[bit + 1][col]))
}

pub fn parity_ok(g: &Grid, col: usize) -> bool {
    (0..ROWS).filter(|r| g[*r][col]).count() % 2 == 1
}

pub fn decode(g: &Grid) -> Result<PrintInfo, String> {
    if let Some(bad) = (0..COLS).find(|c| !parity_ok(g, *c)) {
        return Err(format!("column {bad} fails its parity check"));
    }
    let v: Vec<u8> = (0..COLS).map(|c| column_value(g, c)).collect();
    let serial = v[7] as u32 * 1_000_000 + v[8] as u32 * 10_000 + v[9] as u32 * 100 + v[10] as u32;
    Ok(PrintInfo { minute: v[1], hour: v[2], day: v[3], month: v[4], year: v[5], serial })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_parity() {
        let info = PrintInfo { minute: 37, hour: 21, day: 9, month: 5, year: 17, serial: 29_535_218 };
        let mut g = encode(&info);
        assert_eq!(decode(&g), Ok(info));
        assert_eq!(column_value(&g, 2), 21);
        assert!((0..COLS).all(|c| parity_ok(&g, c)));
        g[3][4] = !g[3][4];
        assert!(decode(&g).is_err());
    }
}
