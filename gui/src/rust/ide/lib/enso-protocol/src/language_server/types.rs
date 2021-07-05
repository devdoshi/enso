//! This module contains language server types.

use super::*;



// =============
// === Event ===
// =============

/// Event emitted by the Language Server `Client`.
pub type Event = json_rpc::handler::Event<Notification>;



// ============
// === Path ===
// ============

/// A path is a representation of a path relative to a specified content root.
#[derive(Clone,Debug,Serialize,Deserialize,Hash,PartialEq,Eq)]
#[serde(rename_all="camelCase")]
pub struct Path {
    /// Path's root id.
    pub root_id:Uuid,
    /// Path's segments.
    pub segments:Vec<String>,

}

impl From<&FileSystemObject> for Path {
    fn from(file_system_object:&FileSystemObject) -> Path {
        match file_system_object {
            FileSystemObject::Directory{name,path}          => path.append_im(name),
            FileSystemObject::File{name,path}               => path.append_im(name),
            FileSystemObject::DirectoryTruncated{name,path} => path.append_im(name),
            FileSystemObject::Other{name,path}              => path.append_im(name),
            FileSystemObject::SymlinkLoop{name,path,..}     => path.append_im(name)
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "//{}/", self.root_id)?;
        write!(f, "{}", self.segments.join("/"))
    }
}

impl Path {
    /// Splits path into name and path to parent directory. e.g.:
    /// Path{root_id,segments:["foo","bar","qux"]} => ("qux",Path{root_id,segments:["foo","bar"]})
    pub fn split(mut self) -> Option<(Path,String)> {
        self.segments.pop().map(|name| (self,name))
    }

    /// Creates a new clone appending a new `segment`.
    pub fn append_im(&self, segment:impl Str) -> Self {
        let mut clone = self.clone();
        clone.segments.push(segment.into());
        clone
    }

    /// Returns the parent `Path` if the current `Path` is not `root`.
    pub fn parent(&self) -> Option<Self> {
        let mut parent = self.clone();
        parent.segments.pop().map(|_| parent)
    }

    /// Returns the file name, i.e. the last segment if exists.
    pub fn file_name(&self) -> Option<&String> {
        self.segments.last()
    }

    /// Returns the file extension, i.e. the part of last path segment after the last dot.
    /// Returns `None` is there is no segments or no dot in the last segment.
    pub fn extension(&self) -> Option<&str> {
        let name           = self.file_name()?;
        let last_dot_index = name.rfind('.')?;
        Some(&name[last_dot_index + 1..])
    }

    /// Returns the stem of filename, i.e. part of last segment without extension if present.
    pub fn file_stem(&self) -> Option<&str> {
        let name        = self.file_name()?;
        let name_length = name.rfind('.').unwrap_or_else(|| name.len());
        Some(&name[..name_length])
    }

    /// Constructs a new path from given root ID and segments.
    pub fn new(root_id:Uuid, segments:impl IntoIterator<Item:AsRef<str>>) -> Path {
        Path {
            root_id,
            segments : segments.into_iter().map(|s| s.as_ref().into()).collect()
        }
    }

    /// Constructs a new path containing a content root only.
    pub fn new_root(root_id:Uuid) -> Path {
        let segments = default();
        Path {root_id,segments}
    }
}



// ====================
// === Notification ===
// ====================

/// Notification generated by the Language Server.
#[derive(Clone,Debug,PartialEq)]
#[derive(Serialize,Deserialize)]
#[serde(tag="method", content="params")]
pub enum Notification {
    /// Filesystem event occurred for a watched path.
    #[serde(rename = "file/event")]
    FileEvent(FileEvent),

    /// Sent from the server to the client to inform about new information for certain expressions
    /// becoming available. This notification is superseded by executionContext/expressionUpdates.
    #[serde(rename = "executionContext/expressionValuesComputed")]
    ExpressionValuesComputed(serde_json::Value),

    /// Sent from the server to the client to inform about new information for certain expressions
    /// becoming available.
    #[serde(rename = "executionContext/expressionUpdates")]
    ExpressionUpdates(ExpressionUpdates),

