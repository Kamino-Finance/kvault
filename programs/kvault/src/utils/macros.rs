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

#[macro_export]
macro_rules! dbg_msg {
   
   
   
   
    () => {
        $crate::xmsg!("[{}:{}]", file!(), line!())
    };
    ($val:expr $(,)?) => {
       
       
        match $val {
            tmp => {
                $crate::xmsg!("[{}:{}] {} = {:#?}",
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




#[macro_export]
macro_rules! kmsg {
   
    ($fmt:expr) => {{
       
        match $fmt.len() {
            0..=50 => {
                let formatted = $crate::arrform!{250, $fmt};
                solana_program::log::sol_log(formatted.as_str());
            },
            51..=100 => {
                let formatted = $crate::arrform!{400, $fmt};
                solana_program::log::sol_log(formatted.as_str());
            },
            101..=200 => {
                let formatted = $crate::arrform!{700, $fmt};
                solana_program::log::sol_log(formatted.as_str());
            },
            _ => {
                let formatted = $crate::arrform!{1300, $fmt};
                solana_program::log::sol_log(formatted.as_str());
            }
        }
    }};

   
    ($fmt:expr, $($arg:expr),+) => {{
       
       
        match $fmt.len() {
            0..=50 => {
                let formatted = $crate::arrform!{150, $fmt, $($arg),+};
                solana_program::log::sol_log(formatted.as_str());
            },
            51..=100 => {
                let formatted = $crate::arrform!{300, $fmt, $($arg),+};
                solana_program::log::sol_log(formatted.as_str());
            },
            101..=200 => {
                let formatted = $crate::arrform!{600, $fmt, $($arg),+};
                solana_program::log::sol_log(formatted.as_str());
            },
            _ => {
                let formatted = $crate::arrform!{1200, $fmt, $($arg),+};
                solana_program::log::sol_log(formatted.as_str());
            }
        }
    }};
}


#[macro_export]
macro_rules! kmsg_sized {
    ($capacity:expr, $fmt:expr) => {{
        let formatted = $crate::arrform!{$capacity, $fmt};
        solana_program::log::sol_log(formatted.as_str());
    }};
    ($capacity:expr, $fmt:expr, $($arg:expr),+) => {{
        let formatted = $crate::arrform!($capacity, $fmt, $($arg),+);
        solana_program::log::sol_log(formatted.as_str());
    }};
}
