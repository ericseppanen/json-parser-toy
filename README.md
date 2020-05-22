# Let's build a parser!

This is a demonstration of building a parser in Rust using the [`nom`](https://docs.rs/nom/5.1.1/nom/) crate.  I recently built a parser for the [`cddl-cat`](https://docs.rs/cddl-cat/latest/cddl_cat/) crate using nom, and I found it a surprisingly not-terrible experience, much better than my past experiences with other parser-generators in other languages.

Since I like Rust a lot, and I need an excuse to do more writing about Rust, I thought I'd do another demonstration project.  I decided to choose a simple syntax, to keep this a short project. So I'm going to build a parser for JSON.

There are a million JSON parsers in the world already, so I don't expect this code to have much non-educational value.  But, hey, you never know.

## Part 1. Let's get started!

A few details, before I write the first lines of code:

1. I'm going to use [RFC8259](https://tools.ietf.org/html/rfc8259) as my authoritative reference for the JSON grammar.
2. I'm not going to build a JSON serializer.  My goal will only be to consume JSON text and output a structured tree containing the data (a lot like [`serde_json::Value`](https://docs.serde.rs/serde_json/value/enum.Value.html) ).
3. I'll be using [`nom` 5.1.1](https://docs.rs/nom/5.1.1/nom/).  There is a newer 6.0 alpha release out at the time of this writing, but I'm going to ignore it until it has a stable version number.
4. Some of the code I write will violate the usual `rustfmt` style.  This isn't because I hate `rustfmt`; far from it! But as you'll see, `nom` code can look a little weird, so it's sometimes more readable if we bend the styling rules a little bit.  Do what you like in your own code.

Let's start with a few words about `nom`.  It can take a little bit of time to adjust to writing a parser with `nom`, because it doesn't work by first tokenizing the input and then parsing those tokens.  You tackle both of those steps at once.

I'll be using only the `nom` functions, not the `nom` macros.  There are basically two implementations of everything in `nom` (a function and a macro), though the two are from different development eras and aren't exactly the same.  I'm inclined to always choose functions over macros when possible, because it's more likely to lead to a friendly programmer experience, so that's the way I'm going to go.  Note, however, that a lot of the `nom` documentation only refers to the older macro implementations of things, and so do many examples and tutorials.  So don't be surprised when you see macros everywhere.  Don't be afraid, either: the `nom` functions work great, are stable, and provide most of the things you need.

A bit of advice for reading the [`nom` documentation](https://docs.rs/crate/nom/5.1.1), if you're following along with this implementation:
- Start from the [modules](https://docs.rs/nom/5.1.1/nom/#modules) section of the documentation.
- We'll be starting with the [character](https://docs.rs/nom/5.1.1/nom/character/index.html) and [number](https://docs.rs/nom/5.1.1/nom/number/index.html) modules.
- We'll use the [combinator](https://docs.rs/nom/5.1.1/nom/combinator/index.html), [multi](https://docs.rs/nom/5.1.1/nom/multi/index.html), [sequence](https://docs.rs/nom/5.1.1/nom/sequence/index.html), and [branch](https://docs.rs/nom/5.1.1/nom/branch/index.html) modules to tie things together. I'll try to link to the relevant documentation as we go.

## Part 2. Now let's actually start.

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

I got the `tag` function from `nom::bytes`, though it's not specific to byte-arrays; it works just fine with text strings as well.  It's not a big deal; it's just a minor quirk of the way `nom` is organized.

We use `alt` to express "one of these choices".  This is a common style in `nom`, and we'll see it again when we use other combinators from `nom::sequence`.

There are a few other things that should be explained.

[`IResult`](https://docs.rs/nom/5.1.1/nom/type.IResult.html) is an important part of working with `nom`.  It's a specialized `Result`, where an `Ok` always returns a tuple of two values.  In this case, `IResult<&str, &str>` returns two string slices.  The first is the "remainder": this is everything that wasn't parsed.  The second part is the output from a successful parse; in this case we just return the string we matched.  For example, I could add this to my test, and it would work:

```rust
assert_eq!(json_bool("false more"), Ok((" more", "false")));
```

The `json_bool` function consumed the `false` part of the string, and left the rest for somebody else to deal with.

When json_bool returns an error, that doesn't necessarily mean that something is wrong.  Our parser isn't going to give up.  It just means that this particular bit of grammar didn't match.  Depending on how we write our code, other parser functions might be called instead.  You can actually see this in action if you look at how the `alt` combinator works.  It first calls a parser function `tag(false)`, and if that returns an error, it instead feeds the same input into `tag(true)`, to see if it might succeed instead.

This probably still looks kind of strange, because `tag(false)` isn't a complete parser function; it's a function that returns a parser function.  See how our code calls `alt` and `tag` (twice)?  The return value from that code is another function, and that function gets called with the argument `(input)`.

Don't be scared off by the intimidating-looking parameters of the `tag` function in the documentation-- look at the [examples](https://docs.rs/nom/5.1.1/nom/bytes/complete/fn.tag.html#example).  Despite the extra layer of indirection, it's still pretty easy to use.

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

To change the return value, we use `nom`'s `map` combinator function.  It allows us to apply a closure to convert the matched string into something else: in the `json_bool` case, one of the `JsonBool` variants.  You will probably smell something funny about that code, though: we already matched the "true" and "false" strings once in the parser generated by the `tag` function, so why are we doing it again?  Your instincts are right on-- we should probably back up and fix that, but let's wrap up this discussion first.

The json_null function does almost exactly the same thing, though it doesn't need a `match` because it could only have matched one thing.

We need to derive `PartialEq` and `Debug` for our structs and enums so that the `assert_eq!` will work.  Our tests are now using the new data structures `JsonBool` and `JsonNull`.

## Part 4. Another way of doing the same thing.

In `nom`, there are often multiple ways of achieving the same goal.  In our case, `map` is a little bit overkill for this use case.  Let's instead use the `value` combinator instead, which is specialized for the case where we only care that the child parser succeeded.

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

Note that I added `Clone` to the data structures, because `value` requires it.  I also added `Copy` because these are trivially small structs & enums, out of habit.

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

We'll need to do something when we encounter values that are grammatically correct (e.g. 1000 digits), that we can't handle.  This is a common problem, since most grammars don't attempt to set limits on the size of numbers.  Often there will be a limit set somewhere in the language/format specification, but it's not part of the formal grammar.  JSON doesn't set such limits, which can lead to compatibility problems between implementations.

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

Again, we use `alt` to specify that an integer is either `0`, or a non-`0` digit followed by zero or more additional digits.

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

The `opt` is another `nom` combinator; it means "optional", and unsurprisingly it will return an `Option<T>` where `T` in this case is `&str` (because that's what `tag("-")` will returns.  But that return type is ignored; `recognize` will throw it away and just give us back the characters that were consumed by the successful match.

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

Let's leave that for later, though.  For now we'll finish up this section with a few unit tests:

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

I'll quit with the suspense and show you the broken part:

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