    /// Sent from the server to the client to inform about a failure during execution of an
    /// execution context.
    #[serde(rename = "executionContext/executionFailed")]
    ExecutionFailed(ExecutionFailed),

    /// Sent from the server to the client to inform about a status of execution.
    #[serde(rename = "executionContext/executionStatus")]
    ExecutionStatus(ExecutionStatus),

    /// Sent from server to the client to inform abouth the change in the suggestions database.
    #[serde(rename = "search/suggestionsDatabaseUpdates")]
    SuggestionDatabaseUpdates(SuggestionDatabaseUpdatesEvent),
}

/// Sent from the server to the client to inform about a failure during execution of an execution
/// context.
#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
#[serde(rename_all="camelCase")]
pub struct ExecutionFailed {
    pub context_id : ContextId,
    pub message    : String,
}



// =======================
// === ExecutionUpdate ===
// =======================

/// Sent from the server to the client to inform about new information for certain expressions
/// becoming available.
#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
#[serde(rename_all="camelCase")]
pub struct ExpressionUpdates {
    pub context_id : ContextId,
    pub updates    : Vec<ExpressionUpdate>,
}

/// An update about the computed expression.
#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
#[serde(rename_all="camelCase")]
pub struct ExpressionUpdate {
    pub expression_id  : ExpressionId,
    #[serde(rename = "type")] // To avoid collision with the `type` keyword.
    pub typename       : Option<String>,
    pub method_pointer : Option<SuggestionId>,
    pub profiling_info : Vec<ProfilingInfo>,
    pub from_cache     : bool,
    pub payload        : ExpressionUpdatePayload
}

/// Profiling information on an executed expression. It is implemented as a union as additional
/// types of information will be added in the future.
#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
// Not sure what the future variants will be, and implementing Copy is not essential for this.
#[allow(missing_copy_implementations)]
pub enum ProfilingInfo {
    #[serde(rename_all="camelCase")]
    ExecutionTime {
        nano_time : u64,
    }
}

#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
#[serde(tag="type")]
pub enum ExpressionUpdatePayload {
    Value,
    #[serde(rename_all="camelCase")]
    DataflowError {
        trace : Vec<ExpressionId>
    },
    #[serde(rename_all="camelCase")]
    Panic {
        message : String,
        trace   : Vec<ExpressionId>,
    },
}



// =======================
// === ExecutionStatus ===
// =======================

/// Sent from the server to the client to inform about a status of execution.
#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
#[serde(rename_all="camelCase")]
pub struct ExecutionStatus {
    pub context_id  : ContextId,
    pub diagnostics : Vec<Diagnostic>,
}

/// The type of diagnostic message.
#[derive(Clone,Copy,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
pub enum DiagnosticType {Error,Warning}

/// A diagnostic object is produced as a result of an execution attempt, like pushing the method
/// pointer to a call stack, or editing the file. It can represent a compiler warning, a compilation
/// error, or a runtime error. The message has optional path, location and stack fields containing
/// information about the location in the source code.
//
// In case of the runtime errors, the path and location fields may be empty if the error happens in
// a builtin node. Then, to locate the error in the code, you can use the stack field with a stack
// trace to find the first element with non-empty location (as the head of the stack will point to
// the builtin element).
#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
#[serde(rename_all="camelCase")]
pub struct Diagnostic {
    kind          : DiagnosticType,
    message       : String,
    path          : Option<Path>,
    location      : Option<TextRange>,
    expression_id : Option<ExpressionId>,
    stack         : Vec<StackTraceElement>,
}

/// The frame of the stack trace. If the error refer to a builtin node, the path and location fields
/// will be empty.
#[derive(Clone,Debug,Deserialize,PartialEq,Serialize)]
#[allow(missing_docs)]
#[serde(rename_all="camelCase")]
pub struct StackTraceElement {
    function_name : String,
    path          : Option<Path>,
    location      : Option<TextRange>,
}



// =================
// === FileEvent ===
// =================

