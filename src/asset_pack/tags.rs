use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Tags {
    pub tags: HashMap<String, HashSet<String>>,
    pub sets: HashMap<String, HashSet<String>>,
}

impl Tags {
    pub fn new() -> Self {
        Tags {
            tags: HashMap::new(),
            sets: HashMap::new(),
        }
    }
}

impl Display for Tags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let indent = "    ";

        writeln!(f, "Tag and tag sets:")?;

        writeln!(f)?;
        writeln!(f, "{}Tags:", indent)?;

        for (tag, files) in self.tags.iter() {
            write!(f, "{}{}{}: [ ", indent, indent, tag)?;

            for file in files {
                write!(f, "'{}', ", file)?;
            }

            writeln!(f, " ]")?;
        }

        writeln!(f)?;
        writeln!(f, "{}Tag sets:", indent)?;

        for (set, tags) in self.tags.iter() {
            write!(f, "{}{}{}: [ ", indent, indent, set)?;

            for tag in tags {
                write!(f, "{}, ", tag)?;
            }

            writeln!(f, " ]")?;
        }

        Ok(())
    }
}
