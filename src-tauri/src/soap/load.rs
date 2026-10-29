//! Reads a WSDL together with the documents its imports point to, from a URL, from a file the
//! user picked, or from content dropped on the Import page.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use url::Url;

use super::model::{Warnings, WsdlDefinition, WsdlSource};
use super::schema::schema_nodes;
use super::wsdl::{is_wsdl, parse_definition};
use super::xml::{self, WSDL_NS, children, is_builtin, is_standard_location, xsd_name};
use crate::error::AppError;
use crate::files::read_limited;
use crate::import::fetch::fetch_text;
use crate::import::postman::MAX_IMPORT_BYTES;
use crate::request::client::HttpClient;
use crate::request::validate::web_url;

const WSDL_ACCEPT: &str = "application/wsdl+xml, application/xml;q=0.9, text/xml;q=0.9, */*;q=0.5";
/// Documents read for one WSDL, the main one included.
const MAX_DOCUMENTS: usize = 64;
const TOO_BIG: &str = "The file is larger than 20 MB";
const SCHEMA_REFERENCES: [&str; 3] = ["import", "include", "redefine"];

pub async fn load_definition(
    client: &HttpClient,
    source: &WsdlSource,
) -> Result<WsdlDefinition, AppError> {
    let mut gatherer = match source {
        WsdlSource::Url { url } => {
            // Plain http is allowed like the SOAP requests themselves: many services offer
            // their WSDL only that way.
            let url = web_url(url)?;
            let main = fetch_text(client, &url, WSDL_ACCEPT).await?;
            Gatherer::new(main, Origin::Remote(url))
        }
        WsdlSource::File { path } => {
            let path = PathBuf::from(path);
            blocking(move || Gatherer::from_file(&path)).await??
        }
        WsdlSource::Content { content } => Gatherer::new(content.clone(), Origin::Pasted),
    };
    loop {
        let (returned, next) = blocking(move || {
            let next = gatherer.next_remote();
            (gatherer, next)
        })
        .await?;
        gatherer = returned;
        let Some(url) = next else {
            break;
        };
        let fetched = fetch_text(client, &url, WSDL_ACCEPT).await;
        gatherer.add_remote(url, fetched);
    }
    blocking(move || gatherer.finish()).await?
}

/// Parsing and file reads run off the async runtime's worker threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| AppError::invalid("The WSDL could not be read"))
}

enum Origin {
    Remote(Url),
    /// `folder` is the canonical folder of the main file; local imports must stay inside it.
    File {
        url: Url,
        folder: PathBuf,
    },
    /// Dropped content, which has no location for relative imports.
    Pasted,
}

impl Origin {
    fn url(&self) -> Option<&Url> {
        match self {
            Self::Remote(url) | Self::File { url, .. } => Some(url),
            Self::Pasted => None,
        }
    }
}

/// Collects the documents of one WSDL. Local files are read as imports are found; remote ones
/// are handed out by `next_remote` so the caller can download them.
struct Gatherer {
    main: String,
    origin: Origin,
    /// Imported documents by resolved location.
    documents: BTreeMap<String, String>,
    /// Documents whose own imports are still to be followed; `None` is the main one.
    unscanned: Vec<Option<String>>,
    remote: VecDeque<Url>,
    seen: BTreeSet<String>,
    total_bytes: usize,
    warnings: Warnings,
}

impl Gatherer {
    fn new(main: String, origin: Origin) -> Self {
        let seen = origin.url().map(Url::to_string).into_iter().collect();
        Self {
            total_bytes: main.len(),
            main,
            origin,
            documents: BTreeMap::new(),
            unscanned: vec![None],
            remote: VecDeque::new(),
            seen,
            warnings: Warnings::default(),
        }
    }

    fn from_file(path: &Path) -> Result<Self, AppError> {
        let path = fs::canonicalize(path).map_err(|_| AppError::FileAccess)?;
        let main = read_text(&path)?;
        let url = Url::from_file_path(&path).map_err(|()| AppError::FileAccess)?;
        let folder = path
            .parent()
            .map(Path::to_path_buf)
            .ok_or(AppError::FileAccess)?;
        Ok(Self::new(main, Origin::File { url, folder }))
    }

    /// Follows the imports found so far and returns the next document to download, if any.
    fn next_remote(&mut self) -> Option<Url> {
        while let Some(key) = self.unscanned.pop() {
            self.follow_imports(key);
        }
        self.remote.pop_front()
    }

    fn add_remote(&mut self, url: Url, fetched: Result<String, AppError>) {
        match fetched {
            Ok(text) => self.add_document(url.to_string(), text),
            Err(error) => self
                .warnings
                .add(format!("Could not read the import {url}: {error}")),
        }
    }

    fn finish(self) -> Result<WsdlDefinition, AppError> {
        let mut definition = parse_definition(&self.main, &self.documents)?;
        let mut warnings = self.warnings;
        for warning in definition.warnings.drain(..) {
            warnings.add(warning);
        }
        definition.warnings = warnings.into_vec();
        Ok(definition)
    }

