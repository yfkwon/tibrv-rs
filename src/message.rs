//! Interfaces for creating and using Rendezvous Messages

use std;
use tibrv_sys::*;
use field::*;
use std::mem;
use std::marker::PhantomData;
use std::ffi::CString;

/// A struct representing an owned Rendezvous Message.
///
/// The memory allocated to this type of Message is the responsibility
/// of the Rust application. When this type is dropped,
/// `tibrvMsg_Destroy` will be run to free any memory allocated
/// to store the message.
pub struct Msg {
    pub inner: tibrvMsg,
}

impl Msg {
    /// Construct a new owned Rendezvous Message
    pub fn new() -> Result<Self, &'static str> {
        let mut ptr: tibrvMsg = unsafe { mem::zeroed() };
        let result = unsafe { tibrvMsg_Create(&mut ptr) };
        match result {
            tibrv_status::TIBRV_OK => Ok(Msg { inner: ptr }),
            _ => Err("Error!"),
        }
    }

    pub fn try_clone(&self) -> Result<Self, &'static str> {
        let mut ptr: tibrvMsg = unsafe { mem::zeroed() };
        let result = unsafe { tibrvMsg_CreateCopy(self.inner, &mut ptr) };
        match result {
            tibrv_status::TIBRV_OK => Ok(Msg { inner: ptr }),
            _ => Err("Error!"),
        }
    }

    /// Add a `MsgField` to this message.
    ///
    /// The referenced field must be marked mutable, as although the
    /// process should not mutate the field, the C library makes no
    /// guarantee.
    ///
    /// The contents of message fields are always copied, therefore
    /// slice types must be `Copy`. A borrowed `MsgField` does not need
    /// to live beyond the point where it is added to the `Msg`.
    pub fn add_field(&mut self,
                     field: &mut MsgField)
                     -> Result<&mut Self, &'static str> {
        match unsafe { tibrvMsg_AddField(self.inner, &mut field.inner) } {
            tibrv_status::TIBRV_OK => Ok(self),
            _ => Err("Bork!"),
        }
    }

    /// Get a specified field from this message.
    ///
    /// Data in scalar fields is copied, and data in pointer fields
    /// is guaranteed to live at least as long as the parent `Msg`.
    ///
    /// This variant retrieves the field by name.
    pub fn get_field_by_name<'a>
        (&'a self,
         name: &str)
         -> Result<BorrowedMsgField<'a>, &'static str> {
        self.get_field(Some(name), None)
    }

    /// Get a specified field from this message.
    ///
    /// Data in scalar fields is copied, and data in pointer fields
    /// is guaranteed to live at least as long as the parent `Msg`.
    ///
    /// This variant retrieves the field by id.
    pub fn get_field_by_id<'a>
        (&'a self,
         id: u32)
         -> Result<BorrowedMsgField<'a>, &'static str> {
        self.get_field(None, Some(id))
    }

    fn get_field<'a>(&'a self,
                     name: Option<&str>,
                     id: Option<u32>)
                     -> Result<BorrowedMsgField<'a>, &'static str> {
        assert!(name.is_some() != id.is_some(),
                "One of id or name must be provided.");
        let mut field: tibrvMsgField = unsafe { mem::zeroed() };
        let field_name = CString::new(name.unwrap_or("")).unwrap();
        let name_ptr = name.map_or(std::ptr::null(), |_| field_name.as_ptr());
        let result = unsafe {
            tibrvMsg_GetFieldEx(self.inner,
                                name_ptr,
                                &mut field,
                                id.unwrap_or(0) as tibrv_u16)
        };

        match result {
            tibrv_status::TIBRV_OK => {
                Ok(BorrowedMsgField {
                    inner: MsgField {
                        name: field_name,
                        inner: field,
                    },
                    phantom: PhantomData,
                })
            }
            _ => Err("Bork!"),
        }

    }

    /// Remove a specified field from this message.
    ///
    /// Data in scalar fields is copied, and data in pointer fields
    /// is guaranteed to live at least as long as the parent `Msg`.
    ///
    /// This variant retrieves the field by name.
    pub fn remove_field_by_name(&self, name: &str) -> Result<(), &'static str> {
        self.remove_field(Some(name), None)
    }

    /// Remove a specified field from this message.
    ///
    /// Data in scalar fields is copied, and data in pointer fields
    /// is guaranteed to live at least as long as the parent `Msg`.
    ///
    /// This variant retrieves the field by id.
    pub fn remove_field_by_id(&self, id: u32) -> Result<(), &'static str> {
        self.remove_field(None, Some(id))
    }

    fn remove_field(&self,
                    name: Option<&str>,
                    id: Option<u32>)
                    -> Result<(), &'static str> {
        assert!(name.is_some() != id.is_some(),
                "One of id or name must be provided.");
        let field_name = CString::new(name.unwrap_or("")).unwrap();
        let name_ptr = name.map_or(std::ptr::null(), |_| field_name.as_ptr());
        let result = unsafe {
            tibrvMsg_RemoveFieldEx(self.inner, name_ptr, id.unwrap_or(0) as u16)
        };

        match result {
            tibrv_status::TIBRV_OK => Ok(()),
            _ => Err("Bork!"),
        }
    }

    /// Get the number of fields within this message.
    pub fn num_fields(&mut self) -> Result<u32, &'static str> {
        let mut ptr: tibrv_u32 = unsafe { mem::zeroed() };
        let result = unsafe { tibrvMsg_GetNumFields(self.inner, &mut ptr) };
        match result {
            tibrv_status::TIBRV_OK => Ok(ptr as u32),
            _ => Err("Bork!"),
        }
    }

    /// Expand the internal storage of a message.
    ///
    /// Messages automatically expand when adding a field would
    /// overflow the available space, however if adding a large
    /// number of fields it may be useful to preallocate enough
    /// space to hold them all.
    pub fn expand(&mut self, amount: i32) -> Result<&mut Self, &'static str> {
        match unsafe { tibrvMsg_Expand(self.inner, amount as tibrv_i32) } {
            tibrv_status::TIBRV_OK => Ok(self),
            _ => Err("Bork!"),
        }
    }

    /// Get the size of the message (in bytes).
    ///
    /// Does not include space allocated but not yet used.
    pub fn byte_size(&self) -> Result<u32, &'static str> {
        let mut ptr: tibrv_u32 = unsafe { mem::zeroed() };
        match unsafe { tibrvMsg_GetByteSize(self.inner, &mut ptr) } {
            tibrv_status::TIBRV_OK => Ok(ptr as u32),
            _ => Err("Bork!"),
        }
    }
}