/// The `file/event` notification parameters.
#[derive(Clone,Debug,PartialEq)]
#[derive(Serialize,Deserialize)]
#[allow(missing_docs)]
pub struct FileEvent {
    pub path : Path,
    pub kind : FileEventKind,
}

/// Describes kind of filesystem event (was the file created or deleted, etc.)
#[derive(Clone,Copy,Debug,PartialEq)]
#[derive(Serialize,Deserialize)]
#[allow(missing_docs)]
pub enum FileEventKind {
    Added,
    Removed,
    Modified,
}



// ======================
// === FileAttributes ===
// ======================

/// Attributes of the file in the filesystem.
#[derive(Clone,Debug,PartialEq,Eq,Hash)]
#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct FileAttributes {
    /// When the file was created.
    pub creation_time:UTCDateTime,
    /// When the file was last accessed.
    pub last_access_time:UTCDateTime,
    /// When the file was last modified.
    pub last_modified_time:UTCDateTime,
    /// What kind of file is this.
    pub kind:FileSystemObject,
    /// Size of the file in bytes.
    /// (size of files not being `RegularFile`s is unspecified).
    pub byte_size:u64,
}

/// A representation of what kind of type a filesystem object can be.
#[derive(Hash,Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(tag = "type")]
#[allow(missing_docs)]
pub enum FileSystemObject {
    Directory {
        name:String,
        path:Path,
    },
    /// A directory which contents have been truncated, i.e. with its subtree not listed
    /// any further due to depth limit being reached.
    // FIXME: To be clarified in https://github.com/enso-org/enso/issues/708
    DirectoryTruncated {
        name:String,
        path:Path,
    },
    File {
        name:String,
        path:Path,
    },
    /// Represents other, potenatially unrecognized object. Example is a broken symbolic link.
    // FIXME: To be clarified in https://github.com/enso-org/enso/issues/708
    Other {
        name:String,
        path:Path,
    },
    /// Represents a symbolic link that creates a loop.
    SymlinkLoop {
        name:String,
        path:Path,
        /// A target of the symlink. Since it is a loop, target is a subpath of the symlink.
        target: Path,
    }
}

impl FileSystemObject {
    /// Creates a new Directory variant.
    pub fn new_directory(path:Path) -> Option<Self> {
        path.split().map(|(path,name)| Self::Directory{name,path})
    }

    /// Creates a new DirectoryTruncated variant.
    pub fn new_directory_truncated(path:Path) -> Option<Self> {
        path.split().map(|(path,name)| Self::DirectoryTruncated{name,path})
    }

    /// Creates a new File variant.
    pub fn new_file(path:Path) -> Option<Self> {
        path.split().map(|(path,name)| Self::File{name,path})
    }

    /// Creates a new Other variant.
    pub fn new_other(path:Path) -> Option<Self> {
        path.split().map(|(path,name)| Self::Other{name,path})
    }

    /// Creates a new SymlinkLoop variant.
    pub fn new_symlink_loop(path:Path,target:Path) -> Option<Self> {
        path.split().map(|(path,name)| Self::SymlinkLoop{name,path,target})
    }

    /// Take the name of this file system object, consuming self.
    pub fn take_name(self) -> String {
        match self {
            FileSystemObject::Directory          {name,..} => name,
            FileSystemObject::DirectoryTruncated {name,..} => name,
            FileSystemObject::File               {name,..} => name,
            FileSystemObject::Other              {name,..} => name,
            FileSystemObject::SymlinkLoop        {name,..} => name,
        }
    }
}



// =====================
// === Content Roots ===
// =====================

