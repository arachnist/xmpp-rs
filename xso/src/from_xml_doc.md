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

The derive macros need additional information, such as XML namespaces and
names to match. This must be specified via key-value pairs on the type or
fields the derive macro is invoked on. These key-value pairs are specified as
Rust attributes. In order to disambiguate between XML attributes and Rust
attributes, we are going to refer to Rust attributes using the term *meta*
instead, which is consistent with the Rust language reference calling that
syntax construct *meta*.

All key-value pairs interpreted by these derive macros must be wrapped in a
`#[xml( ... )]` *meta*.

### Struct meta

The following keys are defined on structs:

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

### Field meta

For fields, the *meta* consists of a nested meta inside the `#[xml(..)]` meta,
the identifier of which controls *how* the field is mapped to XML, while the
contents control the parameters of that mapping.

The following mapping types are defined:

| Type | Description |
| --- | --- |
| [`attribute`](#attribute-meta) | Map the field to an XML attribute on the struct's element |

#### `attribute` meta

The `attribute` meta does not support additional parameters. The field it is
used on is mapped to an XML attribute of the same name and must be of type
[`String`].
