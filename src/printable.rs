use std::{fmt::Display, rc::Rc};

use owo_colors::{OwoColorize, Stream};

#[macro_export]
macro_rules! print_smart_list {
    ($($k:expr => $v: expr,)*) => { print_smart_list!([$(($k, $v),)*]); };
    ($kvs: expr) => {
        $crate::printable::AlignedList::from($kvs)
            .with_options($crate::printable::ListPrintOptions::default())
            .print();
    };
}

#[macro_export]
macro_rules! print_smart_table {
    ($($k:expr => $vs: expr,)*) => {
        $crate::printable::Table::from([
            $(($k, $vs)),*
        ]).with_options($crate::printable::TablePrintOptions {
            chars: $crate::printable::TableCharOptions::rounded(),
        }).print();
    };
}

#[derive(Clone, Debug, Default)]
pub struct TablePrintOptions {
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
impl<K, V> Table<K, V>
where
    K: Display,
    V: Display,
{
    pub fn print(&self) {
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

        // Conditionally print top table cap
        if let Some(co) = &copt.caps {
            for (i, w) in widths.iter().enumerate() {
                let v_char = if i == 0 { co.dr } else { co.hd };
                let h_line = &copt.h.to_string().repeat(*w + 2);
                print!("{v_char}{h_line}");
            }
            println!("{}", co.dl);
        }

        // Print table headers
        print!("{}", copt.v);
        for (k, w) in self.keys.iter().zip(&widths) {
            let k = k.to_string();
            let space = " ".repeat(w - k.chars().count());
            print!(
                " {}{} {}",
                k.if_supports_color(Stream::Stdout, |n| n.blue()),
                space,
                copt.v
            );
        }
        println!();

        // Print header separator
        for (i, w) in widths.iter().enumerate() {
            let v_char = if i == 0 { copt.vr } else { copt.hv };
            let h_line = &copt.h.to_string().repeat(*w + 2);
            print!("{v_char}{h_line}");
        }
        println!("{}", copt.vl);

        // Print table rows
        let complete_rows = self
            .columns
            .iter()
            .map(|vs| vs.len())
            .max()
            .unwrap_or_default();
        for r in 0..complete_rows {
            for (i, width) in widths.iter().enumerate() {
                let v = self.columns[i][r].to_string();
                let space = " ".repeat(width - v.chars().count());
                print!("{} {v}{space} ", copt.v);
            }
            println!("{}", copt.v);
        }

        // Conditionally print bottom table cap
        if let Some(co) = &copt.caps {
            for (i, w) in widths.iter().enumerate() {
                let v_char = if i == 0 { co.ur } else { co.hu };
                let h_line = &copt.h.to_string().repeat(*w + 2);
                print!("{v_char}{h_line}");
            }
            println!("{}", co.ul);
        }
    }
}

#[derive(Clone, Debug)]
pub struct ListPrintOptions {
    pub bullet: Rc<str>,
}
impl Default for ListPrintOptions {
    fn default() -> Self {
        ListPrintOptions {
            bullet: Rc::from("-> "),
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
impl<K, V> AlignedList<K, V>
where
    K: Display,
    V: Display,
{
    pub fn print(&self) {
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
        for (k, v) in filtered.into_iter() {
            let space = " ".repeat(keys_width - k.chars().count());
            let bullet = &self.options.bullet;
            println!(
                "{bullet}{}{space} : {v}",
                k.if_supports_color(Stream::Stdout, |n| n.blue())
            );
        }
    }
}