/// A content root represents a location on a real file-system that has been virtualized for use in
/// the Cloud.
#[allow(missing_docs)]
#[derive(Clone,Debug,Deserialize,Eq,Hash,PartialEq,Serialize)]
#[serde(tag="type")]
pub enum ContentRoot {
    /// Points to the project home.
    #[serde(rename_all="camelCase")]
    Project {
        id : Uuid,
    },
    /// This content root points to the system root (`/`) on unix systems, or to a drive root on
    /// Windows. In Windows' case, there may be multiple `Root` entries corresponding to the various
    /// drives.
    #[serde(rename_all="camelCase")]
    FileSystemRoot {
        id   : Uuid,
        path : String,
    },
    /// The user's home directory
    #[serde(rename_all="camelCase")]
    Home {
        id : Uuid,
    },
    /// An Enso library location.
    #[serde(rename_all="camelCase")]
    Library {
        id        : Uuid,
        namespace : String,
        name      : String,
        version   : String,
    },
    /// A content root that has been added by the IDE.
    #[serde(rename_all="camelCase")]
    Custom {
        id : Uuid,
    }
}

impl ContentRoot {
    pub fn id(&self) -> Uuid {
        match self {
            ContentRoot::Project        {id   } => *id,
            ContentRoot::FileSystemRoot {id,..} => *id,
            ContentRoot::Home           {id   } => *id,
            ContentRoot::Library        {id,..} => *id,
            ContentRoot::Custom         {id   } => *id,
        }
    }
}



// ================
// === Position ===
// ================

/// A representation of a position in a text file.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Position {
    pub line      : usize,
    pub character : usize
}

impls!{ From + &From <enso_data::text::TextLocation> for Position { |location|
    Position {
        line      : location.line,
        character : location.column,
    }
}}

impls!{ From + &From <Position> for enso_data::text::TextLocation { |position|
    enso_data::text::TextLocation {
        line      : position.line,
        column    : position.character,
    }
}}



// =================
// === TextRange ===
// =================

/// A representation of a range of text in a text file.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TextRange {
    pub start: Position,
    pub end: Position
}

impls!{ From + &From <Range<enso_data::text::TextLocation>> for TextRange { |range|
    TextRange {
        start : range.start.into(),
        end   : range.end.into(),
    }
}}

impls!{ From + &From <TextRange> for Range<enso_data::text::TextLocation> { |range|
    range.start.into()..range.end.into()
}}



// ================
// === TextEdit ===
// ================

/// A representation of a change to a text file at a given position.
#[derive(Hash, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct TextEdit {
    pub range: TextRange,
    pub text: String
}

impl TextEdit {
    /// Compute an edit that represents the difference between the two given strings based on their
    /// common pre- and postfix. This is an approximation of the diff between the two strings that
    /// assumes that anythign between the common prefix and the common post-fix has changed.
    ///
    /// Example:
    /// ```
    /// # use enso_protocol::language_server::{TextEdit, Position, TextRange};
    /// # use enso_data::text::TextLocation;
    /// let source     = "\n333<->🌊12345\n";
    /// let target     = "\n333x🔥12345\n";
    /// let diff       = TextEdit::from_prefix_postfix_differences(source,target);
    /// let edit_range = TextRange {
    ///     start : Position{line:1, character:3},
    ///     end   : Position{line:1, character:7},
    /// };
    /// assert_eq!(diff, TextEdit{range:edit_range, text:"x🔥".to_string()});
    ///
    /// let source     = "1\n2\n";
    /// let target     = "1\n3\n2\n";
    /// let diff       = TextEdit::from_prefix_postfix_differences(source,target);
    /// let edit_range = TextRange {
    ///     start : Position{line:1, character:0},
    ///     end   : Position{line:1, character:0},
    /// };
    /// assert_eq!(diff, TextEdit{range:edit_range, text:"3\n".to_string()});
    ///
    /// let source     = "1\n2\n3\n";
    /// let target     = "1\n3\n";
    /// let diff       = TextEdit::from_prefix_postfix_differences(source,target);
    /// let edit_range = TextRange {
    ///     start : Position{line:1, character:0},
    ///     end   : Position{line:2, character:0},
    /// };
    /// assert_eq!(diff, TextEdit{range:edit_range, text:"".to_string()});
    /// ```
    pub fn from_prefix_postfix_differences(source:&str, target:&str) -> TextEdit {
        use enso_data::text::Index;
        use enso_data::text::TextLocation;

        let source_length = source.chars().count();
        let target_length = target.chars().count();

        let common_prefix_length  = utils::string::common_prefix_length(source, target);
        let common_postfix_length = utils::string::common_postfix_length(source, target);
        let common_parts_length   = common_prefix_length + common_postfix_length;
        let overlaping_chars      = common_parts_length.saturating_sub(source_length.min(target_length));
        let prefix_length         = common_prefix_length;
        let postfix_length        = common_postfix_length - overlaping_chars;

        let source_start_index = Index::new(prefix_length);
        let source_end_index   = Index::new(source_length - postfix_length);

        let source_start_position = TextLocation::from_index(source,source_start_index);
        let source_end_position   = TextLocation::from_index(source,source_end_index);
        let source_text_range     = source_start_position..source_end_position;

        let target_range = prefix_length..(target_length - postfix_length);
        let target_text  = target.chars().skip(target_range.start).take(target_range.len());

        TextEdit {
            range : source_text_range.into(),
            text  : target_text.collect()
        }
    }