    fn follow_imports(&mut self, key: Option<String>) {
        let (locations, base) = match &key {
            // Imports of something that is not a WSDL are not worth downloading: reading fails.
            None if !is_wsdl(&self.main) => return,
            None => (references(&self.main), self.origin.url().cloned()),
            Some(key) => match self.documents.get(key) {
                Some(text) => (references(text), Url::parse(key).ok()),
                None => return,
            },
        };
        for location in locations {
            self.follow(base.as_ref(), &location);
        }
    }

    fn follow(&mut self, base: Option<&Url>, location: &str) {
        let url = match resolve(base, location) {
            Ok(url) => url,
            Err(reason) => {
                self.warnings
                    .add(format!("Skipped the import {location}: {reason}"));
                return;
            }
        };
        if is_standard_location(&url) {
            return;
        }
        if url.scheme() == "file" {
            self.read_local(&url, location);
        } else {
            self.queue_remote(&url, location);
        }
    }

    fn queue_remote(&mut self, url: &Url, location: &str) {
        let Ok(url) = web_url(url.as_str()) else {
            self.warnings.add(format!(
                "Skipped the import {location}: it is not a valid web address"
            ));
            return;
        };
        if self.seen.insert(url.to_string()) && self.has_room() {
            self.remote.push_back(url);
        }
    }

    fn read_local(&mut self, url: &Url, location: &str) {
        let Origin::File { folder, .. } = &self.origin else {
            return;
        };
        let folder = folder.clone();
        let Some(path) = url
            .to_file_path()
            .ok()
            .and_then(|path| fs::canonicalize(path).ok())
        else {
            self.warnings
                .add(format!("Could not read the import {location}"));
            return;
        };
        // Canonical paths resolve `..` and links, so this holds for where the file really is.
        if !path.starts_with(&folder) {
            self.warnings.add(format!(
                "Skipped the import {location}: it is outside the WSDL's folder"
            ));
            return;
        }
        let Ok(key) = Url::from_file_path(&path).map(String::from) else {
            return;
        };
        if !self.seen.insert(key.clone()) || !self.has_room() {
            return;
        }
        match read_text(&path) {
            Ok(text) => self.add_document(key, text),
            Err(error) => self
                .warnings
                .add(format!("Could not read the import {location}: {error}")),
        }
    }

    fn has_room(&mut self) -> bool {
        let room = 1 + self.documents.len() + self.remote.len() < MAX_DOCUMENTS;
        if !room {
            self.warnings.add(format!(
                "Only the first {MAX_DOCUMENTS} documents of the WSDL were read"
            ));
        }
        room
    }

    fn add_document(&mut self, key: String, text: String) {
        if self.total_bytes + text.len() > MAX_IMPORT_BYTES {
            self.warnings
                .add("Imports stop at 20 MB in total; the rest were not read".to_owned());
            return;
        }
        self.total_bytes += text.len();
        self.unscanned.push(Some(key.clone()));
        self.documents.insert(key, text);
    }
}

fn read_text(path: &Path) -> Result<String, AppError> {
    let bytes = read_limited(path, MAX_IMPORT_BYTES as u64, TOO_BIG)?;
    String::from_utf8(bytes).map_err(|_| AppError::invalid("The file is not UTF-8 text"))
}

/// Locations of the WSDLs and schemas a document imports, standard schemas left out.
fn references(text: &str) -> Vec<String> {
    let Ok(document) = xml::parse(text) else {
        return Vec::new();
    };
    let root = document.root_element();
    let wsdl_imports = children(root, WSDL_NS, "import")
        .filter(|import| !import.attribute("namespace").is_some_and(is_builtin))
        .filter_map(|import| import.attribute("location"));
    let schema_imports = schema_nodes(root)
        .into_iter()
        .flat_map(|schema| schema.children())
        .filter(|child| xsd_name(*child).is_some_and(|name| SCHEMA_REFERENCES.contains(&name)))
        .filter(|child| !child.attribute("namespace").is_some_and(is_builtin))
        .filter_map(|child| child.attribute("schemaLocation"));
    wsdl_imports
        .chain(schema_imports)
        .map(str::trim)
        .filter(|location| !location.is_empty())
        .map(str::to_owned)
        .collect()
}

