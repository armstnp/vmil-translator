use crate::ast::{Command::*, Segment::*, *};

// TODO: Consider using a static-level string interner for this module
macro_rules! svec {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

fn at_c(arg: &u16) -> String {
    format!("@{arg}", arg = arg)
}

fn at_s(arg: &str) -> String {
    format!("@{arg}", arg = arg)
}

fn pointer_arg(arg: &u16) -> String {
    match arg {
        0 => "THIS",
        1 => "THAT",
        _ => panic!("Invalid pointer {}", arg),
    }
    .to_string()
}

/// Push microcode for the five base segments
fn seg_push(seg_name: &str, seg: &str, arg: &u16) -> Vec<String> {
    svec![
        format!("// push {} {}", seg_name, arg),
        at_s(seg),
        "D=M",
        at_c(arg),
        "A=A+D", // A = SEG+arg
        "D=M",   // D = value to push
        "@SP",
        "M=M+1",
        "A=M-1", // Don't need to refetch SP; this is safe
        "M=D"
    ]
}

fn seg_push_direct(seg_name: &str, arg: &u16, label: String) -> Vec<String> {
    svec![
        format!("// push {} {}", seg_name, arg),
        format!("@{}", label),
        "D=M",
        "@SP",
        "M=M+1",
        "A=M-1",
        "M=D"
    ]
}

fn seg_pop(seg_name: &str, seg: &str, arg: &u16) -> Vec<String> {
    svec![
        format!("// pop {} {}", seg_name, arg),
        at_s(seg),
        "D=M",
        at_c(arg),
        "D=A+D", // A = SEG+arg
        "@R13",
        "M=D", // Store local addr in R13
        "@SP",
        "AM=M-1", // SP--, A <- new SP (val to be popped)
        "D=M",
        "@R13",
        "A=M", // At the local's address...
        "M=D"  // ... store the popped val
    ]
}

fn seg_pop_direct(seg_name: &str, arg: &u16, label: String) -> Vec<String> {
    svec![
        format!("// pop {} {}", seg_name, arg),
        "@SP",
        "AM=M-1",
        "D=M",
        format!("@{}", label),
        "M=D"
    ]
}

fn simple_un_op(name: &str, op: char) -> Vec<String> {
    svec![format!("// {}", name), "@SP", "A=M-1", format!("M={}M", op)]
}

// i.e. no conditions or jumps, just pop and run
fn simple_bin_op(name: &str, op: char) -> Vec<String> {
    svec![
        format!("// {}", name),
        "@SP",
        "AM=M-1",              // SP--, looking at top of stack now
        "D=M",                 // Right arg in D
        "A=A-1",               // Looking at second arg of stack, will overwrite
        format!("M=M{}D", op)  // Op and overwrite second element
    ]
}

pub struct Translator<'a> {
    assembly: &'a str,
    gen_sym: usize,
}

impl<'a> Translator<'a> {
    pub fn new(assembly: &'a str) -> Self {
        Translator {
            assembly,
            gen_sym: 0,
        }
    }

    fn next_gen_sym(&mut self) -> usize {
        let tmp = self.gen_sym;
        self.gen_sym += 1;
        tmp
    }

    fn push(&self, segment: &Segment, arg: &u16) -> Vec<String> {
        match segment {
            Constant => svec![
                format!("// push constant {}", arg),
                at_c(arg),
                "D=A",
                "@SP",
                "A=M",
                "M=D",
                "@SP",
                "M=M+1"
            ],
            Local => seg_push("local", "LCL", arg),
            Argument => seg_push("argument", "ARG", arg),
            This => seg_push("this", "THIS", arg),
            That => seg_push("that", "THAT", arg),
            Static => seg_push_direct("static", arg, format!("{}.{}", self.assembly, arg)),
            Temp => seg_push_direct("temp", arg, format!("R{}", arg + 5)),
            Pointer => seg_push_direct("pointer", arg, pointer_arg(arg)),
        }
    }

    fn pop(&self, segment: &Segment, arg: &u16) -> Vec<String> {
        match segment {
            Constant => panic!("Should not pop constants"),
            Local => seg_pop("local", "LCL", arg),
            Argument => seg_pop("argument", "ARG", arg),
            This => seg_pop("this", "THIS", arg),
            That => seg_pop("that", "THAT", arg),
            Static => seg_pop_direct("static", arg, format!("{}.{}", self.assembly, arg)),
            Temp => seg_pop_direct("temp", arg, format!("R{}", arg + 5)),
            Pointer => seg_pop_direct("pointer", arg, pointer_arg(arg)),
        }
    }

    fn compare(&mut self, cmp_name: &str, jump: &str) -> Vec<String> {
        let sym = self.next_gen_sym();
        let cmp_sym = format!("{}:CMP_{}", self.assembly, sym);
        let end_sym = format!("{}:ENDCMP_{}", self.assembly, sym);
        svec![
            format!("// {}", cmp_name),
            "@SP",
            "AM=M-1", // SP--, looking at top of stack now
            "D=M",    // Right arg in D
            "A=A-1",  // Looking at second arg of stack, will overwrite
            "D=M-D",
            format!("@{}", cmp_sym),
            format!("D;J{}", jump),
            "D=0",
            format!("@{}", end_sym),
            "0;JMP",
            format!("({})", cmp_sym),
            "D=-1",
            format!("({})", end_sym),
            "@SP",
            "A=M-1",
            "M=D"
        ]
    }

    /// Convert VM label to Hack ASM symbol - for consistency across instructions
    fn label_to_sym(&self, label: &str) -> String {
        format!("{}:LABEL_{}", self.assembly, label)
    }

    fn label(&self, label: &str) -> Vec<String> {
        svec![
            format!("// label {}", label),
            format!("({})", self.label_to_sym(label))
        ]
    }

    fn goto(&self, label: &str) -> Vec<String> {
        svec![
            format!("// goto {}", label),
            format!("@{}", self.label_to_sym(label)),
            "0;JMP" // Unconditional jump
        ]
    }

    fn if_goto(&self, label: &str) -> Vec<String> {
        svec![
            format!("// if-goto {}", label),
            "@SP",
            "AM=M-1",
            "D=M",  // Stack popped into D
            format!("@{}", self.label_to_sym(label)),
            "D;JNE" // False is 0
        ]
    }

    pub fn translate(&mut self, commands: &Vec<Command>) -> Vec<String> {
        let mut instructions: Vec<String> = vec![];

        for command in commands {
            let translated = match command {
                Push(seg, arg) => self.push(seg, arg),
                Pop(seg, arg) => self.pop(seg, arg),
                Not => simple_un_op("not", '!'),
                Neg => simple_un_op("neg", '-'),
                Add => simple_bin_op("add", '+'),
                Sub => simple_bin_op("sub", '-'),
                And => simple_bin_op("and", '&'),
                Or => simple_bin_op("or", '|'),
                Eq => self.compare("eq", "EQ"),
                Gt => self.compare("gt", "GT"),
                Lt => self.compare("lt", "LT"),
                Label(sym) => self.label(sym),
                Goto(sym) => self.goto(sym),
                IfGoto(sym) => self.if_goto(sym),
            };

            for line in translated {
                instructions.push(line);
            }
        }

        instructions
    }
}