    /// Return the edit moved by the given number of lines.
    ///
    /// Example:
    /// ```
    /// # use enso_protocol::language_server::{Position, TextRange, TextEdit};
    /// let start = Position{line:0, character:4};
    /// let end   = Position{line:23, character:7};
    /// let range = TextRange{start,end};
    /// let edit  = TextEdit{range,text:"Answer".to_string()};
    ///
    /// let moved = edit.clone().move_by_lines(42);
    ///
    /// assert_eq!(moved.range.start.line, 42);
    /// assert_eq!(moved.range.end.line, 42+23);
    /// assert_eq!(moved.range.start.character, edit.range.start.character);
    /// assert_eq!(moved.range.end.character, edit.range.end.character);
    /// ```
    pub fn move_by_lines(self, lines:usize) -> TextEdit {
        let TextEdit{range,text} = self;
        let TextRange{start,end} = range;
        let start = Position{line:start.line+lines, character:start.character};
        let end   = Position{line:end.line+lines, character:end.character};
        let range = TextRange{start,end};
        TextEdit{range,text}
    }
}



// ================
// === FileEdit ===
// ================

/// A versioned representation of batch edits to a file.
#[derive(Hash, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct FileEdit {
    pub path: Path,
    pub edits: Vec<TextEdit>,
    pub old_version: Sha3_224,
    pub new_version: Sha3_224
}


// ========================
// === ExecutionContext ===
// ========================

/// Execution context ID.
pub type ContextId = Uuid;

/// Execution context expression ID.
pub type ExpressionId = Uuid;

/// A configuration object for properties of the visualisation.
#[derive(Clone,Debug,Deserialize,Eq,Hash,PartialEq,Serialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct VisualisationConfiguration {
    #[allow(missing_docs)]
    pub execution_context_id: ContextId,
    /// A qualified name of the module containing the expression which creates visualisation.
    pub visualisation_module: String,
    /// An enso lambda that will transform the data into expected format, i.e. `a -> a.json`.
    pub expression: String,
}

/// Used to enter deeper in the execution context stack. In general, all consequent stack items
/// should be `LocalCall`s.
#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct LocalCall {
    pub expression_id:ExpressionId
}

/// Points to a method definition.
#[derive(Hash,Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct MethodPointer {
    /// The fully qualified module name.
    pub module          : String,
    pub defined_on_type : String,
    pub name            : String
}

/// Used for entering a method. The first item on the execution context stack should always be
/// an `ExplicitCall`.
#[derive(Hash, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct ExplicitCall {
    pub method_pointer: MethodPointer,
    pub this_argument_expression: Option<String>,
    pub positional_arguments_expressions: Vec<String>
}

/// A representation of an executable position in code, used by the context execution methods.
#[derive(Hash, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(missing_docs)]
pub enum StackItem {
    ExplicitCall(ExplicitCall),
    LocalCall(LocalCall)
}


// ==============================
// === CapabilityRegistration ===
// ==============================

