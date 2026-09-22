//! Method documents served as MCP resources, embedded at build time. The Dockerfile
//! copies `docs/` to where these relative paths resolve inside the build stage.

use rmcp::model::{ListResourcesResult, ReadResourceResult, Resource, ResourceContents};
use rmcp::ErrorData;

struct Doc {
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    text: &'static str,
}

const DOCS: &[Doc] = &[
    Doc {
        uri: "turfops://docs/agronomy",
        name: "agronomy-methods",
        description: "Thresholds, nitrogen targets, disease models and tiers, fungicide \
                      efficacy source, shelf matching and data-staleness rules behind \
                      TurfOps' advice.",
        text: include_str!("../../../docs/agronomy-methods.md"),
    },
    Doc {
        uri: "turfops://docs/timing-windows",
        name: "timing-windows",
        description: "How the seeding and pre-emergent windows are computed: 5 cm soil \
                      crossings held 5 days, typical dates from up to 15 station years, \
                      freeze dates, and how this season's state is resolved.",
        text: include_str!("../../../docs/timing-windows.md"),
    },
];

pub fn list() -> ListResourcesResult {
    ListResourcesResult::with_all_items(
        DOCS.iter()
            .map(|doc| {
                Resource::new(doc.uri, doc.name)
                    .with_description(doc.description)
                    .with_mime_type("text/markdown")
                    .with_size(doc.text.len() as u64)
            })
            .collect(),
    )
}

pub fn read(uri: &str) -> Result<ReadResourceResult, ErrorData> {
    let doc = DOCS
        .iter()
        .find(|doc| doc.uri == uri)
        .ok_or_else(|| ErrorData::resource_not_found(format!("No resource {uri}"), None))?;
    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        doc.text, doc.uri,
    )
    .with_mime_type("text/markdown")]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_doc_is_listed_and_readable() {
        let listed = list();
        assert_eq!(listed.resources.len(), DOCS.len());
        for resource in &listed.resources {
            let read = read(&resource.uri).unwrap();
            assert_eq!(read.contents.len(), 1);
        }
    }

    #[test]
    fn docs_are_not_empty() {
        for doc in DOCS {
            assert!(
                doc.text.starts_with("# "),
                "{} is empty or not markdown",
                doc.uri
            );
        }
    }

    #[test]
    fn unknown_uri_is_not_found() {
        assert!(read("turfops://docs/nope").is_err());
    }
}
