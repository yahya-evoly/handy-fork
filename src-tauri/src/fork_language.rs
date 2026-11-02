//! FORK: per-language dictation shortcuts.
//!
//! "Dictate in English" / "Dictate in French" force their language: the
//! recording is transcribed in the shortcut's language, as with a fixed
//! language setting. The one exception: when Handy's language detector is
//! very confident (`CONFIDENT_DETECTION`) the person spoke the *other* one,
//! the recording is transcribed in the language actually spoken and then
//! translated with Apple's on-device Translation framework (`fork_translate`),
//! which translates far better than the speech model does on its own. Short,
//! mixed or unclear speech keeps the forced language. If translation isn't
//! available, the text stays in the spoken language rather than coming out
//! garbled.
//!
//! The target is set when a recording starts (`TranscribeAction::start`), read
//! wherever the transcription pipeline reads settings, and cleared once that
//! recording's transcription is finished (`finish`). Kept in its own module so
//! upstream files only gain one-line hooks, which keeps merges easy.

use crate::language_detection::{LanguageDecision, LanguageSource};
use crate::managers::model::canonical_language_code;
use crate::managers::transcription::Transcription;
use crate::settings::{AppSettings, LanguageDetectionMode};
use log::{info, warn};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

static OVERRIDE: Mutex<Option<&'static str>> = Mutex::new(None);

/// Binding ids of the fork's per-language shortcuts, with their language.
pub const LANGUAGE_BINDINGS: &[(&str, &str)] =
    &[("transcribe_english", "en"), ("transcribe_french", "fr")];

/// The languages these shortcuts choose between.
const LANGUAGES: [&str; 2] = ["en", "fr"];

/// Detector probability (over English vs French) above which a recording on a
/// forced shortcut counts as spoken in the other language and gets translated.
const CONFIDENT_DETECTION: f32 = 0.97;

fn current() -> Option<&'static str> {
    *OVERRIDE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Pin (or with `None`, unpin) the output language for the recording being
/// started.
pub fn set(language: Option<&'static str>) {
    *OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) = language;
}

/// A recording is starting: pin its output language, tell the overlay which
/// mode it is in ("en", "fr" or "auto") so the recording pill can show it, and
/// warm up the translator so a translation after the recording is quick.
pub fn start_recording(app: &AppHandle, language: Option<&'static str>) {
    set(language);
    let _ = app.emit_to(
        "recording_overlay",
        "fork-dictation-language",
        language.unwrap_or("auto"),
    );
    if let Some(target) = language {
        let source = other_language(target);
        std::thread::spawn(
            move || match crate::fork_translate::translate("ok", source, target) {
                Some(_) => info!("Translator ready ({source} -> {target})"),
                None => warn!("Translator unavailable ({source} -> {target})"),
            },
        );
    }
}

/// Forget the pinned language once its recording has been transcribed.
pub fn clear() {
    set(None);
}

/// `settings` as the transcription pipeline should see them. With a pinned
/// language, the run goes through Handy's detector (English vs French, for any
/// model), listing the pinned language first so it's used whenever detection
/// can't run; `gate_detection` then keeps the pinned language unless the
/// detector is very confident.
pub fn apply(mut settings: AppSettings) -> AppSettings {
    if let Some(target) = current() {
        settings.selected_language = "auto".to_string();
        settings.spoken_languages = vec![target.to_string(), other_language(target).to_string()];
        settings.translate_to_english = false;
        settings.language_detection = LanguageDetectionMode::Always;
    }
    settings
}

/// Hook in `language_detection::Plan::decide` for a detected language. On a
/// forced shortcut, a detection of the other language only stands when the
/// detector's top probability reaches `CONFIDENT_DETECTION`; otherwise the
/// recording is transcribed in the shortcut's language.
pub fn gate_detection(
    decision: LanguageDecision,
    candidates: &[(String, f32)],
) -> LanguageDecision {
    let Some(target) = current() else {
        return decision;
    };
    gate(target, decision, candidates)
}

fn gate(
    target: &str,
    decision: LanguageDecision,
    candidates: &[(String, f32)],
) -> LanguageDecision {
    let spoken = decision.language.as_deref().map(canonical_language_code);
    if spoken == Some(target) {
        return decision;
    }
    let confidence = candidates.first().map_or(0.0, |(_, p)| *p);
    if confidence.is_finite() && confidence >= CONFIDENT_DETECTION {
        info!(
            "Forced '{target}' shortcut: confidently spoken in {spoken:?} ({:.1}%), will translate",
            confidence * 100.0
        );
        return decision;
    }
    info!(
        "Forced '{target}' shortcut: {spoken:?} only {:.1}% likely, keeping '{target}'",
        confidence * 100.0
    );
    LanguageDecision {
        language: Some(target.to_string()),
        source: LanguageSource::Selected,
    }
}

