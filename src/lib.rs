use nom::{branch::alt, bytes::complete::tag, IResult};
use nom::combinator::value;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum JsonBool {
    False,
    True,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct JsonNull {}

fn json_bool(input: &str) -> IResult<&str, JsonBool> {
    alt((
        value(JsonBool::False, tag("false")),
        value(JsonBool::True, tag("true")),
    ))
    (input)
}

fn json_null(input: &str) -> IResult<&str, JsonNull> {
    value(JsonNull {}, tag("null"))
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
