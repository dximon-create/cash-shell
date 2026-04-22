// cash — Pipeline and Redirect types
//
// A Pipeline is what the parser produces from a single input line.
// It contains one or more stages connected by pipes, each stage
// being a command with its own redirects.
//
// Examples:
//   ls -la                      → Pipeline { stages: [ls -la] }
//   ls | grep foo               → Pipeline { stages: [ls, grep foo] }
//   cat file.txt | sort > out   → Pipeline { stages: [cat file.txt, sort >out] }

#[derive(Debug, Clone, PartialEq)]
pub struct Redirect {
    pub kind: RedirectKind,
    /// The file path (for file redirects).
    pub target: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectKind {
    /// < file  — replace stdin from file
    Stdin,
    /// > file  — replace stdout to file (truncate)
    Stdout,
    /// >> file — replace stdout to file (append)
    Append,
    /// 2> file — replace stderr to file
    Stderr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stage {
    /// argv[0]
    pub name: String,
    /// argv[1..]
    pub args: Vec<String>,
    /// Leading VAR=value pairs injected into the child environment.
    pub env: Vec<(String, String)>,
    /// File redirections for this stage.
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    /// Original raw input, kept for the audit log.
    pub raw: String,
}

impl Pipeline {
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }
}
