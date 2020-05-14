use nom::{branch::alt, bytes::complete::tag, IResult};

fn json_bool(input: &str) -> IResult<&str, &str> {
    alt((
        tag("false"),
        tag("true")
    ))
    (input)
}

#[test]
fn test_bool() {
    assert_eq!(json_bool("false"), Ok(("", "false")));
    assert_eq!(json_bool("true"), Ok(("", "true")));
    assert!(json_bool("foo").is_err());
}
