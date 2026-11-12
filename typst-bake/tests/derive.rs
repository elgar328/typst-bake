//! The `IntoValue` / `IntoDict` derives, used from outside the crate like users do.

use typst_bake::__internal::typst::foundations::{Dict, IntoValue, Value};
use typst_bake::{IntoDict, IntoValue};

fn keys(dict: &Dict) -> Vec<&str> {
    dict.iter().map(|(key, _)| key.as_str()).collect()
}

#[test]
fn raw_identifiers_lose_their_prefix() {
    #[derive(IntoValue, IntoDict)]
    struct Inputs {
        r#type: String,
        name: String,
    }

    let dict: Dict = Inputs {
        r#type: "invoice".into(),
        name: "x".into(),
    }
    .into();
    assert_eq!(keys(&dict), ["type", "name"]);
}

#[test]
fn generic_structs_derive() {
    #[derive(IntoValue, IntoDict)]
    struct Wrapper<T> {
        value: T,
    }

    #[derive(IntoValue, IntoDict)]
    struct Borrowed<'a> {
        title: &'a str,
        inner: Wrapper<i64>,
    }

    let dict: Dict = Borrowed {
        title: "t",
        inner: Wrapper { value: 3 },
    }
    .into();
    assert_eq!(keys(&dict), ["title", "inner"]);
    let Some(Value::Dict(inner)) = dict.get("inner").ok().cloned() else {
        panic!("nested struct is a dictionary");
    };
    assert_eq!(inner.get("value").ok().cloned(), Some(3i64.into_value()));

    let dict: Dict = Wrapper { value: "s" }.into();
    assert_eq!(keys(&dict), ["value"]);
}

#[test]
fn recursive_generic_structs_derive() {
    #[derive(IntoValue, IntoDict)]
    struct Section<T> {
        body: T,
        children: Vec<Section<T>>,
    }

    let leaf = Section {
        body: 2i64,
        children: vec![],
    };
    let dict: Dict = Section {
        body: 1i64,
        children: vec![leaf],
    }
    .into();
    assert_eq!(keys(&dict), ["body", "children"]);
}
