use std::{
    borrow::Cow,
    fmt::Display,
    io::{self, IsTerminal, Write},
    rc::Rc,
};

use owo_colors::{OwoColorize, Stream};
use unicode_width::UnicodeWidthStr;

const MIN_COL_WIDTH: usize = 1;

#[cfg(test)]
mod tests;

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
    ($options:expr; $($k:expr => $vs: expr,)*) => {
        $crate::printable::Table::from([
            $(($k, $vs)),*
        ]).with_options($options).print();
    };
}

#[derive(Clone, Debug, Default)]
pub struct TablePrintOptions {
    pub chars: TableCharOptions,
    pub width: TableWidth,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub enum TableWidth {
    #[default]
    Auto,
    Unbounded,
    Fixed(usize),
}

impl TableWidth {
    fn resolve(&self) -> Option<usize> {
        match self {
            Self::Auto => {
                let stdout = io::stdout();
                if !stdout.is_terminal() {
                    return None;
                }
                terminal_size::terminal_size_of(stdout)
                    .map(|(terminal_size::Width(width), _)| width as usize)
                    .filter(|&width| width > 0)
            }
            Self::Unbounded => None,
            Self::Fixed(width) => Some(*width),
        }
    }
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn truncate_to_width(text: &str, width: usize) -> Cow<'_, str> {
    if display_width(text) <= width {
        return Cow::Borrowed(text);
    }
    let budget = match width {
        6.. => width - 3, // abc... | abcd...
        4.. => width - 2, // ab.. | abc..
        n => n,           // a | ab | abc
    };
    let mut end = 0;
    for (index, ch) in text.char_indices() {
        let next = index + ch.len_utf8();
        if display_width(&text[..next]) > budget {
            break;
        }
        end = next;
    }
    let dots = (width - display_width(&text[..end])).min(3);
    Cow::Owned(format!("{}{}", &text[..end], ".".repeat(dots)))
}

fn fit_cell(text: &str, width: usize) -> (Cow<'_, str>, String) {
    let text = truncate_to_width(text, width);
    let padding = " ".repeat(width - display_width(&text));
    (text, padding)
}

fn fit_widths(natural: &[usize], terminal_width: usize) -> Vec<usize> {
    let budget = terminal_width.saturating_sub(3 * natural.len() + 1);
    let total: usize = natural.iter().sum();
    if total <= budget {
        return natural.to_vec();
    }
    if natural.iter().map(|&w| w.min(MIN_COL_WIDTH)).sum::<usize>() > budget {
        return natural.iter().map(|&w| w.min(MIN_COL_WIDTH)).collect();
    }

    // Find a shared ceiling, shrinking the widest columns first.
    let mut low = MIN_COL_WIDTH;
    let mut high = natural.iter().copied().max().unwrap_or(MIN_COL_WIDTH);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if natural.iter().map(|&w| w.min(mid)).sum::<usize>() <= budget {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    let mut widths: Vec<_> = natural.iter().map(|&w| w.min(low)).collect();
    let mut remainder = budget - widths.iter().sum::<usize>();
    for (width, &original) in widths.iter_mut().zip(natural) {
        if remainder > 0 && *width < original {
            *width += 1;
            remainder -= 1;
        }
    }
    widths
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
    #[cfg(test)]
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
        let max_width = self.options.width.resolve();
        self.render(&mut io::stdout().lock(), max_width)
            .expect("failed to print table");
    }

    fn render(&self, output: &mut impl Write, max_width: Option<usize>) -> io::Result<()> {
        let copt = &self.options.chars;
        let keys: Vec<_> = self.keys.iter().map(ToString::to_string).collect();
        let columns: Vec<Vec<_>> = self
            .columns
            .iter()
            .map(|column| column.iter().map(ToString::to_string).collect())
            .collect();
        let mut widths = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let width = columns[i]
                .iter()
                .map(|s| display_width(s))
                .chain(Some(display_width(k)))
                .max()
                .unwrap_or_default();
            widths.push(width);
        }
        if let Some(limit) = max_width {
            widths = fit_widths(&widths, limit);
        }

        if let Some(co) = &copt.caps {
            write_rule(output, &widths, copt.h, co.dr, co.hd, co.dl)?;
        }

        // Print table headers
        write!(output, "{}", copt.v)?;
        for (k, w) in keys.iter().zip(&widths) {
            let (k, space) = fit_cell(k, *w);
            write!(
                output,
                " {}{} {}",
                k.if_supports_color(Stream::Stdout, |n| n.blue()),
                space,
                copt.v
            )?;
        }
        writeln!(output)?;

        write_rule(output, &widths, copt.h, copt.vr, copt.hv, copt.vl)?;

        // Print table rows
        let complete_rows = columns.iter().map(|vs| vs.len()).max().unwrap_or_default();
        let rows = (0..complete_rows).map(|r| columns.iter().map(move |column| &column[r]));
        for row in rows {
            for (v, width) in row.zip(&widths) {
                let (v, space) = fit_cell(v, *width);
                write!(output, "{} {v}{space} ", copt.v)?;
            }
            writeln!(output, "{}", copt.v)?;
        }

        if let Some(co) = &copt.caps {
            write_rule(output, &widths, copt.h, co.ur, co.hu, co.ul)?;
        }
        Ok(())
    }
}

fn write_rule(
    output: &mut impl Write,
    widths: &[usize],
    horizontal: char,
    left: char,
    middle: char,
    right: char,
) -> io::Result<()> {
    for (index, width) in widths.iter().enumerate() {
        let junction = if index == 0 { left } else { middle };
        write!(
            output,
            "{junction}{}",
            horizontal.to_string().repeat(width + 2)
        )?;
    }
    writeln!(output, "{right}")
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
