macro_rules! error {
    ($code: expr, $($t:tt)*) => {
        ::tide::Error::from_str(($code as u16).try_into().unwrap(), format!($($t)*));
    };
}

macro_rules! bail {
    ($code: expr, $($t:tt)*) => {
        return Err(error!($code, $($t)*));
    };
}

macro_rules! ensure {
    ($cond: expr, $code: expr, $($t:tt)*) => {
        if !($cond) {
            bail!($code, $($t)*);
        }
    };
}