/// `CapabilityRegistration` is used to keep track of permissions granting.
#[derive(Hash, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct CapabilityRegistration {
    /// Method is the name of the capability listed in
    /// https://github.com/enso-org/enso/blob/main/docs/language-server/protocol-language-server.md#capabilities
    pub method: String,
    /// One of the enumerated `RegisterOptions` depending of `method`.
    pub register_options: RegisterOptions
}

impl CapabilityRegistration {
    /// Create "file/receivesTreeUpdates" capability for path.
    pub fn create_receives_tree_updates(path:Path) -> Self {
        let method           = "file/receivesTreeUpdates".to_string();
        let register_options = RegisterOptions::Path {path};
        CapabilityRegistration {method,register_options}
    }

    /// Create "text/canEdit" capability for path.
    pub fn create_can_edit_text_file(path:Path) -> Self {
        let method           = "text/canEdit".to_string();
        let register_options = RegisterOptions::Path {path};
        CapabilityRegistration {method,register_options}
    }

    /// Create "executionContext/canModify" capability for path.
    pub fn create_can_modify_execution_context(context_id:Uuid) -> Self {
        let method = "executionContext/canModify".to_string();
        let register_options = RegisterOptions::ExecutionContextId {context_id};
        CapabilityRegistration {method,register_options}
    }

    /// Create "executionContext/receivesUpdates" capability for path.
    pub fn create_receives_execution_context_updates(context_id:Uuid) -> Self {
        let method = "executionContext/receivesUpdates".to_string();
        let register_options = RegisterOptions::ExecutionContextId {context_id};
        CapabilityRegistration {method,register_options}
    }

    /// Create "search/receivesSuggestionsDatabaseUpdates" capability
    pub fn create_receives_suggestions_database_updates() -> Self {
        let method           = "search/receivesSuggestionsDatabaseUpdates".to_string();
        let register_options = RegisterOptions::None {};
        CapabilityRegistration {method,register_options}
    }
}


// =======================
// === RegisterOptions ===
// =======================

/// `capability/acquire` takes method and options specific to the method. This type represents the
/// options. The used variant must match the method. See for details:
/// https://github.com/enso-org/enso/blob/main/docs/language-server/protocol-language-server.md#capabilities
//TODO[ao] we cannot have one variant for each cabability due to `untagged` attribute.
// The best solution is make CapabilityRegistration an enum and write serialization and
// deserialization by hand.
#[derive(Hash, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
#[allow(missing_docs)]
pub enum RegisterOptions {
    Path {path:Path},
    #[serde(rename_all="camelCase")]
    ExecutionContextId {context_id:ContextId},
    None {},
}


// ===========================
// === Suggestion Database ===
// ===========================

/// The identifier of SuggestionEntry in SuggestionDatabase.
pub type SuggestionId = usize;

/// The version of Suggestion Database.
pub type SuggestionsDatabaseVersion = usize;

/// The argument of an atom, method or function suggestion.
#[derive(Hash, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct SuggestionEntryArgument {
    /// The argument name.
    pub name:String,
    /// The argument type. String 'Builtins.Main.Any' is used to specify generic types.
    pub repr_type:String,
    /// Indicates whether the argument is lazy.
    pub is_suspended:bool,
    /// Flag indicating that the argument has default value
    ///
    /// Note: this is obviously redundant, however it is part of the API. It will be removed as
    /// a part of https://github.com/enso-org/enso/issues/1293
    pub has_default:bool,
    /// Optional default value.
    pub default_value:Option<String>,
}

/// The definition scope. The start and end are chars indices.
#[derive(Hash,Debug,Copy,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct SuggestionEntryScope {
    pub start : Position,
    pub end   : Position,
}

impls!{ From + &From <RangeInclusive<enso_data::text::TextLocation>> for SuggestionEntryScope { |range|
    SuggestionEntryScope {
        start : range.start().into(),
        end   : range.end().into(),
    }
}}

impls!{ From + &From <SuggestionEntryScope> for RangeInclusive<enso_data::text::TextLocation> { |this|
    this.start.into()..=this.end.into()
}}