// Ensure we clean up messages we're responsible for.
impl Drop for Msg {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                tibrvMsg_Destroy(self.inner);
            }
        }
    }
}

/// A struct representing a borrowed Rendezvous Message.
///
/// The memory referenced by this type of Message is assumed to be
/// the responsibility of the Rendezvous C library, and will not be
/// freed when the `BorrowedMsg` is dropped.
pub struct BorrowedMsg {
    pub inner: tibrvMsg,
}

impl BorrowedMsg {
    /// Transform a BorrowedMsg into an owned Msg.
    ///
    /// Copies all data within the fields of the message, does not include
    /// any supplementary information attached to the message.
    ///
    /// This function is effectively an allocate and copy.
    pub fn to_owned(&self) -> Result<Msg, &'static str> {
        let mut ptr: tibrvMsg = unsafe { mem::zeroed() };
        let result = unsafe { tibrvMsg_CreateCopy(self.inner, &mut ptr) };
        match result {
            tibrv_status::TIBRV_OK => Ok(Msg { inner: ptr }),
            _ => Err("Error!"),
        }
    }

    /// Detach an inbound message from Rendezvous storage.
    ///
    /// This function is unsafe, as it is only valid for messages
    /// received in a callback invoked from Rendezvous.
    pub unsafe fn detach(self) -> Result<Msg, &'static str> {
        let ptr = self.inner;
        let result = tibrvMsg_Detach(ptr);
        match result {
            tibrv_status::TIBRV_OK => Ok(Msg { inner: ptr }),
            _ => Err("Error!"),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn create_msg() {
        let msg = Msg::new();
        assert!(msg.is_ok());
    }

    #[test]
    fn add_remove_fields() {
        let data = CString::new("A string").unwrap();
        let cstr = data.as_c_str();
        let mut field = Builder::new(&cstr)
            .with_name("StringField")
            .encode();

        let slice: &[u16] = &[1, 2, 3, 4];
        let mut field2 = Builder::new(&slice)
            .with_name("Uint16 field")
            .with_id(2)
            .encode();

        let mut msg = Msg::new().unwrap();
        assert!(msg.add_field(&mut field)
            .unwrap()
            .add_field(&mut field2)
            .is_ok());

        assert_eq!(2, msg.num_fields().unwrap());

        assert!(msg.remove_field_by_id(2).is_ok());
        assert_eq!(1, msg.num_fields().unwrap());

        assert!(msg.remove_field_by_name("StringField").is_ok());
        assert_eq!(0, msg.num_fields().unwrap());
    }

    #[test]
    fn copy_msg() {
        let mut msg = Msg::new().unwrap();
        let slice: &[u16] = &[1, 2, 3, 4];
        let mut field = Builder::new(&slice)
            .with_name("Uint16 field")
            .with_id(2)
            .encode();
        let _ = msg.add_field(&mut field);
        let copy = msg.try_clone().unwrap();
        assert!(msg.inner != copy.inner); // Msgs are behind different pointers
        assert_eq!(34, msg.byte_size().unwrap()); // Both Msgs are populated
        assert_eq!(34, msg.byte_size().unwrap());
    }

    #[test]
    fn roundtrip_slice_msg() {
        let mut msg = Msg::new().unwrap();
        let slice: &[u16] = &[5, 4, 3, 2, 1];
        let mut field = Builder::new(&slice)
            .with_name("slice")
            .encode();
        let _ = msg.add_field(&mut field);
        let extracted = msg.get_field_by_name("slice").unwrap();

        let decoded = <&[u16]>::tibrv_try_decode(&extracted).unwrap();
        assert_eq!(5, decoded[0]);
    }

    #[test]
    fn roundtrip_string_msg() {
        use std::ffi::CStr;

        let mut msg = Msg::new().unwrap();
        let data = CString::new("Hello world!").unwrap();
        let mut field = Builder::new(&data.as_c_str())
            .with_name("string")
            .encode();
        let _ = msg.add_field(&mut field).unwrap();
        let extracted = msg.get_field_by_name("string").unwrap();
        let decoded = <&CStr>::tibrv_try_decode(&extracted).unwrap();
        assert_eq!(decoded, data.as_c_str());
    }

    #[test]
    fn empty_size() {
        let msg = Msg::new().unwrap();
        assert_eq!(8, msg.byte_size().unwrap());
    }
}