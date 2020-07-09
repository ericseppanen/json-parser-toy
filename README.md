# Let's build a parser!

This is a demonstration of building a parser in Rust using the [`nom`](https://docs.rs/nom/5.1.2/nom/) crate.  I recently built a parser for the [`cddl-cat`](https://docs.rs/cddl-cat/latest/cddl_cat/) crate using nom, and I found it a surprisingly not-terrible experience, much better than my past experiences with other parser-generators in other languages.

Since I like Rust a lot, and I need an excuse to do more writing about Rust, I thought I'd do another demonstration project.  I decided to choose a simple syntax, to keep this a short project. So I'm going to build a parser for JSON.

<!-- more -->

There are a million JSON parsers in the world already, so I don't expect this code to have much non-educational value.  But, hey, you never know.

## Part 1. Introduction.

A few details, before I write the first lines of code:

1. I'm going to use [RFC8259](https://tools.ietf.org/html/rfc8259) as my authoritative reference for the JSON grammar.
2. I'm not going to build a JSON serializer.  My goal will only be to consume JSON text and output a structured tree containing the data (a lot like [`serde_json::Value`](https://docs.serde.rs/serde_json/value/enum.Value.html) ).
3. I'll be using [`nom` 5.1](https://docs.rs/nom/5.1.2/nom/).  There is a newer 6.0 alpha release out at the time of this writing, but I'm going to ignore it until it has a stable version number.
4. Some of the code I write will violate the usual `rustfmt` style.  This isn't because I hate `rustfmt`; far from it! But as you'll see, `nom` code can look a little weird, so it's sometimes more readable if we bend the styling rules a little bit.  Do what you like in your own code.
5. All of my source code will be [available on GitHub](https://github.com/ericseppanen/json-parser-toy). If you have comments or suggestions, or see a bug or something wrong in this post, please open an issue there.

Let's start with a few words about `nom`.  It can take a little bit of time to adjust to writing a parser with `nom`, because it doesn't work by first tokenizing the input and then parsing those tokens.  Both of those steps can be tackled at once.

I'll be using only the `nom` functions, not the `nom` macros.  There are basically two implementations of everything in `nom` (a function and a macro), though the two are from different development eras and aren't exactly the same.  I'm inclined to always choose functions over macros when possible, because it's more likely to lead to a friendly programmer experience.  Note, however, that a lot of the `nom` documentation only refers to the older macro implementations of things, and so do many examples and tutorials.  So don't be surprised when you see macros everywhere.  Don't be afraid, either: the `nom` functions work great, are stable, and provide most of the things you need.

A bit of advice for reading the [`nom` documentation](https://docs.rs/crate/nom/5.1.2), if you're following along with this implementation:
- Start from the [modules](https://docs.rs/nom/5.1.2/nom/#modules) section of the documentation.
- We'll be starting with the [character](https://docs.rs/nom/5.1.2/nom/character/index.html) and [number](https://docs.rs/nom/5.1.2/nom/number/index.html) modules.
- We'll use the [combinator](https://docs.rs/nom/5.1.2/nom/combinator/index.html), [multi](https://docs.rs/nom/5.1.2/nom/multi/index.html), [sequence](https://docs.rs/nom/5.1.2/nom/sequence/index.html), and [branch](https://docs.rs/nom/5.1.2/nom/branch/index.html) modules to tie things together. I'll try to link to the relevant documentation as we go.

## Part 2. Our first bit of parser code.

I've started a new library project (`cargo init --lib json-parser-toy`), and added the `nom 5.1` dependency in `Cargo.toml`.  Let's add a very simple parser function, just to verify that we can build and test our code.  We'll try to parse the strings "true" and "false".  In other words, the grammar for our json subset is:
```
value = "false" / "true"
```

Here's our first bit of code:
```rust
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
```

I got the [`tag`](https://docs.rs/nom/5.1.2/nom/bytes/complete/fn.tag.html) function from `nom::bytes`, though it's not specific to byte-arrays; it works just fine with text strings as well.  It's not a big deal; it's just a minor quirk of the way `nom` is organized.

We use [`alt`](https://docs.rs/nom/5.1.2/nom/branch/fn.alt.html) to express "one of these choices".  This is a common style in `nom`, and we'll see it again when we use other combinators from `nom::sequence`.

There are a few other things that should be explained.

[`IResult`](https://docs.rs/nom/5.1.2/nom/type.IResult.html) is an important part of working with `nom`.  It's a specialized `Result`, where an `Ok` always returns a tuple of two values.  In this case, `IResult<&str, &str>` returns two string slices.  The first is the "remainder": this is everything that wasn't parsed.  The second part is the output from a successful parse; in this case we just return the string we matched.  For example, I could add this to my test, and it would work:

```rust
assert_eq!(json_bool("false more"), Ok((" more", "false")));
```

The `json_bool` function consumed the `false` part of the string, and left the rest for somebody else to deal with.

When `json_bool` returns an error, that doesn't necessarily mean that something is wrong.  Our top-level parser isn't going to give up.  It just means that this particular bit of grammar didn't match.  Depending on how we write our code, other parser functions might be called instead.  You can actually see this in action if you look at how the `alt` combinator works.  It first calls a parser function `tag("false")`, and if that returns an error, it instead feeds the same input into `tag("true")`, to see if it might succeed instead.

This probably still looks kind of strange, because `tag("false")` isn't a complete parser function; it's a function that returns a parser function.  See how our code calls `alt` and `tag` (twice)?  The return value from that code is another function, and that function gets called with the argument `(input)`.

Don't be scared off by the intimidating-looking parameters of the `tag` function in the documentation-- look at the [examples](https://docs.rs/nom/5.1.2/nom/bytes/complete/fn.tag.html#example).  Despite the extra layer of indirection, it's still pretty easy to use.

## Part 3. Returning structs.

We don't want to just return the strings that we matched; we want to return some Rust structs that we can put into a tree form.

We could copy the previous function to add another simple JSON element:
```rust
fn json_null(input: &str) -> IResult<&str, &str> {
    tag("null")
    (input)
}
```

That would work, but let's rewrite our two parser functions to return enums or structs instead.
```rust
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

```

First, notice that the parser functions' return value has changed.  The first part of the `IResult` tuple is still the remainder, so it's still `&str`.  But the second part now returns one of our new data structures.

To change the return value, we use `nom`'s [`map`](https://docs.rs/nom/5.1.2/nom/combinator/fn.map.html) combinator function.  It allows us to apply a closure to convert the matched string into something else: in the `json_bool` case, one of the `JsonBool` variants.  You will probably smell something funny about that code, though: we already matched the `"true"` and `"false"` strings once in the parser generated by the `tag` function, so why are we doing it again?  Your instincts are right on-- we should probably back up and fix that, but let's wrap up this discussion first.

The `json_null` function does almost exactly the same thing, though it doesn't need a `match` because it could only have matched one thing.

We need to derive `PartialEq` and `Debug` for our structs and enums so that the `assert_eq!` will work.  Our tests are now using the new data structures `JsonBool` and `JsonNull`.

## Part 4. Another way of doing the same thing.

In `nom`, there are often multiple ways of achieving the same goal.  In our case, `map` is a little bit overkill for this use case.  Let's instead use the [`value`](https://docs.rs/nom/5.1.2/nom/combinator/fn.value.html) combinator instead, which is specialized for the case where we only care that the child parser succeeded.

We'll also refactor `json_bool` so that we don't need to do extra work: we'll apply our combinator a little earlier, before we lose track of which branch we're on.

```rust
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
```

Hopefully this is pretty straightforward.  The `value` combinator returns its first argument (e.g. `JsonNull {}`), if the second argument succeeds (`tag("null")`).  That description is a bit of a lazy mental shortcut, because `value` doesn't do any parsing itself.  Remember, it's a function that consumes one parser function and returns another parser function.  But because `nom` makes things so easy, it's sometimes a lot easier to use the lazy way of thinking when you're plugging combinators together like Lego bricks.

Note that I added `Clone` to the data structures, because `value` requires it.  I also added `Copy` because these are trivially small structs & enums.

## Part 5. Prepare to tree.

Our final output should be some tree-like data structure, similar to [`serde_json::Value`](https://docs.serde.rs/serde_json/value/enum.Value.html).  I'm partial to the word "node" to describe the parts of a tree, so let's start here:

```rust
pub enum Node {
    Null(JsonNull),
    Bool(JsonBool),
}
```

Right away, I don't like where this is going.  Here are all the things I'm unhappy with:

1. The redundant naming.  I have `Node::Null` and `JsonNull`, for a value that contains no additional data.
2. The null and bool types don't really seem like they need their own data structure name, outside of the tree node.  If this were a complex value type that I might want to pass around on its own, sure.  But for this simple case, I think this is a lot simpler:

```rust
#[derive(PartialEq, Debug, Clone)]
pub enum Node {
    Null,
    Bool(bool),
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
```

We got rid of JsonNull and JsonBool entirely.  For your parser you can choose any output structure that makes sense; different grammars have different properties, and they may not map easily onto Rust's prelude types.

## Part 6. Parsing numbers is hard.

The other remaining literal types in JSON are strings and numbers.  Let's tackle numbers first.  Referring to [RFC8259](https://tools.ietf.org/html/rfc8259), the grammar for a JSON number is:

```text
number = [ minus ] int [ frac ] [ exp ]

      decimal-point = %x2E       ; .
      digit1-9 = %x31-39         ; 1-9
      e = %x65 / %x45            ; e E
      exp = e [ minus / plus ] 1*DIGIT
      frac = decimal-point 1*DIGIT
      int = zero / ( digit1-9 *DIGIT )
      minus = %x2D               ; -
      plus = %x2B                ; +
      zero = %x30                ; 0
```

That grammar can represent any integer or floating point value; it would be grammatically correct to have an integer a thousand digits long, or a floating point value with huge exponent.  It's our decision how to handle these values.

JSON (like JavaScript) is a bit unusual in not distinguishing integers from floating-point values.  To make this tutorial a little more widely useful, let's output integers and floats as separate types:

```rust
pub enum Node {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
}
```

We'll need to do something when we encounter values that are grammatically correct (e.g. 1000 digits), that we can't handle.  This is a common problem, since most grammars don't attempt to set limits on the size of numbers.  Often there will be a limit set somewhere, but it's not part of the formal grammar.  JSON doesn't set such limits, which can lead to compatibility problems between implementations.

It will be important in most parsers to set limits and make sure things fail gracefully.  In Rust you're not likely to have problems with buffer overruns, but it might be possible to trigger a denial of service, or perhaps even a crash by triggering excessive recursion.

Let's start by making the parser functions we need, and we'll see where we need error handling.

Let's build a little helper function for the `digit1-9` part, since `nom` only offers `digit`, which includes `0-9`.

```rust
fn digit1to9(input: &str) -> IResult<&str, &str> {
    one_of("123456789")
    (input)
}
```

Unfortunately, it doesn't compile:
```text
error[E0308]: mismatched types
  --> src/lib.rs:21:5
   |
21 | /     one_of("123456789")
22 | |     (input)
   | |___________^ expected `&str`, found `char`
   |
   = note: expected enum `std::result::Result<(&str, &str), nom::internal::Err<(&str, nom::error::ErrorKind)>>`
              found enum `std::result::Result<(&str, char), nom::internal::Err<_>>`
```

This is a pretty easy mistake to make-- we tried to create a parser function that returns a string slice, but it's returning `char` instead, because, well, that's how `one_of` works.  It's not a big problem for us; just fix the return type to match:

```rust
fn digit1to9(input: &str) -> IResult<&str, char> {
    one_of("123456789")
    (input)
}
```

We can now build the next function, one that recognizes integers:
```rust
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
```

Again, we use `alt` to specify that an integer is either `0`, or a nonzero digit, possibly followed by more additional digits.

The new combinator here is `recognize`.  Let's back up and look at the return type of this hypothetical function:

```rust
fn nonzero_integer(input: &str) -> IResult<&str, ____> {
    pair(
        digit1to9,
        digit0
    )
    (input)
}
```

Because we used `pair`, the return type would be a 2-tuple.  The first element would be a `char` (because that's what we returned from `digit1to9`), and the other element would be a `&str`.  So the blank above would be filled in like this:
```rust
fn nonzero_integer(input: &str) -> IResult<&str, (char, &str)> {
    ...
}
```

In this context, not very helpful.  What we'd like to say is, "match this bunch of stuff, but just return the string slice that covers what we matched."  That's exactly what `recognize` does.

Because we're going to store integers in a different `Node` variant, we should also do one last call to `map`.  But that might make life difficult if we want to re-use this code as part of a token that's representing a floating-point number.

So let's leave the `uint` function alone; we'll use it as a building block of another function.
Note also that we can't finish parsing an integer until we've consumed the optional leading "minus" symbol.

```rust
fn json_integer(input: &str) -> IResult<&str, &str> {
    recognize(
        pair(
            opt(tag("-")),
            uint
        )
    )
    (input)
}
```

The `opt` function is another `nom` combinator; it means "optional", and unsurprisingly it will return an `Option<T>` where `T` in this case is `&str` (because that's what `tag("-")` will returns.  But that return type is ignored; `recognize` will throw it away and just give us back the characters that were consumed by the successful match.

Let's add one more step to our function: convert the resulting string into a `Node::Integer`.

```rust
fn json_integer(input: &str) -> IResult<&str, Node> {
    let parser = recognize(
        pair(
            opt(tag("-")),
            uint
        )
    );
    map(parser, |s| {
        let n = s.parse::<i64>().unwrap();
        Node::Integer(n)
    })
    (input)
}
```

Finally, we discover a point where we'll need some error handling.  [`str::parse`](https://doc.rust-lang.org/std/primitive.str.html#method.parse) returns a `Result`, and will certainly return `Err` if we try to parse something too big.

I am going to leave proper error handling until the end, so for now I will just `unwrap` the result.  This means the parser will panic if we give it a huge integer, so we definitely need to come back and fix this later.

For now we'll finish up this section with a few unit tests:

```rust
#[test]
fn test_integer() {
    assert_eq!(json_integer("42"), Ok(("", Node::Integer(42))));
    assert_eq!(json_integer("-123"), Ok(("", Node::Integer(-123))));
    assert_eq!(json_integer("0"), Ok(("", Node::Integer(0))));
    assert_eq!(json_integer("01"), Ok(("1", Node::Integer(0))));
}
```

Note the fourth test case-- this might not be what you expected.  We know that integers with a leading zero aren't allowed by this grammar-- so why did the call to `json_integer` succeed?  It has to do with the way `nom` operates-- each parser only consumes the part of the string it matches, and leaves the rest for some other parser.  So attempting to parse `01` results in a success, returning a result `Node::Integer(0)` along with a remainder string `1`.

`nom` does have ways for parsers to trigger a fatal error if they're unhappy with the sequence of characters, but this grammar probably won't need them.

## Part 7. Parsing numbers some more.

Let's piece together the bits we need to parse floating point numbers.

```rust
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
```

The only new parts here are:
- `nom::character::complete::digit1`: just like `digit0`, except this matches one-or-more digits.
- `nom::sequence::tuple` is a lot like `pair`, but accepts an arbitrary number of other parsers. Each sub-parser must match in sequence, and the return value is a tuple of results.

I added some straightforward unit tests here, and they all pass.  Despite that, I've made a significant mistake, but one that we won't notice until we start stitching the various parts together.  Let's do that now.

When a parser executes, it obviously won't know which elements are arriving in which order, so we need a parser function to handle everything we've built so far.  Thanks to the magic of `nom`, this part is really easy.

```rust
fn json_literal(input: &str) -> IResult<&str, Node> {
    alt((
        json_integer,
        json_float,
        json_bool,
        json_null
    ))
    (input)
}
```

And now we discover that something is wrong:

```rust
#[test]
fn test_literal() {
    assert_eq!(json_literal("56"), Ok(("", Node::Integer(56))));
    assert_eq!(json_literal("78.0"), Ok(("", Node::Float(78.0))));
}
```

```text
test test_literal ... FAILED

failures:

---- test_literal stdout ----
thread 'test_literal' panicked at 'assertion failed: `(left == right)`
  left: `Ok((".0", Integer(78)))`,
 right: `Ok(("", Float(78.0)))`', src/lib.rs:163:5
```

Because we put `json_integer` first, it grabbed the `78` part and declared success, leaving `.0` for someone else to deal with.  Not so big a deal, right?  Let's just swap the order of the parsers:

```rust
fn json_literal(input: &str) -> IResult<&str, Node> {
    alt((
        json_float,
        json_integer,
        json_bool,
        json_null
    ))
    (input)
}
```

```text
test test_literal ... FAILED

failures:

---- test_literal stdout ----
thread 'test_literal' panicked at 'assertion failed: `(left == right)`
  left: `Ok(("", Float(56.0)))`,
 right: `Ok(("", Integer(56)))`', src/lib.rs:162:5
```

We've traded one problem for another.  This time, `json_float` runs first, consumes the input `56` input and declares success, returning `Float(56.0)`.  This isn't wrong, exactly.  Had we decided at the beginning to treat all numbers as floating-point (as JavaScript does) this would be the expected outcome.  But since we committed to storing integers and floats as separate tree nodes, we have a problem.

Since we can't allow either the `json_float` parser or the `json_integer` parser to run first (at least as currently written), let's imagine what we'd like to see happen.  Ideally, we would start parsing the `[ minus ] int` part of the grammar, and if that succeeds we have a possible integer-or-float match.  We should then continue on, trying to match the `[ frac ] [ exp ]` part, and if _either of those_ succeeds, we have a float.

There are a few different ways to implement that logic.

One way would be to get `json_float` to fail if the next character after the integer part is _not_ a `.` or `e` character-- without that it can't possibly be a valid float (according to our grammar), so if `json_float` fails at that point we know the `json_integer` parser will run next (and succeed).

```rust
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
        let n = s.parse::<f64>().unwrap();
        Node::Float(n)
    })
    (input)
}
```

This code has one small annoyance, though it's not a problem in the overall JSON context.  Imagine that we took this `json_float` parser code, and tried to reuse it in another language, where this other language's grammer would allow the input `123.size()`.  This code would `peek` ahead and see the `.` character, and because of that it would parse `123` as a float rather than an integer.  In other words, this `json_float` implementation decides that this input is a float before it's actually finished parsing all the characters making up that float.

There is a slightly better way, though. Remember, our original problem is that `json_float` will succeed in all of the following cases:
- `123`
- `123.0`
- `123e9`
- `123.0e9`
What we'd rather have is a parser that succeeds at the last three, but not the first.  There isn't a combinator in `nom` that implements "A or B or AB", but it's not that hard to implement ourselves:

```rust
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
        let n = s.parse::<f64>().unwrap();
        Node::Float(n)
    })
    (input)
}
```

This new logic uses `alt` to allow two choices: either a `frac` must be present (with an optional `exp`) following, or an `exp` must be present by itself.  An input with neither a valid `frac` or `exp` will now fail, which makes everything work the way we want it to.

## Part 8. Handling string literals

So far we support literal null, boolean, integer, and float types.  There's only one more literal type left to handle: strings.

In the JSON grammar, a string is basically a series of Unicode characters that starts and ends with a quote, plus a few extra rules:

1. Certain characters must be escaped (ASCII control characters, quotes, and backslashes)
2. Any character may be escaped, using `\u` plus 4 hexadecimal digits, e.g. `\uF903`.
3. A small number of common characters have two-character escapes: `\"` `\\` `\/` `\b` `\f` `\n` `\r` `\t`.

That's how RFC 8259 does things, anyway.  Different implementations may have subtle differences.

This means there are many possible ways to represent a certain string. We're only building a parser, so we just need to make sure we can parse all the valid JSON representations (and hopefully return an error on all the invalid ones).

The presence of escape characters makes our job more difficult.  There are different ways we might choose to address this.  I'm going to choose to break escape handling into a separate phase.  This means we will only use `nom` to do the lexing part (finding the bounds of the string literal), and we'll follow up with an "un-escaping" pass to decode the escaped characters.

Bad inputs must be rejected by one of the two phases, but we don't care which one.  For example, `"\ud800"` looks like a valid JSON string, but can't be decoded because U+D800 is a magic "surrogate" character, meaning it's half of a character that needs more than 16 bits to encode.  We should also reject things like `"\x"` (a nonexistent escape), `"\u001"` (not enough hex digits), and `"\"` (which is unterminated because the trailing quote is escaped).  We also need to reject "naked" (non-escaped) control characters (ASCII 0x00-0x1F), though for some reason 0x7F (ASCII DELETE) is legal.

Let's begin by building a parser for "a string of valid non-escaped characters": everything except control characters, backslash, and quote. We don't need to check the upper limit 0x10FFFF because those characters will never appear in a Rust `char`.

```rust
use nom::bytes::complete::take_while1;

fn is_nonescaped_string_char(c: char) -> bool {
    let cv = c as u32;
    (cv >= 0x20) && (cv != 0x22) && (cv != 0x5C)
}

// One or more unescaped text characters
fn nonescaped_string(input: &str) -> IResult<&str, &str> {
    take_while1(is_nonescaped_string_char)
    (input)
}
```

The `take_while1` function comes from the nom `bytes` module (which, remember, isn't specific to byte sequences).  `nom` offers a few different `take` functions in this module; `take_while1` consumes characters that match some condition, requiring at least 1 matching character.

Next, let's add a parser that can detect one escape sequence.  Actually, we're going to be even lazier than that; we'll pretend that `\u` is an escape sequence all by itself, and let the unescape function determine whether the characters that follow make sense.  We could easily do it differently, but since the unescape code will need to look at those characters in detail later, we won't waste time doing that work twice.

```rust
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
```

Using those two pieces, we can now connect them together to parse the entire body of a JSON string (minus the quotes that surround it):

```rust
use nom::multi::many0;

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
```

We've seen most of the pieces here before.

`many0` tries to apply a parser function repeatedly, gathering all of the results into a vector.  This version gathers "zero or more" of whatever we were searching for (which is desirable because `""` is a valid JSON string).  There is also a `many1`, (if you want "one or more") and several other variations.

The final `recognize` throws away the output of `many0` (a vector), and instead just returns to us the string that was matched. It's a little unfortunate that we're throwing away the information we developed about where escapes appear-- perhaps another implementation could do the unescaping work right here.  It seems pretty typical (in my limited experience) to have to make tradeoffs like this.  We're breaking the work into multiple phases, which may require a little bit of redundant effort, but our code gets a little simpler as a result.

There's one subtle thing about these two layers that should be pointed out.  Both `nonescaped_string` and `escape_code` are parsers that return "one or more characters".  And then we use those to build a parser that returns "zero or more characters".  In fact, you can't build a "zero or more" parser using other "zero or more" components, because that could trigger an infinite loop: the outer parser could try to gather an infinite number of empty subparser successes.  Typically `nom` combinators will return an error instead of going into an infinite loop.

The next step is pretty simple: the string body must be wrapped in quotes.

```rust
use nom::sequence::delimited;

fn json_string(input: &str) -> IResult<&str, &str> {
    delimited(
        tag("\""),
        string_body,
        tag("\"")
    )
    (input)
}
```

This is the first time we've used `delimited`.  It runs three sub-parsers, returning the result of the middle one. The result from the first and third arguments (the quote characters) are discarded.

At this point I should plug in some code to do un-escaping. Because this code doesn't use `nom` and doesn't really help us understand how to write a `nom` parser, I'm going to skip the explanation and just pull the [escape8259](https://docs.rs/escape8259/0.5.0/escape8259/) crate that does this part.  A call to un-escape a string is pretty simple:

```rust
pub fn unescape(s: &str) -> Result<String, UnescapeError>
```

So all we need to do is plug that into `json_string`. We earlier used `nom`'s `map` combinator to do this sort of thing, but here we need something a little different because `unescape` may fail. We need to use `map_res` to handle `Result::Err`.

```rust
use nom::combinator::map_res;
use escape8259::unescape;

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
```

We also need to update our `Node` enum to include a string variant (we'll call this `Str`), and make that our final output.

```rust
pub enum Node {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Str(String),
}

fn json_string(input: &str) -> IResult<&str, Node> {
    map(string_literal, |s| {
        Node::Str(s)
    })
    (input)
}
```

Finally, we should write some tests to make sure this is working correctly.

```rust
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
}
```

## Part 9. Arrays and Objects

Finally, all of the hard parts are complete, and we get to the fun parts: arrays and objects (what we'd call a map or a dictionary in most other contexts).

Let's start with the changes to our `Node` enum, to give us a little better idea how these recursive data structures should work.

```rust
pub enum Node {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Str(String),
    Array(Vec<Node>),
    Object(Vec<(String, Node)>),
}
```

An array can be heterogenous (different value types).  Each object member must have a string for its key, and may have any value type.

Let's implement arrays first.

```rust
use nom::multi::separated_list;

fn json_array(input: &str) -> IResult<&str, Node> {
    let parser = delimited(
        tag("["),
        separated_list(tag(","), json_value),
        tag("]")
    );
    map(parser, |v| {
        Node::Array(v)
    })
    (input)
}
```

That was surprisingly easy.  The only new thing we needed was `separated_list`, which alternates between two subparsers.  The first argument is the "separator", and its result is thrown away; we get a vector of results from the second parser.  It will match zero or more elements; `nom` has a `separated_nonempty_list` if you want one-or-more.

Objects are up next; they're a little more complicated so let's implement them as two separate functions.

```rust
use nom::sequence::separated_pair;

fn object_member(input: &str) -> IResult<&str, (String, Node)> {
    separated_pair(string_literal, tag(":"), json_value)
    (input)
}

fn json_object(input: &str) -> IResult<&str, Node> {
    let parser = delimited(
        tag("{"),
        separated_list(
            tag(","),
            object_member
        ),
        tag("}")
    );
    map(parser, |v| {
        Node::Object(v)
    })
    (input)
}
```

This looks a lot like the array implementation.  The only difference (other than the braces) is that where an array looks for a single value, the object looks for a quoted string literal, then a `:` character, and then a value.

And we have a JSON parser!

## Part 10. Spacing out

Well, we almost have a JSON parser.  We might start testing arrays like this:

```rust
#[test]
fn test_array() {
    assert_eq!(json_array("[]"), Ok(("", Node::Array(vec![]))));
    assert_eq!(json_array("[1]"), Ok(("", Node::Array(vec![Node::Integer(1)]))));

    let expected = Node::Array(vec![Node::Integer(1), Node::Integer(2)]);
    assert_eq!(json_array("[1,2]"), Ok(("", expected)));
}
```

But it doesn't work if we write:

```rust
    assert_eq!(json_array("[1, 2]"), Ok(("", expected)));
```

The only difference is the space character after the comma.  We forgot to handle whitespace.

In fact, we haven't handled whitespace anywhere.  Whitespace could appear anywhere: before or after values or any punctuation (braces, brackets, comma, or colon).

To ignore whitespace, we need a parser function that matches whitespace.  We could easily build one, `nom` includes one that matches our needs exactly: `nom::character::complete::multispace0`.

That means we need to do a bunch of substitutions, things like:
```rust
  tag("[")
```
need to become
```rust
  delimited(multispace0, tag("["), multispace0)
```

Which adds a lot of clutter, and is kind of hard to read.  Maybe instead we should write a combinator of our own to make this a little more compact.  This isn't necessary-- the result will emit exactly the same code as the above.  The only reason I'm going to tackle this is it provides a little bit of insight into the pile of generic parameters you see if you look at the documentation for `nom` combinators.  If you don't care, feel free to skip this section.

First, let's write a combinator that does nothing, other than apply a parser we specify.
```rust
fn identity<F, I, O, E>(f: F) -> impl Fn(I) -> IResult<I, O, E>
where
    F: Fn(I) -> IResult<I, O, E>,
{
    f
}
```

That looks pretty intimidating.  But so do most of the built-in `nom` combinators, so if we can understand this combinator function, we'll have a little easier time understanding other parts of `nom`.

Let's see if we can make some sense of all those generic parameters.

`F` is the type of the parser we pass in.  It could be any `nom`-style parser, and we already know what those look like; they accept one input parameter, and return an `IResult`.  This `IResult` has three generic parameters, and we've always used two-- the third has a default value, and we've been omitting it.

So our `F` is a function that accepts one `I` and returns `IResult<I, O, E>`.  `I` is our input parameter (which has been `&str` so far everywhere). `O` is our output type (and we've used a bunch of different ones; `&str`, `Node`, etc.)  The `E` is the parser error type, and we can continue ignoring that for now since we've only used the default.

Our combinator returns a closure.  So its return type is `Fn(I) -> IResult<I, O, E>`.  That looks the same as `F`, but for all cases other than `identity` we'll return a different closure than the input, so we will need to spell out the return type.

A lot of `nom` combinators have even more complex type signatures (`separated_pair` has 8 generic parameters!) but picking them apart is usually pretty straightforward if you're patient.  You'll probably only need to know when something fails to compile.

Anyway, let's write a combinator that wraps its input in a `delimited` with `multispace0` on both sides.

```rust
fn spacey<F, I, O, E>(f: F) -> impl Fn(I) -> IResult<I, O, E>
where
    F: Fn(I) -> IResult<I, O, E>,
{
    delimited(multispace0, f, multispace0)
}
```

This explodes with a huge pile of errors; many complaints about trait bounds that aren't met for `I` and `E`.  But it turns out that this is just because `multispace0` requires those on its `I` and `E`, so we have to guarantee those trait bounds as well.  Copying those trait bounds over to our function will work:

```rust
fn spacey<F, I, O, E>(f: F) -> impl Fn(I) -> IResult<I, O, E>
where
    F: Fn(I) -> IResult<I, O, E>,
    I: nom::InputTakeAtPosition,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
    E: nom::error::ParseError<I>,
{
    delimited(multispace0, f, multispace0)
}
```

Was that worth it?  Maybe not for this program.  But it's interesting to see what's involved in building our own combinators.  Maybe the `nom` function documentation will look a little less scary, too.

Now that we have a useful multispace-handling combinator, we can sprinkle it around all the places where we need to ignore whitespace.  For example:

```rust
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
```

## Part 11. Error handling.

We skipped over a few places where proper error handling is needed. For example, numbers that are out of bounds (e.g. `1e99999`) should return some kind of parse error.

Currently we are using the `IResult` default error type, which is `nom::internal::Err<(&str, nom::error::ErrorKind)>`. That doesn't look promising-- we can't realistically expect to be able to extend that type with our own error variants.

So let's build our own error type. We'll use macros from the [`thiserror`](https://docs.rs/thiserror/1.0/thiserror/) crate to automatically generate some of the boilerplate that's necessary for error types.

```rust
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
```

Because `nom` error handling uses generic parameters, it can be difficult to see how to best implement a custom error type. There is a good minimal example of custom error types in the nom 6.0 sources ([examples/custom_error.rs](https://github.com/Geal/nom/blob/e766634631828462e8a9210a0cf88313bb79f318/examples/custom_error.rs)) that shows the steps needed to make things work gracefully:

1. Figure out how to map a `nom` error into your error type. Usually this will be with a dedicated enum variant.
2. Implement the trait `nom::error::ParseError<I>` for your error type. This will allow all of the `nom` combinators to generate your custom error type when needed.
3. Use the 3-argument form of `IResult`, specifying your error type. You will probably want to do this on most or all of your parser functions so combinators work gracefully.

When building a custom error type that will be generated by nom parsers, it's probably a good idea to consider whether this will be an internal-only error, or whether it will be a public part of your crate. If it's internal, it can be useful to preserve all the nom metadata (the `input` and `kind` parameters to `ParseError::from_error_kind`) for debugging. In a public error struct, it may be wiser to discard that information, as a user of your crate probably doesn't care about `nom` error metadata. I will assume `JSONParseError` is public, so I will discard the `nom` error parameters.

```rust
use nom::error::{ErrorKind, ParseError};

impl<I> ParseError<I> for JSONParseError {
    fn from_error_kind(_input: I, _kind: ErrorKind) -> Self {
        JSONParseError::Unparseable
    }

    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }
}
```

For error handling on integers, we'll split the function into two parts to make it easier to read:

```rust
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
```

Note that `json_integer` works differently from all the other parsers we've written so far: instead of composing parsers using combinators, we actually run the `integer_body` parser and capture its result (the remainder and the matched string slice). We then attempt to parse the string slice into an integer, and hand-assemble an `IResult` by hand.

This can be a useful technique when the `nom` combinators don't supply exactly what you need. Here, I first tried using `map_res` to parse the int, but it turns out that `map_res` always throws away the error value returned by the closure, and substitutes its own error (with kind `MapRes`).

The same approach works for string escaping errors and float parsing errors, though float overflow in Rust results in infinity, not an error. It's fairly hard to trigger a float parse error (though it's possible, due to a [bug](https://github.com/rust-lang/rust/issues/31407) in the rust core library).

## Part 12. Finalization.

There's one more `nom`-specific step that we probably want. Assuming our code is a library, meant to be used by other programs, we don't want `nom::IResult` to show up as our result. Instead, we've prefer a plain `Result<Node, JSONParseError>`.

We can use `all_consuming` to ensure that all input was matched.  Unfortunately, there doesn't seem to be a simple `nom` shortcut for translating the error. We can do this ourselves:

```rust

use nom::combinator::all_consuming;

pub fn parse_json(input: &str) -> Result<Node, JSONParseError> {
    let (_, result) = all_consuming(json_value)(input).map_err(|nom_err| {
        match nom_err {
            nom::Err::Incomplete(_) => unreachable!(),
            nom::Err::Error(e) => e,
            nom::Err::Failure(e) => e,
        }
    })?;
    Ok(result)
}
```

We haven't talked yet about the three [`nom::Err`](https://docs.rs/nom/5.1/nom/enum.Err.html) variants.

- `Incomplete` is only used by `nom` streaming parsers. We don't use those, so we can just mark that branch `unreachable!` (which would panic).
- `Error` is what we usually see when a parser has a problem.  Something didn't match the expected grammar.
- `Failure` appears less often.  It means that the input could only be parsed one way, but a parser decided that it was invalid. Unlike `Error`, this error is propagated upward without trying any alternative paths (if something like `alt` is present).

You may have noticed that our code does use `Failure`: that's what we return when there is an numeric conversion error or a bad escape code. If we accidentally used `Error` instead of `Failure`, the parsers might work correctly, but we would return the wrong error type.  The reason is that the nom `alt` parser would keep trying other parsers, and if all of them fail, there's no way for `alt` to know which error is the right one-- it usually just returns the last error.
