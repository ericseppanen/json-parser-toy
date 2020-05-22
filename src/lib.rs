use nom::{branch::alt, bytes::complete::tag, IResult};
use nom::combinator::value;
use nom::character::complete::{one_of, digit0, digit1};
use nom::sequence::{pair, tuple};
use nom::combinator::{map, opt, peek, recognize};

#[derive(PartialEq, Debug, Clone)]
pub enum Node {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
}

fn json_literal(input: &str) -> IResult<&str, Node> {
    alt((
        json_float,
        json_integer,
        json_bool,
        json_null
    ))
    (input)
}

// This can be done a few different ways:
// one_of("123456789"),
// anychar("0123456789"),
// we could also extract the character value as u32 and do range checks...

fn digit1to9(input: &str) -> IResult<&str, char> {
    one_of("123456789")
    (input)
}

// unsigned_integer = zero / ( digit1-9 *DIGIT )
fn uint(input: &str) -> IResult<&str, &str> {
    alt((
        tag("0"),
        recognize(
            pair(
                digit1to9,
                digit0
            )
        )
    ))
    (input)
}

fn json_integer(input: &str) -> IResult<&str, Node> {
    let parser = recognize(
        pair(
            opt(tag("-")),
            uint
        )
    );
    map(parser, |s| {
        // FIXME: unwrap() may panic if the integer is too big.
        let n = s.parse::<i64>().unwrap();
        Node::Integer(n)
    })
    (input)
}

// number = [ minus ] int [ frac ] [ exp ]
//
//       decimal-point = %x2E       ; .
//       digit1-9 = %x31-39         ; 1-9
//       e = %x65 / %x45            ; e E
//       exp = e [ minus / plus ] 1*DIGIT
//       frac = decimal-point 1*DIGIT
//       int = zero / ( digit1-9 *DIGIT )
//       minus = %x2D               ; -
//       plus = %x2B                ; +
//       zero = %x30                ; 0

fn frac(input: &str) -> IResult<&str, &str> {
    recognize(
        pair(
            tag("."),
            digit1
        )
    )
    (input)
}

fn exp(input: &str) -> IResult<&str, &str> {
    recognize(
        tuple((
            tag("e"),
            opt(alt((
                tag("-"),
                tag("+")
            ))),
            digit1
        ))
    )
    (input)
}

fn json_float(input: &str) -> IResult<&str, Node> {
    let parser = recognize(
        tuple((
            opt(tag("-")),
            uint,
            peek(alt((
                tag("."),
                tag("e"),
            ))),
            opt(frac),
            opt(exp)
        ))
    );
    map(parser, |s| {
        // FIXME: unwrap() may panic if the value is out of range
        let n = s.parse::<f64>().unwrap();
        Node::Float(n)
    })
    (input)
}

fn json_bool(input: &str) -> IResult<&str, Node> {
    alt((
        value(Node::Bool(false), tag("false")),
        value(Node::Bool(true), tag("true")),
    ))
    (input)
}

fn json_null(input: &str) -> IResult<&str, Node> {
    value(Node::Null, tag("null"))
    (input)
}

#[test]
fn test_bool() {
    assert_eq!(json_bool("false"), Ok(("", Node::Bool(false))));
    assert_eq!(json_bool("true"), Ok(("", Node::Bool(true))));
    assert!(json_bool("foo").is_err());
}

#[test]
fn test_null() {
    assert_eq!(json_null("null"), Ok(("", Node::Null)));
}

#[test]
fn test_integer() {
    assert_eq!(json_integer("42"), Ok(("", Node::Integer(42))));
    assert_eq!(json_integer("-123"), Ok(("", Node::Integer(-123))));
    assert_eq!(json_integer("0"), Ok(("", Node::Integer(0))));
    assert_eq!(json_integer("01"), Ok(("1", Node::Integer(0))));
    // FIXME: test too-large integers once error handling is in place.
}

#[test]
fn test_float() {
    assert_eq!(json_float("42.0"), Ok(("", Node::Float(42.0))));
    assert_eq!(json_float("-123.99"), Ok(("", Node::Float(-123.99))));
    assert_eq!(json_float("6.02214086e23"), Ok(("", Node::Float(6.02214086e23))));
    assert_eq!(json_float("-1e6"), Ok(("", Node::Float(-1000000.0))));
    // FIXME: test too-large integers once error handling is in place.
}

#[test]
fn test_literal() {
    assert_eq!(json_literal("56"), Ok(("", Node::Integer(56))));
    assert_eq!(json_literal("78.0"), Ok(("", Node::Float(78.0))));
}
