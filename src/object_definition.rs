use crate::{
    field_value::{DateTime, FieldValue},
    reserved_fields::{self, is_reserved_field, reserved_field_from_str, ReservedFieldError},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
};
use thiserror::Error;
use toml::Table;

#[derive(Error, Debug, Clone)]
pub enum InvalidFieldError {
    #[error("unrecognized type {0}")]
    UnrecognizedType(String),
    #[error("invalid date {0}")]
    InvalidDate(String),
    #[error(
        "type mismatch for field {field:?} - expected type {field_type:?}, got value {value:?}"
    )]
    TypeMismatch {
        field: String,
        field_type: String,
        value: String,
    },
    #[error("invalid child {key:?}[{index:?}] {child:?}")]
    InvalidChild {
        key: String,
        index: usize,
        child: String,
    },
    #[error("not an array: {key:?} ({value:?})")]
    NotAnArray { key: String, value: String },
    #[error("cannot define an object with reserved name {0}")]
    ReservedObjectNameError(String),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "typescript", derive(typescript_type_def::TypeDef))]
pub enum FieldType {
    String,
    Number,
    Date,
    Markdown,
    Boolean,
}

impl FieldType {
    fn from_str(string: &str) -> Result<FieldType, InvalidFieldError> {
        match string {
            "string" => Ok(FieldType::String),
            "number" => Ok(FieldType::Number),
            "date" => Ok(FieldType::Date),
            "markdown" => Ok(FieldType::Markdown),
            "boolean" => Ok(FieldType::Boolean),
            _ => Err(InvalidFieldError::UnrecognizedType(string.to_string())),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Date => "date",
            Self::Markdown => "markdown",
            Self::Boolean => "boolean",
        }
    }

    pub fn default_value(&self) -> FieldValue {
        match self {
            Self::String => FieldValue::String("".to_string()),
            Self::Number => FieldValue::Number(0.0),
            Self::Date => FieldValue::Date(DateTime::now()),
            Self::Markdown => FieldValue::Markdown("".to_string()),
            Self::Boolean => FieldValue::Boolean(false),
        }
    }
}

impl Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

pub type ObjectDefinitions = HashMap<String, ObjectDefinition>;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "typescript", derive(typescript_type_def::TypeDef))]
pub struct ObjectDefinition {
    pub name: String,
    pub fields: HashMap<String, FieldType>,
    pub field_order: Vec<String>,
    pub template: Option<String>,
    #[cfg_attr(feature = "typescript", serde(skip))]
    pub children: HashMap<String, ObjectDefinition>,
}

impl ObjectDefinition {
    pub fn new(name: &str, definition: &Table) -> Result<ObjectDefinition, Box<dyn Error>> {
        if is_reserved_field(name) {
            return Err(InvalidFieldError::ReservedObjectNameError(name.to_string()).into());
        }
        let mut obj_def = ObjectDefinition {
            name: name.to_string(),
            fields: HashMap::new(),
            field_order: vec![],
            template: None,
            children: HashMap::new(),
        };
        for (key, m_value) in definition {
            obj_def.field_order.push(key.to_string());
            if let Some(child_table) = m_value.as_table() {
                obj_def
                    .children
                    .insert(key.clone(), ObjectDefinition::new(key, child_table)?);
            } else if let Some(value) = m_value.as_str() {
                if key == reserved_fields::TEMPLATE {
                    obj_def.template = Some(value.to_string());
                } else if is_reserved_field(key) {
                    return Err(Box::new(ReservedFieldError {
                        field: reserved_field_from_str(key),
                    }));
                } else {
                    obj_def
                        .fields
                        .insert(key.clone(), FieldType::from_str(value)?);
                }
            }
        }
        Ok(obj_def)
    }
    pub fn from_table(table: &Table) -> Result<HashMap<String, ObjectDefinition>, Box<dyn Error>> {
        let mut objects: HashMap<String, ObjectDefinition> = HashMap::new();
        for (name, m_def) in table.into_iter() {
            if let Some(def) = m_def.as_table() {
                objects.insert(name.clone(), ObjectDefinition::new(name, def)?);
            }
        }
        Ok(objects)
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    pub fn artist_and_example_definition_str() -> &'static str {
        "[artist]
        name = \"string\"
        template = \"artist\"
        [artist.tour_dates]
        date = \"date\"
        ticket_link = \"string\"
        [artist.numbers]
        number = \"number\"
        
        [example]
        content = \"markdown\"
        [example.links]
        url = \"string\""
    }

    #[test]
    fn parsing() -> Result<(), Box<dyn Error>> {
        let table: Table = toml::from_str(artist_and_example_definition_str())?;
        let defs = ObjectDefinition::from_table(&table)?;

        println!("{:?}", defs);

        assert_eq!(defs.keys().len(), 2);
        assert!(defs.get("artist").is_some());
        assert!(defs.get("example").is_some());
        let artist = defs.get("artist").unwrap();
        assert_eq!(artist.field_order.len(), 4);
        assert_eq!(artist.field_order[0], "name".to_string());
        assert_eq!(artist.field_order[1], "template".to_string());
        assert_eq!(artist.field_order[2], "tour_dates".to_string());
        assert_eq!(artist.field_order[3], "numbers".to_string());
        assert!(artist.fields.get("name").is_some());
        assert_eq!(artist.fields.get("name").unwrap(), &FieldType::String);
        assert!(
            artist.fields.get("template").is_none(),
            "did not copy the template reserved field"
        );
        assert!(artist.template.is_some());
        assert_eq!(artist.template, Some("artist".to_string()));
        assert_eq!(artist.children.len(), 2);
        assert!(artist.children.get("tour_dates").is_some());
        assert!(artist.children.get("numbers").is_some());
        let tour_dates = artist.children.get("tour_dates").unwrap();
        assert!(tour_dates.fields.get("date").is_some());
        assert_eq!(tour_dates.fields.get("date").unwrap(), &FieldType::Date);
        assert!(tour_dates.fields.get("ticket_link").is_some());
        assert_eq!(
            tour_dates.fields.get("ticket_link").unwrap(),
            &FieldType::String
        );
        let numbers = artist.children.get("numbers").unwrap();
        assert!(numbers.fields.get("number").is_some());
        assert_eq!(numbers.fields.get("number").unwrap(), &FieldType::Number);

        let example = defs.get("example").unwrap();
        assert!(example.fields.get("content").is_some());
        assert_eq!(example.fields.get("content").unwrap(), &FieldType::Markdown);
        assert_eq!(example.children.len(), 1);
        assert!(example.children.get("links").is_some());
        let links = example.children.get("links").unwrap();
        assert!(links.fields.get("url").is_some());
        assert_eq!(links.fields.get("url").unwrap(), &FieldType::String);

        Ok(())
    }
}
