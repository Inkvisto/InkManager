//! The HTTP request method
//!
//! This module contains HTTP-method related structs and errors and such. The
//! main type of this module, `Method`, is also reexported at the root of the
//! crate as `http::Method` and is intended for import through that location
//! primarily.

#[derive(Clone, PartialEq, Eq, Hash)]
enum Method {
    Options,
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
    // If the extension is short enough, store it inline
    ExtensionInline(InlineExtension),
    // Otherwise, allocate it
    ExtensionAllocated(AllocatedExtension),
}
