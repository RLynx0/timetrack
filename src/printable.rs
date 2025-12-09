use std::{
    fmt::{Display, Write},
    rc::Rc,
};

#[macro_export]
macro_rules! print_smart_list {
    ($($k:expr => $v: expr,)*) => { print_smart_list!([$(($k, $v),)*]); };
    ($kvs: expr) => {
        println!("{}", $crate::printable::AlignedList::from($kvs)
            .with_options($crate::printable::ListPrintOptions {
                colors: std::io::IsTerminal::is_terminal(&std::io::stdout()).then_some(
                    $crate::printable::ColorOptions {
                        headers: $crate::printable::AnsiiColor::Blue,
                        lines: $crate::printable::AnsiiColor::None,
                    }
                ),
                ..Default::default()
            }
        ));
    };
}

#[macro_export]
macro_rules! print_smart_table {
    ($($k:expr => $vs: expr,)*) => {
        println!("{}", $crate::printable::Table::from([
            $(($k, $vs)),*
        ]).with_options($crate::printable::TablePrintOptions {
            chars: $crate::printable::TableCharOptions::rounded(),
            colors: std::io::IsTerminal::is_terminal(&std::io::stdout()).then_some(
                $crate::printable::ColorOptions {
                    headers: $crate::printable::AnsiiColor::Blue,
                    lines: $crate::printable::AnsiiColor::None,
                }
            ),
        }));
    };
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

#[derive(Clone, Debug, Default)]
pub struct ColorOptions {
    pub headers: AnsiiColor,
    pub lines: AnsiiColor,
}

#[derive(Clone, Debug, Default)]
pub struct TablePrintOptions {
    pub colors: Option<ColorOptions>,
    pub chars: TableCharOptions,
}
#[derive(Clone, Debug)]
pub struct TableCharOptions {
    caps: Option<TableCapOptions>,
    v: char,
    h: char,
    vr: char,
    vl: char,
    hv: char,
}
#[derive(Clone, Debug)]
struct TableCapOptions {
    dr: char,
    dl: char,
    ur: char,
    ul: char,
    hd: char,
    hu: char,
}
impl TableCharOptions {
    pub fn sharp() -> Self {
        TableCharOptions {
            caps: Some(TableCapOptions {
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
        TableCharOptions {
            caps: Some(TableCapOptions {
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
        TableCharOptions {
            caps: None,
            v: '|',
            h: '-',
            vr: '|',
            vl: '|',
            hv: '|',
        }
    }
}
impl Default for TableCharOptions {
    fn default() -> Self {
        Self::ascii_markdown()
    }
}

#[derive(Clone, Debug)]
pub struct Table<K, V> {
    keys: Vec<K>,
    columns: Vec<Vec<V>>,
    options: TablePrintOptions,
}
impl<K, V> Table<K, V> {
    pub fn with_options(&mut self, options: TablePrintOptions) -> &mut Self {
        self.options = options;
        self
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
        Table {
            keys,
            columns,
            options: TablePrintOptions::default(),
        }
    }
}
impl<K, V> Display for Table<K, V>
where
    K: Display,
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let copt = &self.options.chars;
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

        let (c_header, c_line, c_reset) = match &self.options.colors {
            Some(c) => (
                c.headers.to_string(),
                c.lines.to_string(),
                AnsiiColor::None.to_string(),
            ),
            None => (String::new(), String::new(), String::new()),
        };

        // Conditionally print top table cap
        f.write_str(&c_line)?;
        if let Some(co) = &copt.caps {
            for (i, w) in widths.iter().enumerate() {
                f.write_char(if i == 0 { co.dr } else { co.hd })?;
                f.write_str(&copt.h.to_string().repeat(*w + 2))?;
            }
            f.write_char(co.dl)?;
            f.write_char('\n')?;
        }

        // Print table headers
        f.write_char(copt.v)?;
        for (k, w) in self.keys.iter().zip(&widths) {
            let k = k.to_string();
            let space = " ".repeat(w - k.chars().count());
            write!(f, " {c_header}{k}{space} {c_line}{}", copt.v)?;
        }
        f.write_char('\n')?;

        // Print header separator
        for (i, w) in widths.iter().enumerate() {
            f.write_char(if i == 0 { copt.vr } else { copt.hv })?;
            f.write_str(&copt.h.to_string().repeat(*w + 2))?;
        }
        f.write_char(copt.vl)?;
        f.write_str(&c_reset)?;

        // Print table rows
        let complete_rows = self
            .columns
            .iter()
            .map(|vs| vs.len())
            .max()
            .unwrap_or_default();
        for r in 0..complete_rows {
            f.write_char('\n')?;
            f.write_str(&c_line)?;
            f.write_char(copt.v)?;
            for (i, width) in widths.iter().enumerate() {
                let v = self.columns[i][r].to_string();
                let space = " ".repeat(width - v.chars().count());
                write!(f, " {c_reset}{v}{space} {c_line}{}", copt.v)?;
            }
        }

        // Conditionally print bottom table cap
        if let Some(co) = &copt.caps {
            f.write_char('\n')?;
            for (i, w) in widths.iter().enumerate() {
                f.write_char(if i == 0 { co.ur } else { co.hu })?;
                f.write_str(&copt.h.to_string().repeat(*w + 2))?;
            }
            f.write_char(co.ul)?;
        }

        f.write_str(&c_reset)
    }
}

#[derive(Clone, Debug)]
pub struct ListPrintOptions {
    pub bullet: Rc<str>,
    pub colors: Option<ColorOptions>,
}
impl Default for ListPrintOptions {
    fn default() -> Self {
        ListPrintOptions {
            bullet: Rc::from("-> "),
            colors: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AlignedList<K, V> {
    keys: Vec<K>,
    vals: Vec<V>,
    options: ListPrintOptions,
}
impl<K, V> AlignedList<K, V> {
    pub fn with_options(&mut self, options: ListPrintOptions) -> &mut Self {
        self.options = options;
        self
    }
}
impl<I, K, V> From<I> for AlignedList<K, V>
where
    I: IntoIterator<Item = (K, V)>,
{
    fn from(value: I) -> Self {
        let (keys, vals) = value.into_iter().unzip();
        AlignedList {
            keys,
            vals,
            options: ListPrintOptions::default(),
        }
    }
}
impl<K, V> Display for AlignedList<K, V>
where
    K: Display,
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (col_l, col_k, col_r) = match &self.options.colors {
            Some(co) => (
                co.lines.to_string(),
                co.headers.to_string(),
                AnsiiColor::None.to_string(),
            ),
            None => (String::new(), String::new(), String::new()),
        };
        let filtered = self
            .keys
            .iter()
            .zip(&self.vals)
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .filter(|(_, v)| !v.is_empty())
            .collect::<Vec<_>>();
        let keys_width = filtered
            .iter()
            .map(|(k, _)| k.chars().count())
            .max()
            .unwrap_or_default();
        for (i, (k, v)) in filtered.into_iter().enumerate() {
            let space = " ".repeat(keys_width - k.chars().count());
            let bullet = &self.options.bullet;
            if i != 0 {
                f.write_char('\n')?;
            }
            write!(f, "{col_l}{bullet}{col_k}{k}{space} {col_l}: {col_r}{v}")?;
        }
        Ok(())
    }
}
