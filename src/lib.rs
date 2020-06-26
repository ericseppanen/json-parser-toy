use nom::{branch::alt, IResult};
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{one_of, digit0, digit1, multispace0};
use nom::combinator::{map, map_res, opt, recognize, value};
use nom::multi::{many0, separated_list};
use nom::sequence::{delimited, pair, separated_pair, tuple};
use escape8259::unescape;


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

fn json_value(input: &str) -> IResult<&str, Node> {
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

fn json_array(input: &str) -> IResult<&str, Node> {
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
fn object_member(input: &str) -> IResult<&str, (String, Node)> {
    separated_pair(string_literal, spacey(tag(":")), json_value)
    (input)
}

fn json_object(input: &str) -> IResult<&str, Node> {
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
fn nonescaped_string(input: &str) -> IResult<&str, &str> {
    take_while1(is_nonescaped_string_char)
    (input)
}

// There are only two types of escape allowed by RFC 8259.
// - single-character escapes \" \\ \/ \b \f \n \r \t
// - general-purpose \uXXXX
// Note: we don't enforce that escape codes are valid here.
// There must be a decoder later on.
fn escape_code(input: &str) -> IResult<&str, &str> {
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
fn string_body(input: &str) -> IResult<&str, &str> {
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

fn string_literal(input: &str) -> IResult<&str, String> {
    let parser = delimited(
        tag("\""),
        string_body,
        tag("\"")
    );
    map_res(parser, |s| {
        unescape(s)
    })
    (input)
}

fn json_string(input: &str) -> IResult<&str, Node> {
    map(string_literal, |s| {
        Node::Str(s)
    })
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
            alt((
                recognize(pair(
                    frac,
                    opt(exp)
                )),
                exp
            )),
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
    // FIXME: test too-large floats once error handling is in place.
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
