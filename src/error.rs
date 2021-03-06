//! Standard error type for ocl.
//!

use std::error::Error as StdError;
use num::FromPrimitive;
use ::{Status, EmptyInfoResult, OpenclVersion};

static SDK_DOCS_URL_PRE: &'static str = "https://www.khronos.org/registry/cl/sdk/1.2/docs/man/xhtml/";
static SDK_DOCS_URL_SUF: &'static str = ".html#errors";


fn fmt_status_desc(status: Status, fn_name: &'static str, fn_info: &str) -> String {
    let fn_info_string = if fn_info.is_empty() == false {
        format!("(\"{}\")", fn_info)
    } else {
        String::with_capacity(0)
    };

    format!("\n\n\
        ################################ OPENCL ERROR ############################### \
        \n\nError executing function: {}{}  \
        \n\nStatus error code: {:?} ({})  \
        \n\nPlease visit the following url for more information: \n\n{}{}{}  \n\n\
        ############################################################################# \n",
        fn_name, fn_info_string, status.clone(), status as i32,
        SDK_DOCS_URL_PRE, fn_name, SDK_DOCS_URL_SUF)
}


fn gen_status_error<S: Into<String>>(errcode: i32, fn_name: &'static str, fn_info: S) -> self::Error {
    let status = match Status::from_i32(errcode) {
        Some(s) => s,
        None => panic!("ocl_core::Error::err_status: Invalid error code: '{}'. Aborting.", errcode),
    };

    let fn_info = fn_info.into();
    let desc = fmt_status_desc(status.clone(), fn_name, &fn_info);
    let status_string = format!("{:?}", status);

    let kind = ErrorKind::Status {
            status: status,
            status_string: status_string,
            fn_name: fn_name,
            fn_info: fn_info,
            desc: desc
    };

    Error { kind, cause: None }
}


/// An enum containing either a `String` or one of several other error types.
///
/// Implements the usual error traits.
///
/// ## Stability
///
/// The `String` variant may eventually be removed. Many more variants and
/// sub-types will be added as time goes on and things stabilize.
///
/// `Status` will eventually be changed internally to contain a sub-error type
/// unique to each function which generates it (yeah that'll be fun to
/// implement).
///
/// `UnspecifiedDimensions` may be moved into a sub-type.
///
/// For now, don't assume the existence of or check for any of the above.
///
pub enum ErrorKind {
    Void,
    Conversion(String),
    Status {
        status: Status, status_string: String, fn_name: &'static str, fn_info: String, desc: String
    },
    String(String),
    Nul(::std::ffi::NulError),
    Io(::std::io::Error),
    FromUtf8Error(::std::string::FromUtf8Error),
    UnspecifiedDimensions,
    IntoStringError(::std::ffi::IntoStringError),
    EmptyInfoResult(EmptyInfoResult),
    VersionLow { detected: OpenclVersion, required: OpenclVersion },
    Other(Box<StdError>),
}


/// An Error.
pub struct Error {
    pub kind: ErrorKind,
    pub cause: Option<Box<self::Error>>,
}

impl self::Error {
    /// Returns a new `Error` with the description string: `desc`.
    ///
    /// ### Deprecated
    ///
    /// Use `::from` instead.
    //
    #[deprecated(since="0.4.0", note="Use `::from` instead.")]
    pub fn new<S: Into<String>>(desc: S) -> Self {
        Error { kind: self::ErrorKind::String(desc.into()), cause: None }
    }

    /// Returns a new `ErrorKind::String` with the given description.
    #[deprecated(since="0.4.0", note="Use `::from` instead.")]
    pub fn string<S: Into<String>>(desc: S) -> Self {
        Error { kind: self::ErrorKind::String(desc.into()), cause: None }
    }

    /// Returns an `Error` with the `UnspecifiedDimensions` kind variant.
    pub fn unspecified_dimensions() -> Error {
        Error { kind: ErrorKind::UnspecifiedDimensions, cause: None }
    }

    pub fn version_low(detected: OpenclVersion, required: OpenclVersion) -> Error {
        Error { kind: ErrorKind::VersionLow { detected, required }, cause: None }
    }

