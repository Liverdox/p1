#[macro_export]
/// If the value is Ok or Some,
/// then the Result or Option will be returned from the entire calling function;
/// otherwise, it does nothing.
macro_rules! rev_qumark {
    ( $val:expr ) => {
        if $val.map_or(false, |_| true) {
            return $val;
        }
    };
}

#[macro_export]
macro_rules! vec_none {
    ( $len:expr ) => {
        {
            let mut vec = Vec::new();
            vec.resize_with($len, || None);
            vec
        }
    };
}