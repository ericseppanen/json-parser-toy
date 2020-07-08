use nom::{branch::alt, IResult};
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{one_of, digit0, digit1, multispace0};
use nom::combinator::{all_consuming, map, opt, recognize, value};
use nom::error::{ErrorKind, ParseError};
use nom::multi::{many0, separated_list};
use nom::sequence::{delimited, pair, separated_pair, tuple};
use escape8259::unescape;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum JSONParseError {
    #[error("bad integer")]
    BadInt,
    #[error("bad float")]
    BadFloat,
    #[error("bad escape sequence")]
    BadEscape,
    #[error("unknown parser error")]
    Unparseable,
}

impl<I> ParseError<I> for JSONParseError {
    fn from_error_kind(_input: I, _kind: ErrorKind) -> Self {
        // Because JSONParseError is a simplified public error type,
        // we discard the nom error parameters.
        JSONParseError::Unparseable
    }

    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Node {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Str(String),
    Array(Vec<Node>),
    Object(Vec<(String, Node)>),
}

fn json_value(input: &str) -> IResult<&str, Node, JSONParseError> {
    spacey(alt((
        json_array,
        json_object,
        json_string,
        json_float,
        json_integer,
        json_bool,
        json_null
    )))
    (input)
}

fn spacey<F, I, O, E>(f: F) -> impl Fn(I) -> IResult<I, O, E>
where
    F: Fn(I) -> IResult<I, O, E>,
    I: nom::InputTakeAtPosition,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
    E: nom::error::ParseError<I>,
{
    delimited(multispace0, f, multispace0)
}

fn json_array(input: &str) -> IResult<&str, Node, JSONParseError> {
    let parser = delimited(
        spacey(tag("[")),
        separated_list(spacey(tag(",")), json_value),
        spacey(tag("]")),
    );
    map(parser, |v| {
        Node::Array(v)
    })
    (input)
}

// "key: value", where key and value are any JSON type.
fn object_member(input: &str) -> IResult<&str, (String, Node), JSONParseError> {
    separated_pair(string_literal, spacey(tag(":")), json_value)
    (input)
}

fn json_object(input: &str) -> IResult<&str, Node, JSONParseError> {
    let parser = delimited(
        spacey(tag("{")),
        separated_list(
            spacey(tag(",")),
            object_member
        ),
        spacey(tag("}")),
    );
    map(parser, |v| {
        Node::Object(v)
    })
    (input)
}

// A character that is:
// NOT a control character (0x00 - 0x1F)
// NOT a quote character (0x22)
// NOT a backslash character (0x5C)
// Is within the unicode range (< 0x10FFFF) (this is already guaranteed by Rust char)
fn is_nonescaped_string_char(c: char) -> bool {
    let cv = c as u32;
    (cv >= 0x20) && (cv != 0x22) && (cv != 0x5C)
}

// One or more unescaped text characters
fn nonescaped_string(input: &str) -> IResult<&str, &str, JSONParseError> {
    take_while1(is_nonescaped_string_char)
    (input)
}

// There are only two types of escape allowed by RFC 8259.
// - single-character escapes \" \\ \/ \b \f \n \r \t
// - general-purpose \uXXXX
// Note: we don't enforce that escape codes are valid here.
// There must be a decoder later on.
fn escape_code(input: &str) -> IResult<&str, &str, JSONParseError> {
    recognize(
        pair(
            tag("\\"),
            alt((
                tag("\""),
                tag("\\"),
                tag("/"),
                tag("b"),
                tag("f"),
                tag("n"),
                tag("r"),
                tag("t"),
                tag("u"),
            ))
        )
    )
    (input)
}

// Zero or more text characters
fn string_body(input: &str) -> IResult<&str, &str, JSONParseError> {
    recognize(
        many0(
            alt((
                nonescaped_string,
                escape_code
            ))
        )
    )
    (input)
}

fn string_literal(input: &str) -> IResult<&str, String, JSONParseError> {
    let (remain, raw_string) = delimited(
        tag("\""),
        string_body,
        tag("\"")
    )
    (input)?;

    match unescape(raw_string) {
        Ok(s) => Ok((remain, s)),
        Err(_) => Err(nom::Err::Failure(JSONParseError::BadEscape)),
    }
}

fn json_string(input: &str) -> IResult<&str, Node, JSONParseError> {
    map(string_literal, |s| {
        Node::Str(s)
    })
    (input)
}

// This can be done a few different ways:
// one_of("123456789"),
// anychar("0123456789"),
// we could also extract the character value as u32 and do range checks...

fn digit1to9(input: &str) -> IResult<&str, char, JSONParseError> {
    one_of("123456789")
    (input)
}

// unsigned_integer = zero / ( digit1-9 *DIGIT )
fn uint(input: &str) -> IResult<&str, &str, JSONParseError> {
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

fn integer_body(input: &str) -> IResult<&str, &str, JSONParseError> {
    recognize(
        pair(
            opt(tag("-")),
            uint
        )
    )
    (input)
}

fn json_integer(input: &str) -> IResult<&str, Node, JSONParseError> {
    let (remain, raw_int) = integer_body(input)?;
    match raw_int.parse::<i64>() {
        Ok(i) => Ok((remain, Node::Integer(i))),
        Err(_) => Err(nom::Err::Failure(JSONParseError::BadInt)),
    }
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

fn frac(input: &str) -> IResult<&str, &str, JSONParseError> {
    recognize(
        pair(
            tag("."),
            digit1
        )
    )
    (input)
}

fn exp(input: &str) -> IResult<&str, &str, JSONParseError> {
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

fn float_body(input: &str) -> IResult<&str, &str, JSONParseError> {
    recognize(
        tuple((
            opt(tag("-")),
            uint,
            alt((
                recognize(pair(
                    frac,
                    opt(exp)
                )),
                exp
            )),
        ))
    )
    (input)
}

fn json_float(input: &str) -> IResult<&str, Node, JSONParseError> {
    let (remain, raw_float) = float_body(input)?;
    match raw_float.parse::<f64>() {
        Ok(f) => Ok((remain, Node::Float(f))),
        Err(_) => Err(nom::Err::Failure(JSONParseError::BadFloat)),
    }
}

fn json_bool(input: &str) -> IResult<&str, Node, JSONParseError> {
    alt((
        value(Node::Bool(false), tag("false")),
        value(Node::Bool(true), tag("true")),
    ))
    (input)
}

fn json_null(input: &str) -> IResult<&str, Node, JSONParseError> {
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
    assert_eq!(json_integer("9999999999999999999"), Err(nom::Err::Failure(JSONParseError::BadInt)));
}

#[test]
fn test_float() {
    assert_eq!(json_float("42.0"), Ok(("", Node::Float(42.0))));
    assert_eq!(json_float("-123.99"), Ok(("", Node::Float(-123.99))));
    assert_eq!(json_float("6.02214086e23"), Ok(("", Node::Float(6.02214086e23))));
    assert_eq!(json_float("-1e6"), Ok(("", Node::Float(-1000000.0))));
    // f64::from_str overflows to infinity instead of throwing an error
    assert_eq!(json_float("1e9999"), Ok(("", Node::Float(f64::INFINITY))));

    // Although there are some literal floats that will return errors,
    // they are considered bugs so we shouldn't expect that behavior forever.
    // See https://github.com/rust-lang/rust/issues/31407
    // assert_eq!(
    //     json_float("2.47032822920623272e-324"),
    //     Err(nom::Err::Failure(JSONParseError::BadFloat))
    // );
}

#[test]
fn test_string() {
    // Plain Unicode strings with no escaping
    assert_eq!(json_string(r#""""#), Ok(("", Node::Str("".into()))));
    assert_eq!(json_string(r#""Hello""#), Ok(("", Node::Str("Hello".into()))));
    assert_eq!(json_string(r#""の""#), Ok(("", Node::Str("の".into()))));
    assert_eq!(json_string(r#""𝄞""#), Ok(("", Node::Str("𝄞".into()))));

    // valid 2-character escapes
    assert_eq!(json_string(r#""  \\  ""#), Ok(("", Node::Str("  \\  ".into()))));
    assert_eq!(json_string(r#""  \"  ""#), Ok(("", Node::Str("  \"  ".into()))));

    // valid 6-character escapes
    assert_eq!(json_string(r#""\u0000""#), Ok(("", Node::Str("\x00".into()))));
    assert_eq!(json_string(r#""\u00DF""#), Ok(("", Node::Str("ß".into()))));
    assert_eq!(json_string(r#""\uD834\uDD1E""#), Ok(("", Node::Str("𝄞".into()))));

    // Invalid because surrogate characters must come in pairs
    assert!(json_string(r#""\ud800""#).is_err());
    // Unknown 2-character escape
    assert!(json_string(r#""\x""#).is_err());
    // Not enough hex digits
    assert!(json_string(r#""\u""#).is_err());
    assert!(json_string(r#""\u001""#).is_err());
    // Naked control character
    assert!(json_string(r#""\x0a""#).is_err());
    // Not a JSON string because it's not wrapped in quotes
    assert!(json_string("abc").is_err());
    // An unterminated string (because the trailing quote is escaped)
    assert!(json_string(r#""\""#).is_err());

    // Parses correctly but has escape errors due to incomplete surrogate pair.
    assert_eq!(json_string(r#""\ud800""#), Err(nom::Err::Failure(JSONParseError::BadEscape)));
}

#[test]
fn test_array() {
    assert_eq!(json_array("[ ]"), Ok(("", Node::Array(vec![]))));
    assert_eq!(json_array("[ 1 ]"), Ok(("", Node::Array(vec![Node::Integer(1)]))));

    let expected = Node::Array(vec![Node::Integer(1), Node::Str("x".into())]);
    assert_eq!(json_array(r#" [ 1 , "x" ] "#), Ok(("", expected)));
}

#[test]
fn test_object() {
    assert_eq!(json_object("{ }"), Ok(("", Node::Object(vec![]))));
    let expected = Node::Object(vec![("1".into(), Node::Integer(2))]);
    assert_eq!(json_object(r#" { "1" : 2 } "#), Ok(("", expected)));
}

#[test]
fn test_values() {
    assert_eq!(json_value(" 56 "), Ok(("", Node::Integer(56))));
    assert_eq!(json_value(" 78.0 "), Ok(("", Node::Float(78.0))));
    // These two tests aren't relevant for JSON. They verify that `json_float`
    // will never mistake integers for floats in other grammars that might
    // allow a `.` or `e` character after a literal integer.
    assert_eq!(json_value("123else"), Ok(("else", Node::Integer(123))));
    assert_eq!(json_value("123.x"), Ok((".x", Node::Integer(123))));
    assert_eq!(json_value(r#" "Hello" "#), Ok(("", Node::Str("Hello".into()))));
}
