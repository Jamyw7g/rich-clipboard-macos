#[macro_use]
extern crate objc;
use std::cell::RefCell;
use std::ffi::c_void;
use std::os::raw::c_long;

use objc::runtime::{Object, NO};
use objc_foundation::{object_struct, NSArray, NSData, NSString};
use objc_foundation::{INSData, INSString};
use objc_id::Id;

#[allow(non_upper_case_globals)]
const NSUTF8StringEncoding: u8 = 4;
type NSPasteboardType = *mut NSString;

object_struct!(NSPasteboard);

#[allow(improper_ctypes)]
#[link(name = "AppKit", kind = "framework")]
extern "C" {
    static NSPasteboardTypeTIFF: NSPasteboardType;
    static NSPasteboardTypePNG: NSPasteboardType;
    static NSPasteboardTypePDF: NSPasteboardType;
    static NSPasteboardTypeHTML: NSPasteboardType;
    static NSPasteboardTypeRTF: NSPasteboardType;
    static NSPasteboardTypeTabularText: NSPasteboardType;
    static NSPasteboardTypeString: NSPasteboardType;
    static NSPasteboardTypeFileURL: NSPasteboardType;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    TIFF,
    PNG,
    PDF,
    HTML,
    RTF,
    TabularText,
    String,
    FileUrl,
    Other,
}

#[derive(Debug, Clone)]
pub enum Content {
    Data(Box<[u8]>),
    String(Box<str>),
}

impl From<NSPasteboardType> for Type {
    fn from(ty: NSPasteboardType) -> Self {
        unsafe {
            if msg_send![ty, isEqualToString: NSPasteboardTypeTIFF] {
                Self::TIFF
            } else if msg_send![ty, isEqualToString: NSPasteboardTypePNG] {
                Self::PNG
            } else if msg_send![ty, isEqualToString: NSPasteboardTypePDF] {
                Self::PDF
            } else if msg_send![ty, isEqualToString: NSPasteboardTypeHTML] {
                Self::HTML
            } else if msg_send![ty, isEqualToString: NSPasteboardTypeRTF] {
                Self::RTF
            } else if msg_send![ty, isEqualToString: NSPasteboardTypeTabularText] {
                Self::TabularText
            } else if msg_send![ty, isEqualToString: NSPasteboardTypeString] {
                Self::String
            } else if msg_send![ty, isEqualToString: NSPasteboardTypeFileURL] {
                Self::FileUrl
            } else {
                Self::Other
            }
        }
    }
}

impl From<Type> for NSPasteboardType {
    fn from(ty: Type) -> Self {
        unsafe {
            match ty {
                Type::FileUrl => NSPasteboardTypeFileURL,
                Type::HTML => NSPasteboardTypeHTML,
                Type::PDF => NSPasteboardTypePDF,
                Type::PNG => NSPasteboardTypePNG,
                Type::RTF => NSPasteboardTypeRTF,
                Type::String => NSPasteboardTypeString,
                Type::TIFF => NSPasteboardTypeTIFF,
                Type::TabularText => NSPasteboardTypeTabularText,
                _ => unimplemented!(),
            }
        }
    }
}

type Error = Box<dyn std::error::Error>;

#[derive(Debug)]
pub struct PasteBoard {
    board: Id<NSPasteboard>,
    change_count: RefCell<c_long>,
}

impl PasteBoard {
    pub fn new() -> Result<Self, Error> {
        unsafe {
            let cls = class!(NSPasteboard);
            let board: *mut NSPasteboard = msg_send![cls, generalPasteboard];
            if board.is_null() {
                return Err("Can't get generalPasteboard".into());
            }
            let board = Id::from_ptr(board);
            Ok(Self {
                board,
                change_count: RefCell::new(0),
            })
        }
    }

    pub fn get_contents(&self, ty: Type, newer: bool) -> Result<Content, Error> {
        unsafe {
            let change_count: c_long = msg_send![self.board, changeCount];
            if newer && change_count == *self.change_count.borrow() {
                return Err("There is no newer content to get.".into());
            } else {
                *self.change_count.borrow_mut() = change_count;
            }
            let content = match ty {
                Type::TIFF | Type::PNG | Type::PDF => {
                    let data: Id<NSData> = Id::from_ptr(msg_send![
                        self.board,
                        dataForType: NSPasteboardType::from(ty)
                    ]);
                    Content::Data(data.bytes().to_vec().into_boxed_slice())
                }
                Type::FileUrl | Type::HTML | Type::RTF | Type::String | Type::TabularText => {
                    let string: Id<NSString> = Id::from_ptr(msg_send![
                        self.board,
                        stringForType: NSPasteboardType::from(ty)
                    ]);
                    Content::String(string.as_str().to_string().into_boxed_str())
                }
                _ => return Err("Unsupport other type at now".into()),
            };
            Ok(content)
        }
    }

