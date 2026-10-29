//! Where a server's services and message types come from: a `.proto` file the user picked, or
//! the server's own reflection service. Also lists the methods and builds message templates.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use prost::Message;
use prost_reflect::prost_types::{DescriptorProto, FileDescriptorProto, FileDescriptorSet};
use prost_reflect::{DescriptorPool, FieldDescriptor, Kind, MessageDescriptor};
use serde::Serialize;
use serde_json::{Map, Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::metadata::MetadataMap;
use tonic::transport::Channel;
use tonic::{Code, Request};

use crate::error::AppError;

/// Files a reflection answer may pull in, following imports.
const MAX_FILES: usize = 500;
const MAX_TEMPLATE_DEPTH: usize = 4;
const REFLECTION_SERVICES: [&str; 2] = [
    "grpc.reflection.v1.ServerReflection",
    "grpc.reflection.v1alpha.ServerReflection",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodInfo {
    pub name: String,
    /// `package.Service/Method`, as requests store it.
    pub full_name: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
    /// An input message with every field, as a starting point (JSON).
    pub template: String,
}

/// Compiles a `.proto` file; its imports are read from its own folder (and the well-known
/// Google types are built in).
pub fn from_proto_file(path: &Path) -> Result<DescriptorPool, AppError> {
    let folder = path
        .parent()
        .ok_or_else(|| AppError::invalid("The .proto file has no folder"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::invalid("This file name cannot be used"))?;
    let mut compiler = protox::Compiler::new([folder]).map_err(|error| {
        AppError::InvalidInput(format!("The .proto file cannot be read: {error}"))
    })?;
    compiler.include_imports(true);
    compiler.open_file(name).map_err(|error| {
        AppError::InvalidInput(format!("The .proto file has an error: {error}"))
    })?;
    DescriptorPool::from_file_descriptor_set(compiler.file_descriptor_set())
        .map_err(|error| AppError::InvalidInput(format!("The .proto file has an error: {error}")))
}

/// Asks the server's reflection service for every service and the files they need. The newer
/// v1 protocol is tried first, then v1alpha, which older servers offer.
pub async fn from_reflection(
    channel: Channel,
    metadata: &MetadataMap,
) -> Result<DescriptorPool, AppError> {
    let files = match reflect_v1(channel.clone(), metadata).await {
        Err(Reflection::Unimplemented) => reflect_v1alpha(channel, metadata).await,
        other => other,
    }
    .map_err(Reflection::into_error)?;
    pool_from_files(files)
}

fn pool_from_files(
    mut files: HashMap<String, FileDescriptorProto>,
) -> Result<DescriptorPool, AppError> {
    add_well_known(&mut files);
    repair_imports(&mut files);
    let mut ordered = Vec::with_capacity(files.len());
    let mut placed = HashSet::new();
    for name in files.keys() {
        place(name, &files, &mut placed, &mut ordered);
    }
    DescriptorPool::from_file_descriptor_set(FileDescriptorSet { file: ordered }).map_err(|error| {
        AppError::InvalidInput(format!("The server's descriptors cannot be used: {error}"))
    })
}

/// Servers often leave out the well-known Google types (such as Timestamp) their files use;
/// those are built in.
fn add_well_known(files: &mut HashMap<String, FileDescriptorProto>) {
    let global = DescriptorPool::global();
    loop {
        let mut wanted: Vec<String> = files
            .values()
            .flat_map(|file| file.dependency.iter().cloned())
            .collect();
        wanted.extend(
            missing_references(files)
                .iter()
                .filter_map(|reference| reference.well_known_file(&global)),
        );
        let found: Vec<(String, FileDescriptorProto)> = wanted
            .into_iter()
            .filter(|name| !files.contains_key(name))
            .filter_map(|name| {
                global
                    .get_file_by_name(&name)
                    .map(|file| (name, file.file_descriptor_proto().clone()))
            })
            .collect();
        if found.is_empty() {
            return;
        }
        files.extend(found);
    }
}

/// Servers built on protobufjs merge files and leave their imports out, which descriptors must
/// have; each file is made to import the files that define the types it uses.
fn repair_imports(files: &mut HashMap<String, FileDescriptorProto>) {
    let defined = defined_types(files);
    for (name, file) in files.iter_mut() {
        for reference in references(file) {
            if let Some(owner) = reference.owner(&defined)
                && owner != name
                && !file.dependency.contains(owner)
            {
                file.dependency.push(owner.clone());
            }
        }
    }
}

/// References to types that none of the files defines.
fn missing_references(files: &HashMap<String, FileDescriptorProto>) -> Vec<Reference> {
    let defined = defined_types(files);
    files
        .values()
        .flat_map(references)
        .filter(|reference| reference.owner(&defined).is_none())
        .collect()
}

/// The file that defines each message and enum, by full name.
fn defined_types(files: &HashMap<String, FileDescriptorProto>) -> HashMap<String, String> {
    fn add(prefix: &str, message: &DescriptorProto, file: &str, out: &mut HashMap<String, String>) {
        let name = join(prefix, message.name());
        for nested in &message.nested_type {
            add(&name, nested, file, out);
        }
        for item in &message.enum_type {
            out.insert(join(&name, item.name()), file.to_owned());
        }
        out.insert(name, file.to_owned());
    }
    let mut defined = HashMap::new();
    for (name, file) in files {
        for message in &file.message_type {
            add(file.package(), message, name, &mut defined);
        }
        for item in &file.enum_type {
            defined.insert(join(file.package(), item.name()), name.clone());
        }
    }
    defined
}

/// A type name as a field or method writes it: full with a leading dot, or relative to the
/// scope it is written in (protobufjs writes relative names).
struct Reference {
    package: String,
    scope: String,
    name: String,
}

impl Reference {
    /// The full names it may mean, innermost scope first, as protobuf resolves names.
    fn candidates(&self) -> Vec<String> {
        if let Some(full) = self.name.strip_prefix('.') {
            return vec![full.to_owned()];
        }
        let mut found = vec![join(&self.scope, &self.name)];
        let mut scope = self.scope.as_str();
        while !scope.is_empty() {
            scope = scope.rfind('.').map_or("", |dot| &scope[..dot]);
            found.push(join(scope, &self.name));
        }
        found
    }

    fn owner<'a>(&self, defined: &'a HashMap<String, String>) -> Option<&'a String> {
        self.candidates().iter().find_map(|name| defined.get(name))
    }

    fn well_known_file(&self, global: &DescriptorPool) -> Option<String> {
        self.candidates().iter().find_map(|name| {
            global
                .get_message_by_name(name)
                .map(|message| message.parent_file().name().to_owned())
                .or_else(|| {
                    global
                        .get_enum_by_name(name)
                        .map(|item| item.parent_file().name().to_owned())
                })
        })
    }

    /// The full name to ask a server about when no file defines it yet.
    fn likely_name(&self) -> String {
        match self.name.strip_prefix('.') {
            Some(full) => full.to_owned(),
            None if self.name.contains('.') => self.name.clone(),
            None => join(&self.package, &self.name),
        }
    }
}

/// The types a file's fields and methods refer to.
fn references(file: &FileDescriptorProto) -> Vec<Reference> {
    fn walk(package: &str, scope: &str, message: &DescriptorProto, out: &mut Vec<Reference>) {
        let scope = join(scope, message.name());
        for name in message
            .field
            .iter()
            .filter_map(|field| field.type_name.as_deref())
        {
            out.push(Reference {
                package: package.to_owned(),
                scope: scope.clone(),
                name: name.to_owned(),
            });
        }
        for nested in &message.nested_type {
            walk(package, &scope, nested, out);
        }
    }
    let package = file.package();
    let mut found = Vec::new();
    for message in &file.message_type {
        walk(package, package, message, &mut found);
    }
    for method in file.service.iter().flat_map(|service| &service.method) {
        for name in [method.input_type(), method.output_type()] {
            found.push(Reference {
                package: package.to_owned(),
                scope: package.to_owned(),
                name: name.to_owned(),
            });
        }
    }
    found
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

/// Files must come after the files they import.
fn place(
    name: &str,
    files: &HashMap<String, FileDescriptorProto>,
    placed: &mut HashSet<String>,
    ordered: &mut Vec<FileDescriptorProto>,
) {
    if !placed.insert(name.to_owned()) {
        return;
    }
    let Some(file) = files.get(name) else {
        return;
    };
    for dependency in &file.dependency {
        place(dependency, files, placed, ordered);
    }
    ordered.push(file.clone());
}

enum Reflection {
    Unimplemented,
    Failed(String),
}

impl Reflection {
    fn into_error(self) -> AppError {
        match self {
            Self::Unimplemented => AppError::invalid(
                "This server does not offer reflection; choose its .proto file instead",
            ),
            Self::Failed(message) => {
                AppError::InvalidInput(format!("Server reflection failed: {message}"))
            }
        }
    }
}

/// The two reflection protocols have the same shape in different packages.
macro_rules! reflect {
    ($name:ident, $module:path) => {
        async fn $name(
            channel: Channel,
            metadata: &MetadataMap,
        ) -> Result<HashMap<String, FileDescriptorProto>, Reflection> {
            use pb::server_reflection_client::ServerReflectionClient;
            use pb::server_reflection_request::MessageRequest;
            use pb::server_reflection_response::MessageResponse;
            use $module as pb;

            let (sender, receiver) = mpsc::channel(16);
            let mut request = Request::new(ReceiverStream::new(receiver));
            *request.metadata_mut() = metadata.clone();
            let send = |message: MessageRequest| {
                let sender = sender.clone();
                async move {
                    sender
                        .send(pb::ServerReflectionRequest {
                            host: String::new(),
                            message_request: Some(message),
                        })
                        .await
                        .map_err(|_| Reflection::Failed("the connection closed".to_owned()))
                }
            };
            send(MessageRequest::ListServices(String::new())).await?;
            let mut answers = ServerReflectionClient::new(channel)
                .server_reflection_info(request)
                .await
                .map_err(|status| match status.code() {
                    Code::Unimplemented => Reflection::Unimplemented,
                    _ => Reflection::Failed(status.message().to_owned()),
                })?
                .into_inner();
            async fn next(
                answers: &mut tonic::Streaming<pb::ServerReflectionResponse>,
            ) -> Result<MessageResponse, Reflection> {
                match answers.message().await {
                    Ok(Some(answer)) => answer
                        .message_response
                        .ok_or_else(|| Reflection::Failed("an empty answer".to_owned())),
                    Ok(None) => Err(Reflection::Failed("the server ended the answer".to_owned())),
                    Err(status) if status.code() == Code::Unimplemented => {
                        Err(Reflection::Unimplemented)
                    }
                    Err(status) => Err(Reflection::Failed(status.message().to_owned())),
                }
            }
            let services = match next(&mut answers).await? {
                MessageResponse::ListServicesResponse(list) => list
                    .service
                    .into_iter()
                    .map(|service| service.name)
                    .filter(|name| !REFLECTION_SERVICES.contains(&name.as_str()))
                    .collect::<Vec<_>>(),
                MessageResponse::ErrorResponse(error) => {
                    return Err(Reflection::Failed(error.error_message));
                }
                _ => return Err(Reflection::Failed("an unexpected answer".to_owned())),
            };
            let global = DescriptorPool::global();
            let mut files: HashMap<String, FileDescriptorProto> = HashMap::new();
            let mut asked: HashSet<String> = HashSet::new();
            let mut probing = false;
            let mut wanted: Vec<MessageRequest> = services
                .into_iter()
                .map(MessageRequest::FileContainingSymbol)
                .collect();
            loop {
                while let Some(message) = wanted.pop() {
                    send(message).await?;
                    let found = match next(&mut answers).await? {
                        MessageResponse::FileDescriptorResponse(found) => found,
                        MessageResponse::ErrorResponse(_) if probing => continue,
                        MessageResponse::ErrorResponse(error) => {
                            return Err(Reflection::Failed(error.error_message));
                        }
                        _ => return Err(Reflection::Failed("an unexpected answer".to_owned())),
                    };
                    for bytes in found.file_descriptor_proto {
                        let file = FileDescriptorProto::decode(bytes.as_slice()).map_err(|_| {
                            Reflection::Failed("an unreadable file descriptor".to_owned())
                        })?;
                        // Well-known Google files are built in; some servers do not serve them.
                        for dependency in &file.dependency {
                            if !files.contains_key(dependency)
                                && file.name.as_deref() != Some(dependency.as_str())
                                && global.get_file_by_name(dependency).is_none()
                            {
                                wanted.push(MessageRequest::FileByFilename(dependency.clone()));
                            }
                        }
                        files.insert(file.name.clone().unwrap_or_default(), file);
                    }
                    wanted.retain(|request| match request {
                        MessageRequest::FileByFilename(name) => !files.contains_key(name),
                        _ => true,
                    });
                    if files.len() > MAX_FILES {
                        return Err(Reflection::Failed("too many files".to_owned()));
                    }
                }
                // Servers that leave imports out are asked for each type still missing.
                probing = true;
                wanted = missing_references(&files)
                    .iter()
                    .filter(|reference| reference.well_known_file(&global).is_none())
                    .map(Reference::likely_name)
                    .filter(|name| asked.insert(name.clone()))
                    .map(MessageRequest::FileContainingSymbol)
                    .collect();
                if wanted.is_empty() {
                    return Ok(files);
                }
            }
        }
    };
}

reflect!(reflect_v1, tonic_reflection::pb::v1);
reflect!(reflect_v1alpha, tonic_reflection::pb::v1alpha);

/// Every service and method in the descriptors, with a template for each input message.
pub fn services(pool: &DescriptorPool) -> Vec<ServiceInfo> {
    pool.services()
        .filter(|service| !REFLECTION_SERVICES.contains(&service.full_name()))
        .map(|service| ServiceInfo {
            name: service.full_name().to_owned(),
            methods: service
                .methods()
                .map(|method| MethodInfo {
                    name: method.name().to_owned(),
                    full_name: format!("{}/{}", service.full_name(), method.name()),
                    client_streaming: method.is_client_streaming(),
                    server_streaming: method.is_server_streaming(),
                    template: serde_json::to_string_pretty(&template(&method.input(), 0))
                        .unwrap_or_default(),
                })
                .collect(),
        })
        .collect()
}

/// A JSON message with each field set to an empty value of its type (proto3 JSON names).
fn template(message: &MessageDescriptor, depth: usize) -> Value {
    match message.full_name() {
        "google.protobuf.Timestamp" => return json!("1970-01-01T00:00:00Z"),
        "google.protobuf.Duration" => return json!("0s"),
        "google.protobuf.Struct" | "google.protobuf.Any" => return json!({}),
        _ => {}
    }
    let mut object = Map::new();
    let mut oneofs = HashSet::new();
    for field in message.fields() {
        // Only one member of a oneof may be set, so the first one stands for the group.
        if let Some(oneof) = field.containing_oneof()
            && !oneofs.insert(oneof.name().to_owned())
        {
            continue;
        }
        object.insert(field.json_name().to_owned(), field_template(&field, depth));
    }
    Value::Object(object)
}

fn field_template(field: &FieldDescriptor, depth: usize) -> Value {
    if field.is_map() {
        return json!({});
    }
    let single = match field.kind() {
        Kind::Message(message) if depth < MAX_TEMPLATE_DEPTH => template(&message, depth + 1),
        Kind::Message(_) => json!({}),
        Kind::Enum(values) => values
            .values()
            .next()
            .map_or(Value::Null, |value| json!(value.name())),
        Kind::String | Kind::Bytes => json!(""),
        Kind::Bool => json!(false),
        // proto3 JSON writes 64-bit integers as strings.
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 | Kind::Uint64 | Kind::Fixed64 => json!("0"),
        Kind::Float | Kind::Double => json!(0.0),
        _ => json!(0),
    };
    if field.is_list() {
        Value::Array(vec![single])
    } else {
        single
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROTO: &str = r#"
syntax = "proto3";
package shop.v1;
import "google/protobuf/timestamp.proto";
enum Size { SIZE_UNSPECIFIED = 0; SMALL = 1; }
message Item { string name = 1; int64 count = 2; repeated Size sizes = 3; map<string, string> tags = 4;
  oneof pick { string code = 5; int32 number = 6; } google.protobuf.Timestamp added = 7; }
message ItemList { repeated Item items = 1; }
service Shop {
  rpc Get(Item) returns (Item);
  rpc Watch(Item) returns (stream Item);
}
"#;

    #[test]
    fn compiles_a_proto_file_and_lists_methods_with_templates() {
        let folder =
            std::env::temp_dir().join(format!("codestamp-proto-{}", crate::storage::db::new_id()));
        std::fs::create_dir_all(&folder).expect("folder");
        let path = folder.join("shop.proto");
        std::fs::write(&path, PROTO).expect("write");
        let pool = from_proto_file(&path).expect("pool");
        let services = services(&pool);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "shop.v1.Shop");
        let watch = &services[0].methods[1];
        assert_eq!(watch.full_name, "shop.v1.Shop/Watch");
        assert!(watch.server_streaming && !watch.client_streaming);
        let template: Value = serde_json::from_str(&services[0].methods[0].template).expect("json");
        assert_eq!(template["count"], "0");
        assert_eq!(template["sizes"], json!(["SIZE_UNSPECIFIED"]));
        assert_eq!(template["tags"], json!({}));
        assert_eq!(template["added"], "1970-01-01T00:00:00Z");
        assert!(template.get("code").is_some() && template.get("number").is_none());
        let mut compiler = protox::Compiler::new([&folder]).expect("compiler");
        compiler.open_file("shop.proto").expect("open");
        let mut served: HashMap<String, FileDescriptorProto> = compiler
            .file_descriptor_set()
            .file
            .into_iter()
            .map(|file| (file.name().to_owned(), file))
            .collect();
        assert_eq!(served.len(), 1);
        assert!(
            pool_from_files(served.clone()).is_ok(),
            "well-known file filled in"
        );
        // protobufjs servers send no imports at all.
        for file in served.values_mut() {
            file.dependency.clear();
        }
        // ...and write type names relative to where they are used.
        let mut relative = served.clone();
        for message in relative
            .values_mut()
            .flat_map(|file| &mut file.message_type)
        {
            for field in &mut message.field {
                if let Some(name) = field.type_name.as_mut() {
                    *name = name
                        .strip_prefix(".shop.v1.")
                        .or_else(|| name.strip_prefix('.'))
                        .unwrap_or(name)
                        .to_owned();
                }
            }
        }
        assert!(pool_from_files(relative).is_ok(), "relative names resolved");
        let pool = pool_from_files(served).expect("imports repaired");
        assert_eq!(super::services(&pool).len(), 1);

        std::fs::write(
            folder.join("parts.proto"),
            "syntax = \"proto3\"; package parts; message Part { string id = 1; }",
        )
        .expect("write");
        std::fs::write(
            folder.join("boxes.proto"),
            "syntax = \"proto3\"; package boxes; import \"parts.proto\";
             message Box { parts.Part part = 1; }
             service Boxes { rpc Get(Box) returns (parts.Part); }",
        )
        .expect("write");
        let mut compiler = protox::Compiler::new([&folder]).expect("compiler");
        compiler.include_imports(true);
        compiler.open_file("boxes.proto").expect("open");
        let mut merged: HashMap<String, FileDescriptorProto> = compiler
            .file_descriptor_set()
            .file
            .into_iter()
            .map(|mut file| {
                file.dependency.clear();
                (file.name().to_owned(), file)
            })
            .collect();
        assert_eq!(merged.len(), 2);
        repair_imports(&mut merged);
        assert_eq!(merged["boxes.proto"].dependency, ["parts.proto"]);
        assert!(missing_references(&merged).is_empty());
        assert!(pool_from_files(merged).is_ok());
        std::fs::write(&path, "syntax = \"proto3\"; message {").expect("write");
        assert!(from_proto_file(&path).is_err());
        let _ = std::fs::remove_dir_all(folder);
    }
}
