# Make a struct or enum parseable from XML

This derives the [`FromXml`] trait on a struct or enum. It is the counterpart
to [`macro@IntoXml`].

## Example

```rust
# use xso::FromXml;
#[derive(FromXml, Debug, PartialEq)]
#[xml(namespace = "urn:example", name = "foo")]
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
| `namespace` | *string literal* or *path* | The XML element namespace to match. If it is a *path*, it must point at a `&'static str`. |
| `name` | *string literal* or *path* | The XML element name to match. If it is a *path*, it must point at a `&'static NcNameStr`. |

Note that the `name` value must be a valid XML element name, without colons.
The namespace prefix, if any, is assigned automatically at serialisation time
and cannot be overridden. The following will thus not compile:

```compile_fail
# use xso::FromXml;
#[derive(FromXml, Debug, PartialEq)]
#[xml(namespace = "urn:example", name = "fnord:foo")]  // colon not allowed
struct Foo;
```

## Limitations

Supports only empty structs currently. For example, the following will not
work:

```compile_fail
# use xso::FromXml;
#[derive(FromXml, Debug, PartialEq)]
#[xml(namespace = "urn:example", name = "foo")]
struct Foo {
    some_field: String,
}
```