    /// Returns a new `ocl_core::Result::Err` containing an
    /// `ocl_core::ErrorKind::String` variant with the given description.
    ///
    /// ### Deprecated
    ///
    /// Use `::err_string` or `Err("...".into())` instead.
    //
    #[deprecated(since="0.4.0", note="Use `Err(\"...\".into())` instead.")]
    pub fn err<T, S: Into<String>>(desc: S) -> self::Result<T> {
        Err(Error { kind: ErrorKind::String(desc.into()), cause: None })
    }

    /// Returns a new `Err(ocl_core::ErrorKind::String(...))` variant with the
    /// given description.
    // #[deprecated(since="0.4.0", note="Use `Err(\"...\".into())` instead.")]
    pub fn err_string<T, S: Into<String>>(desc: S) -> self::Result<T> {
        Err(Error { kind: ErrorKind::String(desc.into()), cause: None })
    }

    /// Returns a new `ocl::Result::Err` containing an `ocl::Error` with the
    /// given error code and description.
    #[inline(always)]
    pub fn eval_errcode<T, S: Into<String>>(errcode: i32, result: T, fn_name: &'static str, fn_info: S)
            -> self::Result<T>
    {
        if (Status::CL_SUCCESS as i32) == errcode {
            Ok(result)
        } else {
            Err(gen_status_error(errcode, fn_name, fn_info))
        }
    }

    /// Returns a new `ocl::Result::Err` containing an
    /// `ocl::ErrorKind::Conversion` variant with the given description.
    pub fn err_conversion<T, S: Into<String>>(desc: S) -> self::Result<T> {
        Err(Error { kind: ErrorKind::Conversion(desc.into()), cause: None })
    }

    /// If this is a `String` variant, concatenate `txt` to the front of the
    /// contained string. Otherwise, do nothing at all.
    #[deprecated(since="0.6.0", note="Use `Err(\"...\".into())` instead.")]
    pub fn prepend<'s, S: AsRef<&'s str>>(&'s mut self, txt: S) {
        if let ErrorKind::String(ref mut string) = self.kind {
            string.reserve_exact(txt.as_ref().len());
            let old_string_copy = string.clone();
            string.clear();
            string.push_str(txt.as_ref());
            string.push_str(&old_string_copy);
        } else {
            panic!("Cannot prepend to a non-`String` error variant.");
        }
    }

    /// Creates a new error with this error as its cause.
    pub fn chain<E: Into<Error>>(self, err: E) -> Self {
        // let desc = format!("{}: {}", pre, self.description());
        let err = err.into();
        assert!(err.cause.is_none(), "Cannot chain an error that already has a cause.");
        Error { kind: err.kind, cause: Some(Box::new(self)) }
    }

    /// Returns the error status code for `Status` variants.
    pub fn status(&self) -> Option<Status> {
        match self.kind {
            ErrorKind::Status { ref status, .. } => Some(status.clone()),
            _ => None,
        }
    }

    /// Returns the error variant and contents.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Returns the immediate cause of this error (e.g. the next error in the
    /// chain).
    pub fn cause(&self) -> Option<&self::Error> {
        // match self.cause {
        //     Some(ref bc) => Some(&*bc),
        //     None => None,
        // }
        self.cause.as_ref().map(|c| &**c)
    }

    /// Writes the error message for this error to a formatter.
    fn write_msg(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            match self.kind {
                ErrorKind::VersionLow { detected, required } => write!(f, "OpenCL version too \
                    low to use this feature (detected: {}, required: {}).", detected, required),
                ErrorKind::Void => write!(f, "OpenCL Error"),
                ErrorKind::Conversion(ref desc) => write!(f, "{}", desc.as_str()),
                ErrorKind::Nul(ref err) => write!(f, "{}", err.description()),
                ErrorKind::Io(ref err) => write!(f, "{}", err.description()),
                ErrorKind::FromUtf8Error(ref err) => write!(f, "{}", err.description()),
                ErrorKind::IntoStringError(ref err) => write!(f, "{}", err.description()),
                ErrorKind::Status { ref desc, .. } => write!(f, "{}", desc),
                ErrorKind::String(ref desc) => write!(f, "{}", desc),
                ErrorKind::UnspecifiedDimensions => write!(f, "Cannot convert to a valid set of \
                    dimensions. Please specify some dimensions."),
                ErrorKind::EmptyInfoResult(ref err) => write!(f, "{}", err.description()),
                ErrorKind::Other(ref err) => write!(f, "{}", err.description()),
                // _ => f.write_str(self.description()),
            }
        }

    /// Writes the error message for this error and its cause to a formatter.
    fn _fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self.cause {
            Some(ref cause) => {
                self.write_msg(f)?;
                write!(f, ": ")?;
                cause._fmt(f)
            },
            None => self.write_msg(f)
        }
    }
}

