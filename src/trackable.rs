use std::{error, fmt::Display, rc::Rc, str::FromStr};

pub const BUILTIN_ACTIVITY_IDLE_NAME: &str = "Idle";
pub const BUILTIN_ACTIVITY_IDLE_WBS: &str = "Idle";

#[derive(Debug, Clone)]
pub enum ParseActivityErr {
    MissingName,
    MissingWbs,
}
impl error::Error for ParseActivityErr {}
impl Display for ParseActivityErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseActivityErr::MissingName => write!(f, "missing name"),
            ParseActivityErr::MissingWbs => write!(f, "missing wbs"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Activity {
    name: Rc<str>,
    wbs: Rc<str>,
    default_description: Option<Rc<str>>,
}
impl Activity {
    pub fn new(name: &str, wbs: &str, description: Option<&str>) -> Self {
        Activity {
            name: Rc::from(name),
            wbs: Rc::from(wbs),
            default_description: description.map(Rc::from),
        }
    }

    pub fn builtin_idle() -> Self {
        Activity {
            name: Rc::from(BUILTIN_ACTIVITY_IDLE_NAME),
            wbs: Rc::from(BUILTIN_ACTIVITY_IDLE_WBS),
            default_description: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn wbs(&self) -> &str {
        &self.wbs
    }
    pub fn description(&self) -> Option<&str> {
        self.default_description.as_deref()
    }
}
impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}",
            self.name,
            self.wbs,
            match &self.default_description {
                Some(d) => d,
                None => "",
            }
        )
    }
}
impl FromStr for Activity {
    type Err = ParseActivityErr;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut fields = input.split("\t");
        let name = fields.next().ok_or(ParseActivityErr::MissingName)?;
        let wbs = fields.next().ok_or(ParseActivityErr::MissingWbs)?;
        let default_description = match fields.next().map(|s| s.trim()) {
            Some("") => None,
            opt => opt.map(Rc::from),
        };
        Ok(Activity {
            name: Rc::from(name),
            wbs: Rc::from(wbs),
            default_description,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ActivityCategory {
    pub name: Rc<str>,
    pub children: Vec<ActivityItem>,
}

#[derive(Clone, Debug)]
pub enum ActivityItem {
    Leaf(Activity),
    Category(ActivityCategory),
}
impl Display for ActivityItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityItem::Leaf(activity) => write!(f, "{activity}"),
            ActivityItem::Category(category) => write!(f, "{}/\t\t", category.name),
        }
    }
}
