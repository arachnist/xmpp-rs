# Make a struct or enum parseable from XML

This derives the [`FromXml`] trait on a struct or enum. It is the counterpart
to [`macro@IntoXml`].

## Example

```rust
# use xso::FromXml;
static MY_NAMESPACE: &str = "urn:example";

#[derive(FromXml, Debug, PartialEq)]
#[xml(namespace = MY_NAMESPACE, name = "foo")]
struct Foo;

let foo: Foo = xso::from_bytes(b"<foo xmlns='urn:example'/>").unwrap();
assert_eq!(foo, Foo);
```

## Attributes

The derive macros need to know which XML namespace and name the elements it
is supposed have. This must be specified via key-value pairs on the type the
derive macro is invoked on. These are specified as Rust attributes. In order
to disambiguate between XML attributes and Rust attributes, we are going to
refer to Rust attributes using the term *meta* instead, which is consistent
with the Rust language reference calling that syntax construct *meta*.

All key-value pairs interpreted by these derive macros must be wrapped in a
`#[xml( ... )]` *meta*. The following keys are defined on structs:

| Key | Value type | Description |
| --- | --- | --- |
| `namespace` | *path* | The path to a `&'static str` which holds the XML namespace to match. |
| `name` | *string literal* | The XML element name to match. |

## Limitations

Supports only empty structs currently. For example, the following will not
work:

```compile_fail
# use xso::FromXml;
# static MY_NAMESPACE: &str = "urn:example";
#[derive(FromXml, Debug, PartialEq)]
#[xml(namespace = MY_NAMESPACE, name = "foo")]
struct Foo {
    some_field: String,
}
```
