# Let's build a parser!

This is a demonstration of building a parser in Rust using the [`nom`](https://docs.rs/nom/5.1.1/nom/) crate.  I recently built a parser for the [`cddl-cat`](https://docs.rs/cddl-cat/latest/cddl_cat/) crate using nom, and I found it a surprisingly not-terrible experience, much better than my past experiences with other parser-generators in other languages.

Since I like Rust a lot, and I need an excuse to do more writing about Rust, I thought I'd do another demonstration project.  I decided to choose a simple syntax, to keep this a short project. So I'm going to build a parser for JSON.

There are a million JSON parsers in the world already, so I don't expect this code to have much non-educational value.  But, hey, you never know.

## Let's get started!

A few details, before I write the first lines of code:

1. I'm going to use [RFC8259](https://tools.ietf.org/html/rfc8259) as my authoritative reference for the JSON grammar.
2. I'm not going to build a JSON serializer.  My goal will only be to consume JSON text and output a structured tree containing the data (a lot like `serde_json::Value`).
3. I'll be using [`nom` 5.1.1](https://docs.rs/nom/5.1.1/nom/).  There is a newer 6.0 alpha release out at the time of this writing, but I'm going to ignore it until it has a stable version number.
4. Some of the code I write will violate the usual `rustfmt` style.  This isn't because I hate `rustfmt`; far from it! But as you'll see, `nom` code can look a little weird, so it's sometimes more readable if we bend the styling rules a little bit.  Do what you like in your own code.

Let's start with a few words about `nom`.  It can take a little bit of time to adjust to writing a parser with `nom`, because it doesn't work by first tokenizing the input and then parsing those tokens.  You tackle both of those steps at once.

I'll be using only the `nom` functions, not the `nom` macros.  There are basically two implementations of everything in `nom` (a function and a macro), though the two are from different development eras and aren't exactly the same.  I'm inclined to always choose functions over macros when possible, because it's more likely to lead to a friendly programmer experience, so that's the way I'm going to go.  Note, however, that a lot of the `nom` documentation only refers to the older macro implementations of things, and so do many examples and tutorials.  So don't be surprised when you see macros everywhere.  Don't be afraid, either: the `nom` functions work great, are stable, and provide most of the things you need.

A bit of advice for reading the [`nom` documentation](https://docs.rs/crate/nom/5.1.1), if you're following along with this implementation:
- Start from the [modules](https://docs.rs/nom/5.1.1/nom/#modules) section of the documentation.
- We'll be starting with the [character](https://docs.rs/nom/5.1.1/nom/character/index.html) and [number](https://docs.rs/nom/5.1.1/nom/number/index.html) modules.
- We'll use the [combinator](https://docs.rs/nom/5.1.1/nom/combinator/index.html), [multi](https://docs.rs/nom/5.1.1/nom/multi/index.html), [sequence](https://docs.rs/nom/5.1.1/nom/sequence/index.html), and [branch](https://docs.rs/nom/5.1.1/nom/branch/index.html) modules to tie things together. I'll try to link to the relevant documentation as we go.

## Now let's actually start.

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

Already, I realize that I told a lie.  I got the `tag` function from `nom::bytes`, though it's not specific to byte-arrays; it works just fine with text strings as well.  It's not a big deal; it's just a minor quirk of the way `nom` is organized.

There are a few other things that should be explained.

[`IResult`](https://docs.rs/nom/5.1.1/nom/type.IResult.html) is an important part of working with `nom`.  It's a specialized `Result`, where an `Ok` always returns a tuple of two values.  In this case, `IResult<&str, &str>` returns two string slices.  The first is the "remainder": this is everything that wasn't parsed.  The second part is the output from a successful parse; in this case we just return the string we matched.  For example, I could add this to my test, and it would work:

```rust
assert_eq!(json_bool("false more"), Ok((" more", "false")));
```

The `json_bool` function consumed the `false` part of the string, and left the rest for somebody else to deal with.

When json_bool returns an error, that doesn't necessarily mean that something is wrong.  Our parser isn't going to give up.  It just means that this particular bit of grammar didn't match.  Depending on how we write our code, other parser functions might be called instead.  You can actually see this in action if you look at how the `alt` combinator works.  It first calls a parser function `tag(false)`, and if that returns an error, it instead feeds the same input into `tag(true)`, to see if it might succeed instead.

This probably still looks kind of strange, because `tag(false)` isn't a complete parser function; it's a function that returns a parser function.  See how our code calls `alt` and `tag` (twice)?  The return value from that code is another function, and that function gets called with the argument `(input)`.

Don't be scared off by the intimidating-looking parameters of the `tag` function in the documentation-- look at the [examples](https://docs.rs/nom/5.1.1/nom/bytes/complete/fn.tag.html#example).  Despite the extra layer of indirection, it's still pretty easy to use.