/// A type of suggestion entry.
#[derive(Hash,Debug,Copy,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub enum SuggestionEntryType {Atom,Method,Function,Local}

/// A Suggestion Entry.
#[derive(Hash, Debug, Clone, PartialEq, Eq,Serialize,Deserialize)]
#[allow(missing_docs)]
#[serde(tag="type")]
#[serde(rename_all="camelCase")]
pub enum SuggestionEntry {
    #[serde(rename_all="camelCase")]
    Atom {
        external_id   : Option<Uuid>,
        name          : String,
        module        : String,
        arguments     : Vec<SuggestionEntryArgument>,
        return_type   : String,
        documentation : Option<String>,
    },
    #[serde(rename_all="camelCase")]
    Method {
        external_id   : Option<Uuid>,
        name          : String,
        module        : String,
        arguments     : Vec<SuggestionEntryArgument>,
        self_type     : String,
        return_type   : String,
        documentation : Option<String>,
    },
    #[serde(rename_all="camelCase")]
    Function {
        external_id : Option<Uuid>,
        name        : String,
        module      : String,
        arguments   : Vec<SuggestionEntryArgument>,
        return_type : String,
        scope       : SuggestionEntryScope,
    },
    #[serde(rename_all="camelCase")]
    Local {
        external_id : Option<Uuid>,
        name        : String,
        module      : String,
        return_type : String,
        scope       : SuggestionEntryScope,
    },
}

impl SuggestionEntry {
    /// Get name of the suggested entity.
    pub fn name(&self) -> &String {
        match self {
            Self::Atom     {name,..} => name,
            Self::Function {name,..} => name,
            Self::Local    {name,..} => name,
            Self::Method   {name,..} => name,
        }
    }
}

/// The entry in the suggestions database.
#[derive(Clone,Debug,Deserialize,Eq,Hash,PartialEq,Serialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct SuggestionsDatabaseEntry {
    pub id         : SuggestionId,
    pub suggestion : SuggestionEntry,
}


// === Suggestion Database Updates ===

#[derive(Clone,Copy,Debug,Deserialize,Eq,Hash,PartialEq,Serialize)]
#[allow(missing_docs)]
pub enum FieldAction {Remove,Set}

#[derive(Clone,Copy,Debug,Deserialize,Eq,Hash,PartialEq,Serialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct FieldUpdate<T> {
    pub tag   : FieldAction,
    pub value : Option<T>,
}

impl<T> FieldUpdate<T> {
    /// Create a field update with `Set` tag.
    pub fn set(value:T) -> Self {
        FieldUpdate {
            tag   : FieldAction::Set,
            value : Some(value)
        }
    }

    /// Create a field update with `Remove` tag.
    pub fn remove() -> Self {
        FieldUpdate {
            tag   : FieldAction::Remove,
            value : None,
        }
    }

    /// Maps the field update by applying `f` on the underlying value.
    pub fn map<U>(self, f:impl Fn(T) -> U) -> FieldUpdate<U> {
        FieldUpdate {
            tag   : self.tag,
            value : self.value.map(f),
        }
    }

    /// Maps the field update by applying `f` on the underlying value, if `f` returns [`Ok`].
    /// Otherwise returns the error returned by `f`.
    pub fn try_map<U,E>
    (self, f:impl Fn(T) -> std::result::Result<U,E>) -> std::result::Result<FieldUpdate<U>,E> {
        Ok(FieldUpdate {
            tag   : self.tag,
            value : self.value.map(f).transpose()?,
        })
    }
}

#[derive(Clone,Debug,Deserialize,Eq,Hash,PartialEq,Serialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub enum SuggestionArgumentUpdate {
    #[serde(rename_all="camelCase")]
    Add {
        index    : usize,
        argument : SuggestionEntryArgument,
    },
    #[serde(rename_all="camelCase")]
    Remove {
        index : usize,
    },
    #[serde(rename_all="camelCase")]
    Modify {
        index         : usize,
        name          : Option<FieldUpdate<String>>,
        repr_type     : Option<FieldUpdate<String>>,
        is_suspended  : Option<FieldUpdate<bool>>,
        has_default   : Option<FieldUpdate<bool>>,
        default_value : Option<FieldUpdate<String>>,
    }
}

