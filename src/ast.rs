#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Segment {
    Constant,
    Local,
    Static,
    Argument,
    This,
    That,
    Pointer,
    Temp,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    // Stack Basics
    Push(Segment, u16),
    Pop(Segment, u16),
    Add,
    Sub,
    Neg,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,

    // Control
    Label(String),
    Goto(String),
    IfGoto(String),
}
