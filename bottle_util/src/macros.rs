#[macro_export]
macro_rules! opt {
    (, $default:ident) => {
        $default
    };
    ($optional:expr, $default:ident) => {
        $optional
    };
}

#[macro_export]
macro_rules! params_internal {
    ($vec:ident, required, $key:expr, $val:expr) => {
        $vec.push(($key, $val.to_string()));
    };
    ($vec:ident, optional, $key:expr, $val:expr) => {
        if let Some(ref v) = $val {
            $vec.push(($key, v.to_string()));
        }
    };
    ($vec:ident, repeated, $key:expr, $val:expr) => {
        for (i, v) in $val.iter().enumerate() {
            $vec.push((format!("{}[{}]", $key, i), v.to_string()));
        }
    };
}

/// The macros are used to more conveniently build request params for API endpoints.
/// Inspired by https://nullderef.com/blog/web-api-client/
/// The main one is `build_params!`. Example:
/// ```
/// let params = build_params!{
///     required foo => foo,
///     optional bar => bar,
///     repeated baz => baz,
/// };
#[macro_export]
macro_rules! build_params {
    (
        $(
            $kind:ident $name:ident $( => $val:expr )?
        ),+ $(,)?
    ) => {
        {
            let mut params = Vec::new();
            $(
                $crate::params_internal!(
                    params,
                    $kind,
                    stringify!($name).to_string(),
                    $crate::opt!($( $val )?, $name)
                );
            )+
            params
        }
    };
}
