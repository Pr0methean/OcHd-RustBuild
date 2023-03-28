use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Task<'a> {
    deps: &'a Vec<Task<'a>>,
    name: &'static str,
}

impl <'a> fmt::Display for Task<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}