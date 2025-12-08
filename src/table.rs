use std::fmt::Display;

pub struct PrintOptions {
    pub chars: CharOptions,
}
pub struct CharOptions {
    caps: Option<CapOptions>,
    v: char,
    h: char,
    vr: char,
    vl: char,
    hv: char,
}
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

pub struct Table<K, V> {
    keys: Vec<K>,
    columns: Vec<Vec<V>>,
}
impl<K, V> Table<K, V>
where
    K: Display,
    V: Display,
{
    pub fn to_string_with_options(&self, print_options: PrintOptions) -> String {
        let copt = print_options.chars;
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

        // Conditionally print top table cap
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
            out.push_str(&format!(" {k}{space} {}", copt.v))
        }
        out.push('\n');

        // Print header separator
        for (i, w) in widths.iter().enumerate() {
            out.push(if i == 0 { copt.vr } else { copt.hv });
            out.push_str(&copt.h.to_string().repeat(*w + 2));
        }
        out.push(copt.vl);

        // Print table rows
        let complete_rows = self
            .columns
            .iter()
            .map(|vs| vs.len())
            .max()
            .unwrap_or_default();
        for r in 0..complete_rows {
            out.push('\n');
            out.push(copt.v);
            for (i, width) in widths.iter().enumerate() {
                let v = self.columns[i][r].to_string();
                let space = " ".repeat(width - v.chars().count());
                out.push_str(&format!(" {v}{space} {}", copt.v));
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
        write!(
            f,
            "{}",
            self.to_string_with_options(PrintOptions {
                chars: CharOptions::ascii_markdown()
            })
        )
    }
}
