#[macro_export]
macro_rules! gen_signer_seeds_two {
    (
    $seed: expr, $first_key: expr, $second_key: expr, $bump: expr
) => {
        &[&[$seed, $first_key.as_ref(), $second_key.as_ref(), &[$bump]]]
    };
}

#[macro_export]
macro_rules! gen_signer_seeds {
    (
    $seed: expr, $first_key: expr, $bump: expr
) => {
        &[$seed as &[u8], $first_key.as_ref(), &[$bump]]
    };
}

#[cfg(target_os = "solana")]
#[macro_export]
macro_rules! xmsg {
    ($($arg:tt)*) => {{
        ::anchor_lang::solana_program::log::sol_log(&format!($($arg)*));
    }};
}

#[cfg(not(target_os = "solana"))]
#[macro_export]
macro_rules! xmsg {
    ($($arg:tt)*) => {{
        println!($($arg)*);
    }};
}

#[cfg(not(target_os = "solana"))]
#[macro_export]
macro_rules! dbg_msg {
    // NOTE: We cannot use `concat!` to make a static string as a format argument
    // of `eprintln!` because `file!` could contain a `{` or
    // `$val` expression could be a block (`{ .. }`), in which case the `msg!`
    // will be malformed.
    () => {
        msg!("[{}:{}]", file!(), line!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                msg!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg_msg!($val)),+,)
    };
}

#[cfg(target_os = "solana")]
#[macro_export]
macro_rules! dbg_msg {
    // NOTE: We cannot use `concat!` to make a static string as a format argument
    // of `eprintln!` because `file!` could contain a `{` or
    // `$val` expression could be a block (`{ .. }`), in which case the `msg!`
    // will be malformed.
    () => {
        println!("[{}:{}]", file!(), line!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                println!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg_msg!($val)),+,)
    };
}

#[macro_export]
macro_rules! require_msg {
    ($invariant:expr, $error:expr $(,)?, $message: expr) => {
        if !($invariant) {
            msg!($message);
            return Err(anchor_lang::error!($error));
        }
    };
}

#[macro_export]
macro_rules! arrform {
    ($size:expr, $($arg:tt)*) => {{
        let mut af = arrform::ArrForm::<$size>::new();

        af.format(format_args!($($arg)*)).unwrap_or_else(|_| {
            <arrform::ArrForm<$size> as ::std::fmt::Write>::write_str(&mut af, "Buffer overflow").unwrap();
        });
        af
    }}
}

/// Log a formatted message with automatic capacity estimation
/// Uses arrform! and msg! together for efficient logging
/// Capacity is conservatively estimated based on format string length
#[macro_export]
macro_rules! kmsg {
    // For formats without arguments
    ($fmt:expr) => {{
        // Choose capacity tier based on format string length
        match $fmt.len() {
            0..=50 => {
                let formatted = $crate::arrform!{250, $fmt};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            },
            51..=100 => {
                let formatted = $crate::arrform!{400, $fmt};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            },
            101..=200 => {
                let formatted = $crate::arrform!{700, $fmt};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            },
            _ => {
                let formatted = $crate::arrform!{1300, $fmt};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            }
        }
    }};

    // For formats with arguments
    ($fmt:expr, $($arg:expr),+) => {{
        // Choose capacity tier based on format string length
        // This is very conservative to avoid overflows
        match $fmt.len() {
            0..=50 => {
                let formatted = $crate::arrform!{150, $fmt, $($arg),+};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            },
            51..=100 => {
                let formatted = $crate::arrform!{300, $fmt, $($arg),+};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            },
            101..=200 => {
                let formatted = $crate::arrform!{600, $fmt, $($arg),+};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            },
            _ => {
                let formatted = $crate::arrform!{1200, $fmt, $($arg),+};
                anchor_lang::prelude::msg!("{}", formatted.as_str());
            }
        }
    }};
}

/// Same as kmsg! but allows specifying a custom capacity
#[macro_export]
macro_rules! kmsg_sized {
    ($capacity:expr, $fmt:expr) => {{
        let formatted = $crate::arrform!{$capacity, $fmt};
        anchor_lang::prelude::msg!("{}", formatted.as_str());
    }};
    ($capacity:expr, $fmt:expr, $($arg:expr),+) => {{
        let formatted = $crate::arrform!($capacity, $fmt, $($arg),+);
        anchor_lang::prelude::msg!("{}", formatted.as_str());
    }};
}
