use std::fmt::Display;

#[derive(Clone, Debug, Default)]
pub struct PrintOptions {
    pub colors: Option<ColorOptions>,
    pub chars: CharOptions,
}
#[derive(Clone, Debug, Default)]
pub struct ColorOptions {
    pub headers: AnsiiColor,
    pub lines: AnsiiColor,
}
#[derive(Clone, Debug, Default)]
pub enum AnsiiColor {
    #[default]
    None,
    Red,
    Yellow,
    Green,
    Cyan,
    Blue,
    Magenta,
}
impl Display for AnsiiColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ansii_code = match self {
            AnsiiColor::None => "0",
            AnsiiColor::Red => "31",
            AnsiiColor::Yellow => "33",
            AnsiiColor::Green => "32",
            AnsiiColor::Cyan => "35",
            AnsiiColor::Blue => "34",
            AnsiiColor::Magenta => "36",
        };
        write!(f, "\u{001b}[{ansii_code}m")
    }
}
#[derive(Clone, Debug)]
pub struct CharOptions {
    caps: Option<CapOptions>,
    v: char,
    h: char,
    vr: char,
    vl: char,
    hv: char,
}
#[derive(Clone, Debug)]
struct CapOptions {
    dr: char,
    dl: char,
    ur: char,
    ul: char,
    hd: char,
    hu: char,
}
impl CharOptions {
    pub fn sharp() -> Self {
        CharOptions {
            caps: Some(CapOptions {
                dr: '┌',
                dl: '┐',
                ur: '└',
                ul: '┘',
                hd: '┬',
                hu: '┴',
            }),
            v: '│',
            h: '─',
            vr: '├',
            vl: '┤',
            hv: '┼',
        }
    }
    pub fn rounded() -> Self {
        CharOptions {
            caps: Some(CapOptions {
                dr: '╭',
                dl: '╮',
                ur: '╰',
                ul: '╯',
                hd: '┬',
                hu: '┴',
            }),
            v: '│',
            h: '─',
            vr: '├',
            vl: '┤',
            hv: '┼',
        }
    }
    pub fn ascii_markdown() -> Self {
        CharOptions {
            caps: None,
            v: '|',
            h: '-',
            vr: '|',
            vl: '|',
            hv: '|',
        }
    }
}
impl Default for CharOptions {
    fn default() -> Self {
        Self::ascii_markdown()
    }
}

pub struct Table<K, V> {
    keys: Vec<K>,
    columns: Vec<Vec<V>>,
}
impl<K, V> Table<K, V>
where
    K: Display,
    V: Display,
{
    pub fn to_string_with_options(&self, print_options: &PrintOptions) -> String {
        let copt = &print_options.chars;
        let mut out = String::new();
        let mut widths = Vec::new();
        for (i, k) in self.keys.iter().enumerate() {
            let width = self.columns[i]
                .iter()
                .map(|s| s.to_string().chars().count())
                .chain(Some(k.to_string().chars().count()))
                .max()
                .unwrap_or_default();
            widths.push(width);
        }

        let (c_header, c_line, c_reset) = match &print_options.colors {
            Some(c) => (
                c.headers.to_string(),
                c.lines.to_string(),
                AnsiiColor::None.to_string(),
            ),
            None => (String::new(), String::new(), String::new()),
        };

        // Conditionally print top table cap
        out.push_str(&c_line);
        if let Some(co) = &copt.caps {
            for (i, w) in widths.iter().enumerate() {
                out.push(if i == 0 { co.dr } else { co.hd });
                out.push_str(&copt.h.to_string().repeat(*w + 2));
            }
            out.push(co.dl);
            out.push('\n');
        }

        // Print table headers
        out.push(copt.v);
        for (k, w) in self.keys.iter().zip(&widths) {
            let k = k.to_string();
            let space = " ".repeat(w - k.chars().count());
            out.push_str(&format!(" {c_header}{k}{space} {c_line}{}", copt.v));
        }
        out.push('\n');

        // Print header separator
        for (i, w) in widths.iter().enumerate() {
            out.push(if i == 0 { copt.vr } else { copt.hv });
            out.push_str(&copt.h.to_string().repeat(*w + 2));
        }
        out.push(copt.vl);
        out.push_str(&c_reset);

        // Print table rows
        let complete_rows = self
            .columns
            .iter()
            .map(|vs| vs.len())
            .max()
            .unwrap_or_default();
        for r in 0..complete_rows {
            out.push('\n');
            out.push_str(&c_line);
            out.push(copt.v);
            for (i, width) in widths.iter().enumerate() {
                let v = self.columns[i][r].to_string();
                let space = " ".repeat(width - v.chars().count());
                out.push_str(&format!(" {c_reset}{v}{space} {c_line}{}", copt.v));
            }
        }

        // Conditionally print bottom table cap
        if let Some(co) = &copt.caps {
            out.push('\n');
            for (i, w) in widths.iter().enumerate() {
                out.push(if i == 0 { co.ur } else { co.hu });
                out.push_str(&copt.h.to_string().repeat(*w + 2));
            }
            out.push(co.ul);
        }

        out.push_str(&c_reset);
        out
    }
}

impl<I, K, Vs, V> From<I> for Table<K, V>
where
    K: Clone,
    I: IntoIterator<Item = (K, Vs)>,
    Vs: IntoIterator<Item = V>,
{
    fn from(value: I) -> Self {
        let mut columns = Vec::new();
        let mut keys = Vec::new();
        for (k, vs) in value {
            keys.push(k.clone());
            columns.push(vs.into_iter().collect());
        }
        Table { keys, columns }
    }
}

impl<K, V> Display for Table<K, V>
where
    K: Display,
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opts = PrintOptions::default();
        write!(f, "{}", self.to_string_with_options(&opts))
    }
}
