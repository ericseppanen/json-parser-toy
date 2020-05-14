use nom::{branch::alt, bytes::complete::tag, IResult};
use nom::combinator::map;

#[derive(PartialEq, Debug)]
pub enum JsonBool {
    False,
    True,
}

#[derive(PartialEq, Debug)]
pub struct JsonNull {}

fn json_bool(input: &str) -> IResult<&str, JsonBool> {
    let parser = alt((
        tag("false"),
        tag("true")
    ));
    map(parser, |s| {
        match s {
            "false" => JsonBool::False,
            "true" => JsonBool::True,
            _ => unreachable!(),
        }
    })
    (input)
}

fn json_null(input: &str) -> IResult<&str, JsonNull> {
    map(tag("null"), |_| JsonNull {})
    (input)
}

#[test]
fn test_bool() {
    assert_eq!(json_bool("false"), Ok(("", JsonBool::False)));
    assert_eq!(json_bool("true"), Ok(("", JsonBool::True)));
    assert!(json_bool("foo").is_err());
}

#[test]
fn test_null() {
    assert_eq!(json_null("null"), Ok(("", JsonNull {})));
}