impl ::std::fmt::Debug for self::Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        self._fmt(f)
    }
}

impl ::std::fmt::Display for self::Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        self._fmt(f)
    }
}

impl StdError for self::Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::Void => "OpenCL Error",
            ErrorKind::Conversion(ref desc) => desc.as_str(),
            ErrorKind::Nul(ref err) => err.description(),
            ErrorKind::Io(ref err) => err.description(),
            ErrorKind::FromUtf8Error(ref err) => err.description(),
            ErrorKind::IntoStringError(ref err) => err.description(),
            ErrorKind::Status { ref desc, .. } => desc.as_str(),
            ErrorKind::String(ref desc) => desc.as_str(),
            ErrorKind::UnspecifiedDimensions => "Cannot convert to a valid set of dimensions. \
                Please specify some dimensions.",
            ErrorKind::EmptyInfoResult(ref err) => err.description(),
            ErrorKind::VersionLow { .. } => "OpenCL version too low to use this feature.",
            ErrorKind::Other(ref err) => err.description(),
            // _ => panic!("OclErrorKind::description()"),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match self.cause {
            Some(ref bc) => Some(&*bc),
            None => None,
        }
    }
}

impl From<()> for self::Error {
    fn from(_: ()) -> Self {
        Error { kind: self::ErrorKind::Void, cause: None }
    }
}

impl From<EmptyInfoResult> for self::Error {
    fn from(err: EmptyInfoResult) -> Self {
        Error { kind: self::ErrorKind::EmptyInfoResult(err), cause: None }
    }
}

impl From<String> for self::Error {
    fn from(desc: String) -> Self {
        Error { kind: self::ErrorKind::String(desc), cause: None }
    }
}

impl From<self::Error> for String {
    fn from(err: self::Error) -> String {
        format!("{}", err)
    }
}

impl<'a> From<&'a str> for self::Error {
    fn from(desc: &'a str) -> Self {
        Error { kind: self::ErrorKind::String(String::from(desc)), cause: None }
    }
}

impl From<::std::ffi::NulError> for self::Error {
    fn from(err: ::std::ffi::NulError) -> Self {
        Error { kind: self::ErrorKind::Nul(err), cause: None }
    }
}

impl From<::std::io::Error> for self::Error {
    fn from(err: ::std::io::Error) -> Self {
        Error { kind: self::ErrorKind::Io(err), cause: None }
    }
}

impl From<::std::string::FromUtf8Error> for self::Error {
    fn from(err: ::std::string::FromUtf8Error) -> Self {
        Error { kind: self::ErrorKind::FromUtf8Error(err), cause: None }
    }
}

impl From<::std::ffi::IntoStringError> for self::Error {
    fn from(err: ::std::ffi::IntoStringError) -> Self {
        Error { kind: self::ErrorKind::IntoStringError(err), cause: None }
    }
}

unsafe impl ::std::marker::Send for self::Error {}


/// Ocl error result type.
pub type Result<T> = ::std::result::Result<T, self::Error>;

/// An chainable error.
pub trait ChainErr<T, E> {
    /// If the `Result` is an `Err` then `chain_err` evaluates the closure,
    /// which returns *some type that can be converted to `ErrorKind`*, boxes
    /// the original error to store as the cause, then returns a new error
    /// containing the original error.
    //
    // Blatantly ripped off from the `error-chain` crate.
    fn chain_err<F, IE>(self, callback: F) -> ::std::result::Result<T, Error>
        where F: FnOnce() -> IE, IE: Into<Error>;
}

impl<T> ChainErr<T, Error> for self::Result<T> {
    fn chain_err<F, E>(self, callback: F) -> self::Result<T>
            where F: FnOnce() -> E, E: Into<self::Error>
      {
        self.map_err(move |e| {
            let err = callback().into();
            assert!(err.cause.is_none());
            self::Error { kind: err.kind, cause: Some(Box::new(e)) }
        })
    }
}