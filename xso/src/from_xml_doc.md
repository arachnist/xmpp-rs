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

The `attribute` meta causes the field to be mapped to an XML attribute of the
same name. The field must be of type [`String`].

The following keys can be used inside the `#[xml(attribute(..))]` meta:

| Key | Value type | Description |
| --- | --- | --- |
| `namespace` | *string literal* or *path* | The optional namespace of the XML attribute to match. If it is a *path*, it must point at a `&'static str`. Note that attributes, unlike elements, are unnamespaced by default. |
| `name` | *string literal* or *path* | The name of the XML attribute to match. If it is a *path*, it must point at a `&'static NcNameStr`. |

The `attribute` meta also supports a shorthand syntax,
`#[xml(attribute = ..)]`, where the value is treated as the value for the
`name` key and the `namespace` is unset.

##### Example

```rust
# use xso::FromXml;
#[derive(FromXml, Debug, PartialEq)]
#[xml(namespace = "urn:example", name = "foo")]
struct Foo {
    #[xml(attribute)]
    a: String,
    #[xml(attribute = "bar")]
    b: String,
    #[xml(attribute(name = "baz"))]
    c: String,
    #[xml(attribute(namespace = "urn:example", name = "fnord"))]
    d: String,
};

let foo: Foo = xso::from_bytes(b"<foo
    xmlns='urn:example'
    a='1' bar='2' baz='3'
    xmlns:tns0='urn:example' tns0:fnord='4'
/>").unwrap();
assert_eq!(foo, Foo {
    a: "1".to_string(),
    b: "2".to_string(),
    c: "3".to_string(),
    d: "4".to_string(),
});
```
