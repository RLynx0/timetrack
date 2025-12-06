use std::{fmt::Display, iter::FromIterator};

pub struct Table<K, V> {
    keys: Vec<K>,
    columns: Vec<Vec<V>>,
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

        write!(f, "|")?;
        for (k, width) in self.keys.iter().zip(&widths) {
            let k = k.to_string();
            write!(f, " {k}{} |", " ".repeat(width - k.chars().count()))?
        }
        write!(f, "\n|")?;
        for width in &widths {
            write!(f, "-{}-|", "-".repeat(*width))?
        }

        let complete_rows = self
            .columns
            .iter()
            .map(|vs| vs.len())
            .max()
            .unwrap_or_default();
        for r in 0..complete_rows {
            write!(f, "\n|")?;
            for (i, width) in widths.iter().enumerate() {
                let v = self.columns[i][r].to_string();
                write!(f, " {v}{} |", " ".repeat(width - v.chars().count()))?;
            }
        }

        Ok(())
    }
}
