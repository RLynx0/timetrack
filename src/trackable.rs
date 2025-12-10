use std::{
    collections::{HashMap, VecDeque},
    error,
    fmt::Display,
    rc::Rc,
    str::FromStr,
};

pub const BUILTIN_ACTIVITY_IDLE_NAME: &str = "Idle";
pub const BUILTIN_ACTIVITY_IDLE_WBS: &str = "Idle";

#[derive(Debug, Clone)]
pub enum ParseActivityErr {
    NoPath,
    NoName,
    NoWbs,
}
impl error::Error for ParseActivityErr {}
impl Display for ParseActivityErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseActivityErr::NoPath => write!(f, "missing path"),
            ParseActivityErr::NoWbs => write!(f, "missing wbs"),
            ParseActivityErr::NoName => write!(f, "path doesn't end in a name"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Activity {
    path: VecDeque<Rc<str>>,
    leaf: ActivityLeaf,
}
impl Activity {
    pub fn builtin_idle() -> Self {
        Activity {
            path: VecDeque::new(),
            leaf: ActivityLeaf {
                name: Rc::from(BUILTIN_ACTIVITY_IDLE_NAME),
                wbs: Rc::from(BUILTIN_ACTIVITY_IDLE_WBS),
                default_description: None,
            },
        }
    }

    pub fn full_path(&self) -> String {
        let path: String = self.path.iter().map(|n| format!("{n}/")).collect();
        format!("{}{}", path, self.leaf.name)
    }
    pub fn leaf_name(&self) -> &str {
        &self.leaf.name
    }
    pub fn wbs(&self) -> &str {
        &self.leaf.wbs
    }
    pub fn description(&self) -> Option<&str> {
        self.leaf.default_description.as_deref()
    }
}
impl From<ActivityLeaf> for Activity {
    fn from(leaf: ActivityLeaf) -> Self {
        Activity {
            path: VecDeque::new(),
            leaf,
        }
    }
}
impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path: String = self.path.iter().map(|s| format!("{s}/")).collect();
        let descr = self.leaf.default_description.as_deref().unwrap_or_default();
        let name = &self.leaf.name;
        let wbs = &self.leaf.wbs;
        write!(f, "{path}{name}\t{wbs}\t{descr}")
    }
}
impl FromStr for Activity {
    type Err = ParseActivityErr;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut fields = input.split("\t");
        let path = fields.next().ok_or(ParseActivityErr::NoPath)?;
        let wbs = fields.next().map(Rc::from).ok_or(ParseActivityErr::NoWbs)?;
        let default_description = match fields.next() {
            None | Some("") => None,
            Some(d) => Some(Rc::from(d)),
        };
        let mut path: VecDeque<Rc<str>> = path.split("/").map(Rc::from).collect();
        let name = path.pop_back().ok_or(ParseActivityErr::NoName)?;
        if name.is_empty() {
            return Err(ParseActivityErr::NoName);
        }
        let leaf = ActivityLeaf {
            name,
            wbs: Rc::from(wbs),
            default_description,
        };

        Ok(Activity { path, leaf })
    }
}

#[derive(Debug, Clone)]
pub struct ActivityCategory {
    pub branches: HashMap<Rc<str>, Self>,
    pub leafs: HashMap<Rc<str>, ActivityLeaf>,
}
impl ActivityCategory {
    pub fn as_activities(self) -> Vec<Activity> {
        let map_branch = |(name, category): (Rc<str>, ActivityCategory)| {
            category.as_activities().into_iter().map(move |mut a| {
                a.path.push_front(name.clone());
                a
            })
        };
        let branches = self.branches.into_iter().flat_map(map_branch);
        let leafs = self.leafs.into_values().map(Activity::from);
        branches.chain(leafs).collect()
    }
    pub fn as_activities_sorted(self) -> Vec<Activity> {
        let map_branch = |(name, category): (Rc<str>, ActivityCategory)| {
            category.as_activities().into_iter().map(move |mut a| {
                a.path.push_front(name.clone());
                a
            })
        };

        let mut branches: Vec<_> = self.branches.into_iter().collect();
        branches.sort_by(|(a, _), (b, _)| a.cmp(b));
        let branches = branches.into_iter().flat_map(map_branch);

        let mut leafs: Vec<_> = self.leafs.into_values().collect();
        leafs.sort_by(|a, b| a.name().cmp(b.name()));
        let leafs = leafs.into_iter().map(Activity::from);

        branches.chain(leafs).collect()
    }
}
impl<I> From<I> for ActivityCategory
where
    I: IntoIterator<Item = Activity>,
{
    fn from(activities: I) -> Self {
        let mut branches = HashMap::new();
        let mut leafs = HashMap::new();
        for mut activity in activities {
            if activity.path.is_empty() {
                leafs.insert(activity.leaf.name.clone(), activity.leaf);
                continue;
            }
            branches
                .entry(activity.path.pop_front().unwrap())
                .or_insert(Vec::new())
                .push(activity);
        }
        let branches = branches
            .into_iter()
            .map(|(k, v)| (k, ActivityCategory::from(v)))
            .collect();
        ActivityCategory { branches, leafs }
    }
}

#[derive(Debug, Clone)]
pub struct ActivityLeaf {
    name: Rc<str>,
    wbs: Rc<str>,
    default_description: Option<Rc<str>>,
}
impl ActivityLeaf {
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