    pub fn write_contents(&self, content: Content, ty: Type) -> Result<(), Error> {
        unsafe {
            match content {
                Content::Data(data) => {
                    let nsdata_cls = class!(NSData);
                    let data: *mut NSData = msg_send![nsdata_cls, dataWithBytesNoCopy: (data.as_ptr() as *const c_void) 
                                                                               length: data.len() 
                                                                         freeWhenDone: NO];
                    if data.is_null() {
                        return Err("Fail to init NSData".into());
                    }
                    let data: Id<NSData> = Id::from_ptr(data);
                    let _: c_long = msg_send![self.board, clearContents];
                    if msg_send![self.board, setData: &*data forType: NSPasteboardType::from(ty)] {
                        Ok(())
                    } else {
                        Err("Fail to setcontent to clipboard.".into())
                    }
                }
                Content::String(string) => {
                    let nsstring_cls = class!(NSString);
                    let nsstring_instance: *mut Object = msg_send![nsstring_cls, alloc];
                    let string: *mut NSString = msg_send![nsstring_instance, initWithBytesNoCopy: (string.as_ptr() as *const c_void) 
                                                                                          length: string.len() 
                                                                                        encoding: NSUTF8StringEncoding 
                                                                                    freeWhenDone: NO];
                    if string.is_null() {
                        return Err("Fail to init NSString".into());
                    }
                    let string: Id<NSString> = Id::from_ptr(string);
                    let _: c_long = msg_send![self.board, clearContents];
                    if msg_send![self.board, setString: &*string forType: NSPasteboardType::from(ty)] {
                        Ok(())
                    } else {
                        Err("Fail to setcontent to clipboard.".into())
                    }
                }
            }
        }
    }

    pub fn types(&self) -> Result<Vec<Type>, Error> {
        unsafe {
            let types: Id<NSArray<NSPasteboardType>> = Id::from_ptr(msg_send![self.board, types]);
            let types = (0u64..msg_send![types, count])
                .filter_map(|idx| {
                    let ty: NSPasteboardType = msg_send![types, objectAtIndex: idx];
                    let ty = Type::from(ty);
                    if ty == Type::Other {
                        None
                    } else {
                        Some(ty)
                    }
                })
                .collect();
            Ok(types)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_link() {
        unsafe {
            let mut static_data = vec![
                NSPasteboardTypeTIFF, NSPasteboardTypePNG, NSPasteboardTypePDF,
                NSPasteboardTypeHTML, NSPasteboardTypeRTF, NSPasteboardTypeTabularText,
                NSPasteboardTypeString, NSPasteboardTypeFileURL
            ];
            static_data.drain(..).for_each(|instance| {
                let str_ptr: *const i8 = msg_send![instance, UTF8String];
                assert!(!str_ptr.is_null());
            })
        }
    }

    #[test]
    fn pasteboard() {
        let ori = "Hello world".to_string().into_boxed_str();
        let content = Content::String(ori.clone());

        let board = PasteBoard::new().unwrap();
        board.write_contents(content.clone(), Type::String).unwrap();

        let types = board.types().unwrap();
        assert!(types.contains(&Type::String));
        let res = board.get_contents(Type::String, true).unwrap();
        if let Content::String(val) = res {
            assert_eq!(val, ori);
        } else {
            panic!("Get incorrect value.");
        }

        let ori = b"Hello world".to_vec().into_boxed_slice();
        let content = Content::Data(ori.clone());
        board.write_contents(content, Type::PDF).unwrap();

        let types = board.types().unwrap();
        assert!(types.contains(&Type::PDF));
        let res = board.get_contents(Type::PDF, true).unwrap();
        if let Content::Data(val) = res {
            assert_eq!(val, ori);
        } else {
            panic!("Get incorrect value.");
        }
    }
}
