use nom::{
    branch::alt,
    bytes::complete::{tag, is_a},
    character::{complete::{digit1, space1}, is_digit},
    combinator::{map, map_res, verify},
    sequence::tuple,
    IResult,
};

use crate::ast::{Command::*, Segment::*, *};

fn integer(input: &str) -> IResult<&str, u16> {
    map_res(digit1, |c: &str| c.parse())(input)
}

fn segment(input: &str) -> IResult<&str, Segment> {
    map(
        alt((
            tag("constant"),
            tag("local"),
            tag("static"),
            tag("argument"),
            tag("this"),
            tag("that"),
            tag("pointer"),
            tag("temp"),
        )),
        |seg| match seg {
            "constant" => Constant,
            "local" => Local,
            "static" => Static,
            "argument" => Argument,
            "this" => This,
            "that" => That,
            "pointer" => Pointer,
            "temp" => Temp,
            _ => panic!("Unexpected parse {}", seg),
        },
    )(input)
}

fn push(input: &str) -> IResult<&str, Command> {
    map(
        tuple((tag("push"), space1, segment, space1, integer)),
        |(_, _, segment, _, arg)| Push(segment, arg),
    )(input)
}

#[test]
fn test_push() {
    assert_eq!(push("push  pointer  32"), Ok(("", Push(Pointer, 32))));
}

fn pop(input: &str) -> IResult<&str, Command> {
    verify(
        map(
            tuple((tag("pop"), space1, segment, space1, integer)),
            |(_, _, segment, _, arg)| Pop(segment, arg),
        ),
        |p| {
            if let Pop(Constant, _) = p {
                false
            } else {
                true
            }
        },
    )(input)
}

fn prim(input: &str) -> IResult<&str, Command> {
    map(
        alt((
            tag("add"),
            tag("sub"),
            tag("neg"),
            tag("eq"),
            tag("gt"),
            tag("lt"),
            tag("and"),
            tag("or"),
            tag("not"),
        )),
        |prim| match prim {
            "add" => Add,
            "sub" => Sub,
            "neg" => Neg,
            "eq" => Eq,
            "gt" => Gt,
            "lt" => Lt,
            "and" => And,
            "or" => Or,
            "not" => Not,
            _ => panic!("Unexpected parse {}", prim),
        },
    )(input)
}

#[test]
fn test_prim() {
    assert_eq!(prim("neg"), Ok(("", Neg)));
}

fn symbol(input: &str) -> IResult<&str, String> {
    map(
        verify(
            is_a("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_.$:0123456789"),
            |c: &str| !is_digit(c.as_bytes()[0]),
        ),
        |sym: &str| sym.to_string(),
    )(input)
}

fn branching(input: &str) -> IResult<&str, Command> {
    map(
        tuple((
            alt((
                tag("label"),
                tag("goto"),
                tag("if-goto"),
            )),
            space1,
            symbol
        )),
        |(op, _, sym)| match op {
            "label" => Label(sym.to_string()),
            "goto" => Goto(sym.to_string()),
            "if-goto" => IfGoto(sym.to_string()),
            _ => panic!("Unexpected parse {}", sym)
        }
    )(input)
}

pub fn parse(input: &str) -> Vec<Command> {
    let mut commands = vec![];

    for line in input.lines() {
        let line = line.split_once("//").map(|(s, _)| s).unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }

        let res = alt((push, pop, prim, branching))(line);

        match res {
            Ok(("", command)) => commands.push(command),
            Ok((remainder, _)) => panic!("Command {} has extra parts {}", line, remainder),
            Err(line) => panic!("Invalid command {}", line),
        }
    }

    commands
}
