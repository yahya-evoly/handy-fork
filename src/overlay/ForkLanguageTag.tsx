// FORK: shows which dictation mode the current recording uses (EN / FR / AUTO)
// next to the recording dot. The backend (src-tauri/src/fork_language.rs)
// announces the mode when a recording starts, before the overlay shows.
//
// The listener lives at module level, not in the component: the tag mounts
// only once the recording row renders, which can be after the announcement.
import { listen } from "@tauri-apps/api/event";
import React, { useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";

type DictationLanguage = "auto" | "en" | "fr";

let current: DictationLanguage = "auto";
const subscribers = new Set<() => void>();

listen<DictationLanguage>("fork-dictation-language", (event) => {
  current = event.payload;
  subscribers.forEach((notify) => notify());
});

const subscribe = (notify: () => void) => {
  subscribers.add(notify);
  return () => {
    subscribers.delete(notify);
  };
};

export const ForkLanguageTag: React.FC = () => {
  const { t } = useTranslation();
  const language = useSyncExternalStore(subscribe, () => current);
  return (
    <span className={`fork-lang-tag fork-lang-${language}`}>
      {t(`overlay.dictationLanguage.${language}`)}
    </span>
  );
};