/// Called with a recording's transcription: translates it into the pinned
/// language when it was spoken in the other one, then unpins the language.
pub fn finish(result: anyhow::Result<Transcription>) -> anyhow::Result<Transcription> {
    let target = current();
    clear();
    let Some(target) = target else {
        return result;
    };
    let mut transcription = result?;
    let Some(source) = translation_source(target, transcription.language.as_ref()) else {
        return Ok(transcription);
    };
    if transcription.text.trim().is_empty() {
        return Ok(transcription);
    }
    match crate::fork_translate::translate(&transcription.text, source, target) {
        Some(translated) => {
            info!("Translated dictation from '{source}' to '{target}'");
            transcription.text = translated;
        }
        None => {
            warn!("Translation '{source}' -> '{target}' unavailable; keeping the text as spoken")
        }
    }
    Ok(transcription)
}

/// The language to translate from, when the recording was spoken in a
/// language other than `target`.
fn translation_source(target: &str, decision: Option<&LanguageDecision>) -> Option<&'static str> {
    let decision = decision?;
    // Only a confident detection by Handy's detector (see `gate`) counts.
    if decision.source != LanguageSource::Detected {
        return None;
    }
    let spoken = canonical_language_code(decision.language.as_deref()?);
    LANGUAGES
        .into_iter()
        .find(|language| *language == spoken && *language != target)
}

fn other_language(language: &str) -> &'static str {
    if language == "fr" {
        "en"
    } else {
        "fr"
    }
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
    fn pinned_language_detects_between_both_and_clears() {
        let mut base = get_default_settings();
        base.selected_language = "de".into();
        base.translate_to_english = true;

        clear();
        let untouched = apply(base.clone());
        assert_eq!(untouched.selected_language, "de");
        assert!(untouched.translate_to_english);

        set(Some("fr"));
        let pinned = apply(base.clone());
        assert_eq!(pinned.selected_language, "auto");
        assert_eq!(pinned.spoken_languages, vec!["fr", "en"]);
        assert!(!pinned.translate_to_english);
        assert_eq!(pinned.language_detection, LanguageDetectionMode::Always);

        clear();
        assert_eq!(apply(base).selected_language, "de");
        assert!(is_language_binding("transcribe_french"));
        assert!(!is_language_binding("transcribe"));
    }

    fn decision(language: &str, source: LanguageSource) -> LanguageDecision {
        LanguageDecision {
            language: Some(language.to_string()),
            source,
        }
    }

    #[test]
    fn translates_only_detected_other_language() {
        let detected_fr = decision("fr", LanguageSource::Detected);
        assert_eq!(translation_source("en", Some(&detected_fr)), Some("fr"));
        assert_eq!(translation_source("fr", Some(&detected_fr)), None);

        // A guess isn't evidence the person spoke the other language.
        let fallback_fr = decision("fr", LanguageSource::Fallback);
        assert_eq!(translation_source("en", Some(&fallback_fr)), None);

        // Only English <-> French.
        let detected_de = decision("de", LanguageSource::Detected);
        assert_eq!(translation_source("en", Some(&detected_de)), None);
        assert_eq!(translation_source("en", None), None);
    }

    fn ranked(top: &str, p: f32) -> Vec<(String, f32)> {
        let other = other_language(top).to_string();
        vec![(top.to_string(), p), (other, 1.0 - p)]
    }

    #[test]
    fn keeps_forced_language_unless_very_confident() {
        let detected_fr = || decision("fr", LanguageSource::Detected);

        // Unsure the English shortcut was spoken in French: stays English.
        let kept = gate("en", detected_fr(), &ranked("fr", 0.80));
        assert_eq!(kept, decision("en", LanguageSource::Selected));
        assert_eq!(translation_source("en", Some(&kept)), None);

        // Very confident: transcribe as French, then translate to English.
        let confident = gate("en", detected_fr(), &ranked("fr", 0.99));
        assert_eq!(confident, detected_fr());
        assert_eq!(translation_source("en", Some(&confident)), Some("fr"));

        // Detected the shortcut's own language: nothing to do.
        assert_eq!(
            gate("fr", detected_fr(), &ranked("fr", 0.60)),
            detected_fr()
        );
    }
}
