use crate::{object::Object, object_definition::ObjectDefinition};
use liquid::ValueView;
use std::{collections::HashMap, error::Error, fmt};

#[derive(Debug, Clone)]
struct InvalidPageError;
impl Error for InvalidPageError {}
impl fmt::Display for InvalidPageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid page")
    }
}

#[derive(Debug)]
pub struct PageTemplate<'a> {
    pub definition: &'a ObjectDefinition,
    pub object: &'a Object,
    pub content: String,
}

pub struct Page<'a> {
    name: String,
    content: Option<String>,
    template: Option<PageTemplate<'a>>,
}

impl<'a> Page<'a> {
    pub fn new_with_template(
        name: String,
        definition: &'a ObjectDefinition,
        object: &'a Object,
        content: String,
    ) -> Page<'a> {
        Page {
            name,
            content: None,
            template: Some(PageTemplate {
                definition,
                object,
                content,
            }),
        }
    }
    pub fn new(name: String, content: String) -> Page<'a> {
        Page {
            name,
            content: Some(content),
            template: None,
        }
    }
    pub fn render(
        &self,
        parser: &liquid::Parser,
        objects_map: &HashMap<String, Vec<Object>>,
    ) -> Result<String, Box<dyn Error>> {
        tracing::debug!("rendering {}", self.name);
        let mut objects: HashMap<String, Vec<liquid::model::Value>> = HashMap::new();
        for (name, objs) in objects_map {
            let values = objs.iter().map(|o| o.liquid_object()).collect();
            objects.insert(name.to_string(), values);
        }
        let globals = liquid::object!({ "objects": objects, "page": self.name });
        if let Some(template_info) = &self.template {
            let template = parser.parse(&template_info.content)?;
            let mut object_vals = match template_info.object.values.to_value() {
                liquid::model::Value::Object(v) => Ok(v),
                _ => Err(InvalidPageError),
            }?;
            object_vals.extend(liquid::object!({
                "object_name": template_info.object.object_name,
              "order": template_info.object.order,
              "path": template_info.object.path,
            }));
            let mut context = liquid::object!({
              template_info.definition.name.to_owned(): object_vals
            });
            tracing::debug!("=== context (template): \n{}", toml::to_string(&context)?);
            context.extend(globals);
            return Ok(template.render(&context)?);
        } else if let Some(content) = &self.content {
            tracing::debug!("=== context (page): \n{}", toml::to_string(&globals)?);
            let template = parser.parse(content)?;
            return Ok(template.render(&globals)?);
        }
        panic!("Pages must have either a template or a path");
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        fields::{DateTime, FieldType, FieldValue},
        liquid_parser, MemoryFileSystem,
    };

    use super::*;

    fn get_objects_map() -> HashMap<String, Vec<Object>> {
        let tour_dates_objects = vec![HashMap::from([
            (
                "date".to_string(),
                FieldValue::Date(DateTime::from("12/22/2022 00:00:00").unwrap()),
            ),
            (
                "ticket_link".to_string(),
                FieldValue::String("foo.com".to_string()),
            ),
        ])];
        let numbers_objects = vec![HashMap::from([(
            "number".to_string(),
            FieldValue::Number(2.57),
        )])];
        let artist_values = HashMap::from([
            (
                "name".to_string(),
                FieldValue::String("Tormenta Rey".to_string()),
            ),
            ("numbers".to_string(), FieldValue::Objects(numbers_objects)),
            (
                "tour_dates".to_string(),
                FieldValue::Objects(tour_dates_objects),
            ),
        ]);
        let artist = Object {
            filename: "tormenta-rey".to_string(),
            object_name: "artist".to_string(),
            path: "artist/tormenta-rey".to_string(),
            order: 1,
            values: artist_values,
        };
        let links_objects = vec![HashMap::from([(
            "url".to_string(),
            FieldValue::String("foo.com".to_string()),
        )])];
        let c_values = HashMap::from([
            (
                "content".to_string(),
                FieldValue::Markdown("# hello".to_string()),
            ),
            ("name".to_string(), FieldValue::String("home".to_string())),
            ("links".to_string(), FieldValue::Objects(links_objects)),
        ]);

        let c = Object {
            filename: "home".to_string(),
            object_name: "c".to_string(),
            path: "home".to_string(),
            order: -1,
            values: c_values,
        };

        HashMap::from([
            ("artist".to_string(), vec![artist]),
            ("c".to_string(), vec![c]),
        ])
    }

    fn artist_definition() -> ObjectDefinition {
        let artist_def_fields = HashMap::from([("name".to_string(), FieldType::String)]);
        let tour_dates_fields = HashMap::from([
            ("date".to_string(), FieldType::Date),
            ("ticket_link".to_string(), FieldType::String),
        ]);
        let numbers_fields = HashMap::from([("number".to_string(), FieldType::Number)]);
        let artist_children = HashMap::from([
            (
                "tour_dates".to_string(),
                ObjectDefinition {
                    name: "tour_dates".to_string(),
                    field_order: vec!["date".to_string(), "ticket_link".to_string()],
                    fields: tour_dates_fields,
                    template: None,
                    children: HashMap::new(),
                },
            ),
            (
                "numbers".to_string(),
                ObjectDefinition {
                    name: "numbers".to_string(),
                    field_order: vec![],
                    fields: numbers_fields,
                    template: None,
                    children: HashMap::new(),
                },
            ),
        ]);
        ObjectDefinition {
            name: "artist".to_string(),
            field_order: vec![
                "name".to_string(),
                "tour_dates".to_string(),
                "numbers".to_string(),
            ],
            fields: artist_def_fields,
            template: Some("artist".to_string()),
            children: artist_children,
        }
    }

    fn page_content() -> &'static str {
        "{% assign c = objects.c | where: \"name\", \"home\" | first %}
        name: {{c.name}}
        content: {{c.content}}
        page_path: {{c.path}}
        {% for link in c.links %}
          link: {{link.url}}
        {% endfor %}
        {% for artist in objects.artist %}
          artist: {{artist.name}}
          path: {{artist.path}}
        {% endfor %}
        "
    }
    fn artist_template_content() -> &'static str {
        "name: {{artist.name}}
        {% for number in artist.numbers %}
          number: {{number.number}}
        {% endfor %}
        {% for date in artist.tour_dates %}
          date: {{date.date | date: \"%b %d, %y\"}}
          link: {{date.ticket_link}}
        {% endfor %}"
    }

    #[test]
    fn regular_page() -> Result<(), Box<dyn Error>> {
        let liquid_parser = liquid_parser::get(None, None, &MemoryFileSystem::default())?;
        let objects_map = get_objects_map();
        let page = Page::new("home".to_string(), page_content().to_string());
        let rendered = page.render(&liquid_parser, &objects_map)?;
        println!("rendered: {}", rendered);
        assert!(rendered.contains("name: home"), "filtered object");
        assert!(
            rendered.contains("content: <h1>hello</h1>"),
            "markdown field"
        );
        assert!(rendered.contains("link: foo.com"), "child string field");
        assert!(
            rendered.contains("artist: Tormenta Rey"),
            "item from objects"
        );
        assert!(rendered.contains("page_path: home"), "path is defined");
        assert!(
            rendered.contains("path: artist/tormenta-rey"),
            "items define paths"
        );
        Ok(())
    }
    #[test]
    fn template_page() -> Result<(), Box<dyn Error>> {
        let liquid_parser = liquid_parser::get(None, None, &MemoryFileSystem::default())?;
        let objects_map = get_objects_map();
        let object = objects_map["artist"].first().unwrap();
        let artist_def = artist_definition();
        let page = Page::new_with_template(
            "tormenta-rey".to_string(),
            &artist_def,
            object,
            artist_template_content().to_string(),
        );
        let rendered = page.render(&liquid_parser, &objects_map)?;
        println!("rendered: {}", rendered);
        assert!(rendered.contains("name: Tormenta Rey"), "root field");
        assert!(rendered.contains("number: 2.57"), "child number field");
        assert!(rendered.contains("date: Dec 22, 22"), "child date field");
        assert!(rendered.contains("link: foo.com"), "child string field");
        Ok(())
    }
}
