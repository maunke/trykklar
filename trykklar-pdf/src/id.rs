//! Object ID Macro
macro_rules! object_id {
    ($name:ident) => {
        /// Unique ID containing the document object id.
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(lopdf::ObjectId);

        impl $name {
            #[allow(dead_code)]
            pub(crate) fn new(id: lopdf::ObjectId) -> Self {
                Self(id)
            }

            /// Returns the corresponding document object id.
            #[allow(dead_code)]
            pub fn get(self) -> lopdf::ObjectId {
                self.0
            }
        }
    };
}

pub(crate) use object_id;
