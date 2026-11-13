//! Language of user-facing backend messages (errors, tray menu). English by default;
//! the UI calls `set_language` on start and whenever the user switches language.

use std::sync::atomic::{AtomicBool, Ordering};

static TURKISH: AtomicBool = AtomicBool::new(false);

pub fn set_turkish(turkish: bool) {
    TURKISH.store(turkish, Ordering::Relaxed);
}

pub fn is_turkish() -> bool {
    TURKISH.load(Ordering::Relaxed)
}

/// `t!("English {x}", "Türkçe {x}")` formats the message in the current language.
#[macro_export]
macro_rules! t {
    ($en:literal, $tr:literal) => {
        if $crate::i18n::is_turkish() {
            format!($tr)
        } else {
            format!($en)
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn switches_language_and_captures_arguments() {
        let n = 3;
        super::set_turkish(true);
        assert_eq!(t!("{n} files", "{n} dosya"), "3 dosya");
        super::set_turkish(false);
        assert_eq!(t!("{n} files", "{n} dosya"), "3 files");
    }
}
