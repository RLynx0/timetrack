use std::{fmt::Display, rc::Rc};

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
