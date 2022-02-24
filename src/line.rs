use {lazy_static::lazy_static, regex::Regex, std::collections::HashSet};

lazy_static! {
    static ref COMMENT: Regex = Regex::new("//.*").unwrap();
}

pub struct Annotations {
    pub matches: Vec<(usize, usize)>,
    pub match_indices: HashSet<usize>,
    pub comment_indices: HashSet<usize>,
}

pub struct Line(pub String, pub Annotations);

impl Line {
    pub fn new(s: String, re: Option<&Regex>) -> Self {
        let a = Annotations {
            matches: Vec::new(),
            match_indices: HashSet::new(),
            comment_indices: COMMENT.find_iter(&s).flat_map(|m| m.range()).collect(),
        };
        let mut line = Line(s, a);
        line.annotate(re);
        line
    }

    pub fn annotate(&mut self, re: Option<&Regex>) {
        self.1.matches.clear();
        self.1.match_indices.clear();
        if let Some(re) = re {
            for m in re.find_iter(&self.0) {
                self.1.matches.push((m.start(), m.end()));
                self.1.match_indices.extend(m.range());
            }
        }
    }
}