/// An import location made absolute. Local files are only read from local documents, so a
/// downloaded WSDL cannot reach into this machine.
fn resolve(base: Option<&Url>, location: &str) -> Result<Url, &'static str> {
    let mut url = match Url::parse(location) {
        Ok(url) => url,
        Err(url::ParseError::RelativeUrlWithoutBase) => base
            .ok_or("relative imports need the WSDL's URL or file")?
            .join(location)
            .map_err(|_| "the location is not valid")?,
        Err(_) => return Err("the location is not valid"),
    };
    url.set_fragment(None);
    let from_file = base.is_some_and(|base| base.scheme() == "file");
    match url.scheme() {
        "http" | "https" => Ok(url),
        "file" if from_file => Ok(url),
        _ => Err("only web addresses and files next to the WSDL are read"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads local imports and refuses remote ones, as a run without network would.
    fn offline(mut gatherer: Gatherer) -> Result<WsdlDefinition, AppError> {
        while let Some(url) = gatherer.next_remote() {
            gatherer.add_remote(url, Err(AppError::invalid("offline")));
        }
        gatherer.finish()
    }

    const MAIN: &str = r#"<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
      xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/" xmlns:xs="http://www.w3.org/2001/XMLSchema"
      xmlns:p="urn:ping" xmlns:tns="urn:svc" targetNamespace="urn:svc">
      <wsdl:types>
        <xs:schema targetNamespace="urn:svc">
          <xs:import namespace="urn:ping" schemaLocation="types/ping.xsd"/>
          <xs:import namespace="urn:outside" schemaLocation="../outside.xsd"/>
          <xs:import namespace="http://www.w3.org/XML/1998/namespace" schemaLocation="http://www.w3.org/2001/xml.xsd"/>
          <xs:import namespace="urn:web" schemaLocation="https://example.invalid/web.xsd"/>
        </xs:schema>
      </wsdl:types>
      <wsdl:message name="In"><wsdl:part name="p" element="p:Ping"/></wsdl:message>
      <wsdl:portType name="P"><wsdl:operation name="Ping"><wsdl:input message="tns:In"/></wsdl:operation></wsdl:portType>
      <wsdl:binding name="B" type="tns:P">
        <soap:binding/>
        <wsdl:operation name="Ping"><wsdl:input><soap:body use="literal"/></wsdl:input></wsdl:operation>
      </wsdl:binding>
    </wsdl:definitions>"#;

    const PING: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:ping"
      elementFormDefault="qualified">
      <xs:include schemaLocation="ping-parts.xsd"/>
      <xs:element name="Ping"><xs:complexType><xs:sequence>
        <xs:element name="target" type="xs:string"/>
      </xs:sequence></xs:complexType></xs:element>
    </xs:schema>"#;

    const PING_PARTS: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:ping">
      <xs:include schemaLocation="ping.xsd"/>
    </xs:schema>"#;

    #[test]
    fn reads_imports_inside_the_folder_and_refuses_the_rest() {
        let root = std::env::temp_dir().join(format!("codestamp-soap-{}", uuid::Uuid::new_v4()));
        let service = root.join("service");
        fs::create_dir_all(service.join("types")).expect("folders");
        fs::write(service.join("main.wsdl"), MAIN).expect("main");
        fs::write(service.join("types").join("ping.xsd"), PING).expect("ping");
        fs::write(service.join("types").join("ping-parts.xsd"), PING_PARTS).expect("parts");
        fs::write(root.join("outside.xsd"), PING).expect("outside");

        let gatherer = Gatherer::from_file(&service.join("main.wsdl")).expect("main file");
        let definition = offline(gatherer).expect("definition");
        fs::remove_dir_all(&root).expect("clean up");

        assert!(
            definition.interfaces[0].operations[0]
                .envelope
                .contains("<pin:Ping>\n      <pin:target>?</pin:target>\n    </pin:Ping>")
        );
        assert_eq!(
            definition.warnings,
            [
                "Skipped the import ../outside.xsd: it is outside the WSDL's folder",
                "Could not read the import https://example.invalid/web.xsd: offline"
            ]
        );
    }

    #[test]
    fn dropped_content_skips_relative_imports() {
        let definition =
            offline(Gatherer::new(MAIN.to_owned(), Origin::Pasted)).expect("definition");
        assert_eq!(
            definition.warnings[..2],
            [
                "Skipped the import types/ping.xsd: relative imports need the WSDL's URL or file",
                "Skipped the import ../outside.xsd: relative imports need the WSDL's URL or file"
            ]
        );
    }

    #[test]
    fn follows_plain_http_imports_on_any_host() {
        let main = r#"<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
          xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">
          <wsdl:types><xs:schema>
            <xs:import namespace="urn:t" schemaLocation="types.xsd"/>
          </xs:schema></wsdl:types>
        </wsdl:definitions>"#;
        let base = Url::parse("http://intranet.example/svc/main.wsdl").expect("url");
        let mut gatherer = Gatherer::new(main.to_owned(), Origin::Remote(base));
        assert_eq!(
            gatherer.next_remote().map(String::from),
            Some("http://intranet.example/svc/types.xsd".to_owned())
        );
    }

    #[test]
    fn resolves_locations_safely() {
        let web = Url::parse("https://example.com/svc/main.wsdl").expect("url");
        let file = Url::parse("file:///C:/svc/main.wsdl").expect("url");
        assert_eq!(
            resolve(Some(&web), "types.xsd#x").map(String::from),
            Ok("https://example.com/svc/types.xsd".to_owned())
        );
        assert!(resolve(Some(&web), "file:///C:/secret.xsd").is_err());
        assert!(resolve(Some(&file), "file:///C:/svc/a.xsd").is_ok());
        assert!(resolve(Some(&file), "C:\\svc\\a.xsd").is_err());
        assert!(resolve(None, "a.xsd").is_err());
    }
}
