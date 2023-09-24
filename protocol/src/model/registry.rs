use crate::serde;
use bevy::prelude::Resource;
use std::collections::HashMap;

use serde::{ser::SerializeStruct, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct Registry<T: RegistryItem>(pub HashMap<String, T>);

impl<T: RegistryItem> Default for Registry<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

pub trait RegistryItem {
    const REGISTRY: &'static str;
}

#[derive(Serialize, Clone)]
struct Yeet<T> {
    name: String,
    id: i32,
    element: T,
}

impl<T: RegistryItem + Clone + Serialize> Serialize for Registry<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let yeet: Vec<Yeet<T>> = self
            .0
            .iter()
            .enumerate()
            .map(|(id, (name, element))| Yeet {
                name: name.clone(),
                id: id.try_into().expect("id didnt fit"),
                element: element.clone(),
            })
            .collect();
        let mut stf = serializer.serialize_struct("Registry", 2)?;
        stf.serialize_field("type", T::REGISTRY)?;
        stf.serialize_field("value", &yeet)?;
        stf.end()
    }
}
