//! FORK: per-shortcut forced transcription language.
//!
//! The "Dictate in English" / "Dictate in French" shortcuts pin the language
//! of the recording they start, whatever the global language setting says.
//! Models like Canary need to be told the source language; given the wrong
//! one they translate instead of transcribing. Pinning it per shortcut makes
//! each shortcut always produce text in its own language.
//!
//! The override is set when a recording starts (`TranscribeAction::start`),
//! read wherever the transcription pipeline reads settings, and cleared once
//! that recording's transcription finishes. Kept in its own module so the
//! upstream files only gain one-line hooks, which keeps merges easy.

use crate::settings::{AppSettings, LanguageDetectionMode};
use std::sync::Mutex;

static OVERRIDE: Mutex<Option<&'static str>> = Mutex::new(None);

/// Binding ids of the fork's per-language shortcuts, with their language.
pub const LANGUAGE_BINDINGS: &[(&str, &str)] =
    &[("transcribe_english", "en"), ("transcribe_french", "fr")];

/// Pin (or with `None`, unpin) the language for the recording being started.
pub fn set(language: Option<&'static str>) {
    *OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) = language;
}

/// Forget the pinned language once its recording has been transcribed.
pub fn clear() {
    set(None);
}

/// `settings` as the transcription pipeline should see them: with the pinned
/// language as the selected one, so the language plan uses it as-is instead
/// of auto-detecting.
pub fn apply(mut settings: AppSettings) -> AppSettings {
    if let Some(language) = *OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) {
        settings.selected_language = language.to_string();
        settings.translate_to_english = false;
        settings.language_detection = LanguageDetectionMode::WhenNeeded;
    }
    settings
}

/// Whether `id` is one of the fork's per-language shortcuts.
pub fn is_language_binding(id: &str) -> bool {
    LANGUAGE_BINDINGS.iter().any(|(binding, _)| *binding == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::get_default_settings;

    // One test, so the shared global isn't raced by parallel tests.
    #[test]
    fn pinned_language_overrides_and_clears() {
        let mut base = get_default_settings();
        base.selected_language = "auto".into();
        base.translate_to_english = true;

        clear();
        let untouched = apply(base.clone());
        assert_eq!(untouched.selected_language, "auto");
        assert!(untouched.translate_to_english);

        set(Some("fr"));
        let pinned = apply(base.clone());
        assert_eq!(pinned.selected_language, "fr");
        assert!(!pinned.translate_to_english);

        clear();
        assert_eq!(apply(base).selected_language, "auto");
        assert!(is_language_binding("transcribe_french"));
        assert!(!is_language_binding("transcribe"));
    }
}
