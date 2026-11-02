// FORK: on-device translation through Apple's Translation framework, used when
// the dictated language differs from a per-language shortcut's language
// (see src-tauri/src/fork_language.rs). Compiled by build_fork.rs.
import Dispatch
import Foundation
#if canImport(Translation)
import Translation
#endif

private final class ResultBox: @unchecked Sendable {
    var text: String?
}

/// Translates `text` from `source` to `target` (language codes such as "en",
/// "fr"). Returns a malloc'd C string the caller frees with
/// `fork_free_translation`, or NULL when translation isn't possible (older
/// macOS, language pair not installed, an error, or no answer within
/// `timeoutSeconds`).
///
/// Call it off the main thread: the Translation framework does work on the
/// main run loop, so blocking the main thread here would deadlock until the
/// timeout. In Handy it runs on a transcription worker thread.
@_cdecl("fork_translate_text")
public func forkTranslateText(
    _ text: UnsafePointer<CChar>,
    _ source: UnsafePointer<CChar>,
    _ target: UnsafePointer<CChar>
) -> UnsafeMutablePointer<CChar>? {
    #if canImport(Translation)
    guard #available(macOS 26.0, *) else { return nil }
    let input = String(cString: text)
    let sourceLanguage = Locale.Language(identifier: String(cString: source))
    let targetLanguage = Locale.Language(identifier: String(cString: target))
    let box = ResultBox()
    let done = DispatchSemaphore(value: 0)
    Task.detached {
        defer { done.signal() }
        let status = await LanguageAvailability().status(from: sourceLanguage, to: targetLanguage)
        guard status == .installed else { return }
        do {
            let session = TranslationSession(installedSource: sourceLanguage, target: targetLanguage)
            box.text = try await session.translate(input).targetText
        } catch {
            box.text = nil
        }
    }
    // Never let a stuck translation hold up dictation.
    let timeoutSeconds = 6.0
    if done.wait(timeout: .now() + timeoutSeconds) == .timedOut { return nil }
    guard let translated = box.text else { return nil }
    return strdup(translated)
    #else
    return nil
    #endif
}

@_cdecl("fork_free_translation")
public func forkFreeTranslation(_ pointer: UnsafeMutablePointer<CChar>?) {
    free(pointer)
}