/// The kind of the suggestions database update.
#[derive(Clone,Copy,Debug,Deserialize,Eq,Hash,PartialEq,Serialize)]
#[allow(missing_docs)]
pub enum SuggestionsDatabaseUpdateKind {Add,Update,Delete}

/// The update of the suggestions database.
#[derive(Hash,Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[allow(missing_docs)]
#[serde(tag="type")]
pub enum SuggestionsDatabaseUpdate {
    #[serde(rename_all="camelCase")]
    Add {
        id         : SuggestionId,
        suggestion : SuggestionEntry,
    },
    #[serde(rename_all="camelCase")]
    Remove {
        id : SuggestionId,
    },
    #[serde(rename_all="camelCase")]
    Modify {
        id            : SuggestionId,
        external_id   : Option<FieldUpdate<Uuid>>,
        #[serde(flatten)]
        modification  : Box<SuggestionsDatabaseModification>,
    }
}

/// The modification of suggestion database entry.
#[derive(Hash,Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[allow(missing_docs)]
#[serde(tag="type")]
pub struct SuggestionsDatabaseModification {
    #[serde(default)]
    pub arguments     : Vec<SuggestionArgumentUpdate>,
    pub module        : Option<FieldUpdate<String>>,
    pub self_type     : Option<FieldUpdate<String>>,
    pub return_type   : Option<FieldUpdate<String>>,
    pub documentation : Option<FieldUpdate<String>>,
    pub scope         : Option<FieldUpdate<SuggestionEntryScope>>,
}

/// Notification about change in the suggestions database.
#[derive(Hash,Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
#[allow(missing_docs)]
pub struct SuggestionDatabaseUpdatesEvent {
    pub updates         : Vec<SuggestionsDatabaseUpdate>,
    pub current_version : SuggestionsDatabaseVersion,
}

/// Utilities for testing code using the LS types.
pub mod test {
    use super::*;

    use crate::language_server::ExpressionId;

    /// Generate [`ExpressionUpdate`] with an update for a single expression bringing only the
    /// typename.
    pub fn value_update_with_type(id:ExpressionId, typename:impl Into<String>) -> ExpressionUpdate {
        ExpressionUpdate {
            expression_id  : id,
            typename       : Some(typename.into()),
            method_pointer : None,
            profiling_info : default(),
            from_cache     : false,
            payload        : ExpressionUpdatePayload::Value,
        }
    }

    /// Generate [`ExpressionUpdate`] with an update for a single expression bringing only the
    /// method pointer.
    pub fn value_update_with_method_ptr
    (id:ExpressionId, method_pointer:SuggestionId) -> ExpressionUpdate {
        ExpressionUpdate {
            expression_id  : id,
            typename       : None,
            method_pointer : Some(method_pointer),
            profiling_info : default(),
            from_cache     : false,
            payload        : ExpressionUpdatePayload::Value
        }
    }

    /// Generate [`ExpressionUpdate`] with an update for a single expression which resulted in
    /// a dataflow error.
    pub fn value_update_with_dataflow_error
    (id:ExpressionId) -> ExpressionUpdate {
        let trace = default();
        ExpressionUpdate {
            expression_id  : id,
            typename       : None,
            method_pointer : None,
            profiling_info : default(),
            from_cache     : false,
            payload        : ExpressionUpdatePayload::DataflowError {trace}
        }
    }

    /// Generate [`ExpressionUpdate`] with an update for a single expression which resulted in
    /// a dataflow panic.
    pub fn value_update_with_dataflow_panic
    (id:ExpressionId, message:impl Into<String>) -> ExpressionUpdate {
        let trace   = default();
        let message = message.into();
        ExpressionUpdate {
            expression_id  : id,
            typename       : None,
            method_pointer : None,
            profiling_info : default(),
            from_cache     : false,
            payload        : ExpressionUpdatePayload::Panic {trace,message}
        }
    }
}
